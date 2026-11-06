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
            |_| {
                Err(crate::Error::Unsupported {
                    reason: "benchmarks and previews cannot open windows",
                })
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
/// phase de repeinture statique et une phase de défilement.
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
    for (name, scroll) in [("static-repaint", false), ("scroll", true)] {
        let counters = session.ui.stats();
        let before = B::process_sample()?;
        let start = Instant::now();
        let mut timings = Vec::with_capacity(options.frames);
        for index in 0..options.frames {
            let frame = Instant::now();
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
            session.prepare()?;
            drop(session.render(options.scale)?);
            timings.push(frame.elapsed().as_secs_f64() * 1000.);
        }
        let wall = start.elapsed();
        let after = B::process_sample()?;
        timings.sort_by(f64::total_cmp);
        let percentile = |fraction: f64| {
            timings[((timings.len() as f64 * fraction).ceil() as usize).saturating_sub(1)]
        };
        let cpu_ms = after.cpu_100ns.saturating_sub(before.cpu_100ns) as f64 / 10_000.;
        let next = session.ui.stats();
        phases.push(BenchmarkPhase {
            name,
            frames: options.frames,
            p50_ms: percentile(0.5),
            p95_ms: percentile(0.95),
            max_ms: *timings.last().unwrap(),
            wall,
            cpu_ms,
            cpu_one_core_percent: cpu_ms / wall.as_secs_f64().max(f64::EPSILON) / 10.,
            before,
            after,
            view_rebuilds: next.views - counters.views,
            layouts: next.layouts - counters.layouts,
            scene_paints: next.paints - counters.paints,
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
