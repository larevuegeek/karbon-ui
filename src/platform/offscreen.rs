//! Protocole hors écran commun à tous les backends : chargement de l'application,
//! tâches de fond, aperçu et mesures. Seuls le moteur (`Offscreen`) et les compteurs du
//! processus viennent du backend.
use super::backend::{Backend, Offscreen};
use crate::{
    application::{DynApplication, ViewContext},
    diagnostics::*,
    task::{TaskCommand, TaskExecutor, TaskScope},
    ui::{
        runtime::{ChromeState, UiEvent, UiRuntime},
        scene::PointF,
        title_bar::TITLE_BAR_HEIGHT,
    },
    window::WindowCommand,
};
use std::{
    sync::mpsc,
    time::{Duration, Instant},
};

/// Exécuter les tâches demandées par l'application (et celles qu'elles enchaînent)
/// jusqu'à ce qu'aucune ne reste en vol, en livrant chaque résultat sur ce thread.
///
/// `executor` est créé à la première tâche et conservé par l'appelant. Les demandes de
/// fenêtres sont confiées à `windows`, qui peut les refuser.
pub(crate) fn settle_tasks(
    ui: &mut UiRuntime,
    executor: &mut Option<TaskExecutor>,
    timeout: Duration,
    operation: &'static str,
    mut windows: impl FnMut(Vec<WindowCommand>) -> crate::Result<()>,
) -> crate::Result<()> {
    let (wake, receive) = mpsc::channel();
    let mut scope = TaskScope::new(move || {
        let _ = wake.send(());
    });
    let deadline = Instant::now() + timeout;
    let mut commands = ui.take_task_commands();
    loop {
        if Instant::now() >= deadline {
            return Err(crate::Error::TimedOut { operation });
        }
        for command in std::mem::take(&mut commands) {
            match command {
                TaskCommand::Spawn(task) => {
                    if executor.is_none() {
                        *executor = Some(TaskExecutor::for_ui()?);
                    }
                    let pool = executor.as_ref().expect("initialized executor");
                    if let Some(error) = scope.spawn(task, pool) {
                        ui.task_completed(error);
                    }
                }
                TaskCommand::Cancel(id) => scope.cancel(id),
                TaskCommand::CancelAll => scope.cancel_all(),
            }
        }
        while let Some(result) = scope.take_next() {
            ui.task_completed(result);
        }
        let requested = ui.take_window_commands();
        if !requested.is_empty() {
            windows(requested)?;
        }
        commands = ui.take_task_commands();
        if !commands.is_empty() {
            continue;
        }
        if scope.active_count() == 0 {
            return Ok(());
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() || receive.recv_timeout(remaining).is_err() {
            return Err(crate::Error::TimedOut { operation });
        }
    }
}

/// Pas de l'horloge simulée de la phase d'animation (60 Hz).
const ANIMATION_FRAME: Duration = Duration::from_nanos(16_666_667);

/// Moteur, runtime chargé et contexte de vue d'une session hors écran.
struct Session<O> {
    // Ordre de destruction : runtime, ouvriers, puis moteur (comme les variables locales
    // de l'ancien protocole Windows).
    ui: UiRuntime,
    _workers: Option<TaskExecutor>,
    offscreen: O,
    context: ViewContext,
    chrome: ChromeState,
    width: u32,
    height: u32,
}
impl<O: Offscreen> Session<O> {
    fn open<B: Backend<Offscreen = O>>(
        options: BenchmarkOptions,
        application: Box<dyn DynApplication>,
    ) -> crate::Result<Self> {
        crate::diagnostics::validate(options)?;
        let mut offscreen = B::offscreen(options.renderer)?;
        offscreen.load_fonts(application.fonts())?;
        let mut ui = UiRuntime::from_dyn(application, offscreen.resources())?;
        ui.set_reduced_motion(options.reduced_motion);
        let mut workers = None;
        settle_tasks(
            &mut ui,
            &mut workers,
            Duration::from_secs(10),
            "benchmark loading",
            // La fenêtre simulée ne change pas (taille, position, état) et ne se ferme
            // pas : seules les ouvertures sont refusées.
            |requested| {
                if requested
                    .iter()
                    .any(|command| matches!(command, WindowCommand::Open(_)))
                {
                    return Err(crate::Error::Unsupported {
                        reason: "benchmarks and previews cannot open windows",
                    });
                }
                Ok(())
            },
        )?;
        let context = ViewContext {
            width: options.width as f32,
            height: (options.height
                - if options.title_bar {
                    TITLE_BAR_HEIGHT as u32
                } else {
                    0
                }) as f32,
            theme: options.theme,
            transparent: options.transparent,
            backend: offscreen.label(),
            scale: options.scale,
        };
        Ok(Self {
            ui,
            _workers: workers,
            offscreen,
            context,
            chrome: ChromeState {
                visible: options.title_bar,
                ..Default::default()
            },
            width: (options.width as f32 * options.scale).round() as u32,
            height: (options.height as f32 * options.scale).round() as u32,
        })
    }
    fn prepare(&mut self) -> crate::Result<()> {
        self.ui
            .prepare(self.context, self.chrome, &mut self.offscreen)
    }
    /// Image préparée à l'instant `now` d'une horloge simulée (phase d'animation).
    fn prepare_at(&mut self, now: Instant) -> crate::Result<()> {
        self.ui
            .prepare_at(self.context, self.chrome, &mut self.offscreen, now)
    }
    /// Première image, puis le pointeur posé à `pointer` (styles `:hover`) si demandé.
    fn prepare_preview(&mut self, pointer: Option<(f32, f32)>) -> crate::Result<()> {
        self.prepare()?;
        if let Some((x, y)) = pointer {
            self.ui.event(crate::ui::runtime::UiEvent::PointerMove(
                crate::ui::scene::PointF { x, y },
            ));
            self.prepare()?;
        }
        Ok(())
    }
    fn render(&mut self, scale: f32) -> crate::Result<Vec<u8>> {
        self.offscreen
            .render(self.ui.scene(), self.width, self.height, scale)
    }
}

/// Protocole `offscreen-prepare-render-readback-v1` : trois images de chauffe, puis une
/// phase de repeinture statique, une phase de défilement, une phase de reconstruction
/// (la vue redemandée à chaque image, comme sous une horloge de lecture) et une phase de
/// survol (le pointeur passe d'une ligne à l'autre à chaque image), enfin une phase
/// d'animation sur une horloge simulée (le clic `options.click`, puis une image de 60 Hz
/// par image mesurée) et une phase de molette (crans de défilement doux sous un pointeur
/// posé sur le contenu, à l'heure réelle).
pub(crate) fn render_benchmark<B: Backend>(
    options: BenchmarkOptions,
    application: Box<dyn DynApplication>,
) -> crate::Result<BenchmarkReport> {
    let start = Instant::now();
    let mut session = Session::open::<B>(options, application)?;
    for _ in 0..3 {
        session.prepare()?;
        drop(session.render(options.scale)?);
    }
    let mut images = std::collections::HashSet::new();
    let mut image_commands = 0;
    let mut scene_image_bytes = 0;
    for command in &session.ui.scene().commands {
        if let crate::ui::scene::DrawCommand::Image { image, .. } = command {
            image_commands += 1;
            if images.insert(std::sync::Arc::as_ptr(image) as usize) {
                scene_image_bytes += image.pixels.capacity();
            }
        }
    }
    let initialization = start.elapsed();
    let mut phases = Vec::new();
    for (name, scroll, rebuild, hover, animation, wheel) in [
        ("static-repaint", false, false, false, false, false),
        ("scroll", true, false, false, false, false),
        ("rebuild", false, true, false, false, false),
        ("hover", false, false, true, false, false),
        ("animation", false, false, false, true, false),
        ("wheel", false, false, false, false, true),
    ] {
        let counters = session.ui.stats();
        let before = B::process_sample()?;
        let start = Instant::now();
        let mut timings = Vec::with_capacity(options.frames);
        let mut prepares = Vec::with_capacity(options.frames);
        // Horloge simulée de la phase d'animation : une image de 60 Hz par image mesurée,
        // le réveil puis la préparation, comme une fenêtre cadencée par l'écran. Le temps
        // de calcul ne décale pas l'échantillonnage : chaque image avance d'un pas constant.
        let mut clock = Instant::now();
        let mut frame_requests = 0;
        for index in 0..options.frames {
            let frame = Instant::now();
            if animation {
                if index == 0
                    && let Some((x, y)) = options.click
                {
                    let point = PointF { x, y };
                    session.ui.event(UiEvent::PointerMove(point));
                    session.ui.event(UiEvent::PointerDown(point));
                    session.ui.event(UiEvent::PointerUp(point));
                }
                clock += ANIMATION_FRAME;
                session.ui.event(UiEvent::Timer(clock));
                session.prepare_at(clock)?;
                if session
                    .ui
                    .next_wake()
                    .is_some_and(|wake| wake <= clock + ANIMATION_FRAME)
                {
                    frame_requests += 1;
                }
            }
            if wheel {
                // Molette sous un pointeur posé sur le contenu : un cran toutes les quatre
                // images (15 crans par seconde), vers le bas la première moitié de la phase
                // puis vers le haut. L'horloge suit l'heure réelle, comme une fenêtre
                // réveillée par l'écran : une image lente décale les suivantes.
                let point = PointF {
                    x: options.width as f32 * 0.7,
                    y: options.height as f32 * 0.4,
                };
                if index == 0 {
                    session.ui.event(UiEvent::PointerMove(point));
                }
                if index % 4 == 0 {
                    session.ui.event(UiEvent::Wheel {
                        position: point,
                        delta_x: 0.,
                        delta_y: if index < options.frames / 2 {
                            100.
                        } else {
                            -100.
                        },
                    });
                }
                clock = (clock + ANIMATION_FRAME).max(Instant::now());
                session.ui.event(UiEvent::Timer(clock));
                session.prepare_at(clock)?;
            }
            if rebuild {
                session.ui.invalidate();
            }
            if hover {
                // Le pointeur passe d'une ligne à l'autre (60 unités plus bas).
                session.ui.event(UiEvent::PointerMove(PointF {
                    x: options.width as f32 * 0.7,
                    y: options.height as f32 * 0.4 + if index % 2 == 0 { 0. } else { 60. },
                }));
            }
            if scroll {
                session.ui.event(UiEvent::Scroll {
                    position: PointF {
                        x: options.width as f32 * 0.7,
                        y: options.height as f32 * 0.4,
                    },
                    delta_x: 0.,
                    delta_y: if (index / 8) % 2 == 0 { 24. } else { -24. },
                });
            }
            if !animation && !wheel {
                session.prepare()?;
            }
            prepares.push(frame.elapsed().as_secs_f64() * 1000.);
            drop(session.render(options.scale)?);
            timings.push(frame.elapsed().as_secs_f64() * 1000.);
        }
        let wall = start.elapsed();
        let after = B::process_sample()?;
        timings.sort_by(f64::total_cmp);
        prepares.sort_by(f64::total_cmp);
        let rank =
            |fraction: f64| ((options.frames as f64 * fraction).ceil() as usize).saturating_sub(1);
        let percentile = |fraction: f64| timings[rank(fraction)];
        let mean = |d: std::time::Duration| d.as_secs_f64() * 1000. / options.frames as f64;
        let cpu_ms = after.cpu_100ns.saturating_sub(before.cpu_100ns) as f64 / 10_000.;
        let next = session.ui.stats();
        phases.push(BenchmarkPhase {
            name,
            frames: options.frames,
            p50_ms: percentile(0.5),
            p95_ms: percentile(0.95),
            max_ms: *timings.last().unwrap(),
            prepare_p50_ms: prepares[rank(0.5)],
            prepare_p95_ms: prepares[rank(0.95)],
            prepare_max_ms: *prepares.last().unwrap(),
            wall,
            cpu_ms,
            cpu_one_core_percent: cpu_ms / wall.as_secs_f64().max(f64::EPSILON) / 10.,
            before,
            after,
            view_rebuilds: next.views - counters.views,
            layouts: next.layouts - counters.layouts,
            scene_paints: next.paints - counters.paints,
            view_ms: mean(next.view_time - counters.view_time),
            style_ms: mean(next.style_time - counters.style_time),
            layout_ms: mean(next.layout_time - counters.layout_time),
            paint_ms: mean(next.paint_time - counters.paint_time),
            frame_requests,
        });
    }
    Ok(BenchmarkReport {
        backend: session.offscreen.label(),
        options,
        initialization,
        phases,
        image_commands,
        unique_images: images.len(),
        scene_image_bytes,
    })
}

/// Une image de l'application, rendue sans fenêtre.
pub(crate) fn render_preview<B: Backend>(
    options: BenchmarkOptions,
    application: Box<dyn DynApplication>,
) -> crate::Result<crate::ui::scene::Image> {
    let mut session = Session::open::<B>(options, application)?;
    session.prepare_preview(options.pointer)?;
    let pixels = session.render(options.scale)?;
    crate::ui::scene::Image::new(session.width, session.height, pixels)
}

/// L'arbre mis en page d'une image, sans rastérisation (diagnostic).
pub(crate) fn inspect_preview<B: Backend, R>(
    options: BenchmarkOptions,
    application: Box<dyn DynApplication>,
    inspect: impl FnOnce(&crate::ui::element::Element) -> R,
) -> crate::Result<R> {
    let mut session = Session::open::<B>(options, application)?;
    session.prepare_preview(options.pointer)?;
    let root = session
        .ui
        .root()
        .ok_or_else(|| crate::Error::invalid_input("the application produced no view"))?;
    Ok(inspect(root))
}
