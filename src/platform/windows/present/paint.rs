//! Peinture d'une fenêtre : WM_PAINT, image préparée, dessin puis soumission par le
//! chemin de présentation, avec recréation du renderer après une perte du périphérique.
use super::{
    super::{
        accessibility,
        events::text,
        ffi::Hwnd,
        graphics::{Renderer, SurfaceChange, is_device_lost},
        window::{record_error, startup, state, timer},
    },
    DRAWING, DrawingState, composition,
    composition::Presenter,
    surface::LayeredSurface,
};
use crate::window::RenderMode;
use std::{
    cell::Cell,
    time::{Duration, Instant},
};
use windows_sys::Win32::{
    Graphics::Gdi::{BeginPaint, EndPaint, PAINTSTRUCT, UpdateWindow},
    UI::WindowsAndMessaging::{DestroyWindow, IsZoomed},
};

thread_local! {
    /// Fin de la dernière peinture : cadence des images pendant une entrée continue.
    static LAST_PAINT: Cell<Option<Instant>> = const { Cell::new(None) };
}
/// Intervalle au-delà duquel une entrée continue (glisser, survol) peint sans attendre.
const FRAME_INTERVAL: Duration = Duration::from_millis(8);
/// Peint tout de suite si une image est en attente et que la précédente date de plus
/// d'un intervalle d'image. Windows ne produit `WM_PAINT` que lorsque la file est vide :
/// une souris à 125–1000 Hz la remplit sans cesse et retarderait l'affichage.
pub(crate) fn flush_if_due(window: Hwnd) {
    if LAST_PAINT
        .with(Cell::get)
        .is_none_or(|at| at.elapsed() >= FRAME_INTERVAL)
    {
        // SAFETY : fenêtre du thread courant ; n'envoie WM_PAINT que si une zone est à
        // redessiner.
        unsafe { UpdateWindow(window) };
    }
}
pub(crate) fn paint_window(window: Hwnd) {
    let mut paint = PAINTSTRUCT::default();
    // SAFETY : `paint` est une structure locale, rendue à EndPaint ci-dessous.
    unsafe { BeginPaint(window, &mut paint) };
    let state = DRAWING.with(|slot| slot.borrow_mut().take());
    let result = if let Some(mut state) = state {
        let result = state.paint(window);
        DRAWING.with(|slot| *slot.borrow_mut() = Some(state));
        result
    } else {
        Ok(())
    };
    // SAFETY : même structure que celle remplie par BeginPaint.
    unsafe { EndPaint(window, &paint) };
    LAST_PAINT.with(|at| at.set(Some(Instant::now())));
    text::sync(window);
    accessibility::sync(window);
    timer::presented(window);
    if let Err(error) = result {
        eprintln!("karbon-ui: rendering failed, closing the window: {error}");
        record_error(error);
        // SAFETY : destruction sur le thread propriétaire ; Windows valide le handle.
        unsafe { DestroyWindow(window) };
    }
}
impl DrawingState {
    fn paint(&mut self, window: Hwnd) -> crate::Result<()> {
        let result = self.paint_frame(window);
        if result.is_err()
            && let Some(profile) = &mut self.profile
        {
            profile.failures += 1;
        }
        result
    }
    fn paint_frame(&mut self, window: Hwnd) -> crate::Result<()> {
        if let Some(at) = self.prepared.take() {
            // Les animations commencent à la première présentation, pas au chargement.
            self.runtime.defer_animations(at.elapsed());
        }
        let started = self.profile.as_ref().map(|_| Instant::now());
        let engine = self.runtime.stats();
        let (width, height, scale) = self.prepare(window)?;
        if width == 0 || height == 0 {
            return Ok(());
        }
        let prepare_time = elapsed(started);
        startup::mark("view built, styled and laid out");
        // SAFETY : simple lecture de l'état de la fenêtre.
        let maximized = unsafe { IsZoomed(window) } != 0;
        let frame = Frame {
            width,
            height,
            scale,
            premultiplied: composition::premultiplied(state::options().transparent, maximized),
            profiled: started.is_some(),
        };
        let presented = (
            width,
            height,
            scale.to_bits(),
            frame.premultiplied,
            self.runtime.stats().paints,
        );
        // Le profilage mesure chaque peinture demandée : il garde le dessin systématique.
        if self.profile.is_none() && self.presented == Some(presented) {
            return Ok(());
        }
        self.presented = None;
        let [surface_time, render_time, submit_time, wait_time] =
            match self.draw_and_submit(window, &frame) {
                Ok(timings) => timings,
                Err(Failure::Submit(error)) => return Err(error),
                Err(Failure::Renderer(error)) => {
                    eprintln!("karbon-ui: recreating the renderer after an error: {error}");
                    // Périphérique perdu : même mode (Auto retente le GPU). Autre échec : WARP.
                    let mode = if self.mode == RenderMode::Auto && !is_device_lost(&error) {
                        RenderMode::Software
                    } else {
                        self.mode
                    };
                    let mut renderer = Renderer::new(mode)?;
                    renderer.load_fonts(self.runtime.font_assets())?;
                    if let Some(profile) = &mut self.profile {
                        profile.emit(self.renderer.label(), "renderer-recovery");
                        *profile = profile.restart();
                        profile.recoveries = 1;
                    }
                    self.renderer = renderer;
                    self.runtime.invalidate();
                    self.prepare(window)?;
                    self.draw_and_submit(window, &frame)
                        .map_err(Failure::into_error)?
                }
            };
        self.presented = Some((
            width,
            height,
            scale.to_bits(),
            frame.premultiplied,
            self.runtime.stats().paints,
        ));
        if let Some(profile) = &mut self.profile {
            let timings = [
                prepare_time,
                render_time,
                surface_time,
                submit_time,
                elapsed(started),
            ];
            profile.record(timings, (width, height), scale);
            let gap = profile.presented(Instant::now());
            let after = self.runtime.stats();
            profile.slow_frame(
                timings,
                wait_time,
                [
                    after.view_time.saturating_sub(engine.view_time),
                    after.style_time.saturating_sub(engine.style_time),
                    after.layout_time.saturating_sub(engine.layout_time),
                    after.paint_time.saturating_sub(engine.paint_time),
                ],
                gap,
            );
        }
        Ok(())
    }
    /// Prépare la surface, dessine puis soumet l'image. Durées : surface, rendu, soumission.
    /// Durées : surface, rendu, soumission, et attente de la place dans la file de
    /// présentation (composition seulement, hors des trois autres).
    fn draw_and_submit(&mut self, window: Hwnd, frame: &Frame) -> Result<[Duration; 4], Failure> {
        let clock = || frame.profiled.then(Instant::now);
        let (width, height, scale) = (frame.width, frame.height, frame.scale);
        let scene = self.runtime.scene();
        let started = clock();
        match &mut self.presenter {
            Presenter::Layered(surface) => {
                if surface
                    .as_ref()
                    .is_none_or(|s| !s.matches(width as i32, height as i32))
                {
                    *surface = Some(
                        LayeredSurface::new(width as i32, height as i32)
                            .map_err(Failure::Submit)?,
                    );
                    if let Some(profile) = &mut self.profile {
                        profile.surface_creations += 1;
                    }
                }
                let surface = surface.as_mut().unwrap();
                let surface_time = elapsed(started);
                let started = clock();
                self.renderer
                    .render_into(scene, width, height, scale, surface.pixels_mut())
                    .map_err(Failure::Renderer)?;
                let render_time = elapsed(started);
                let started = clock();
                surface.present(window).map_err(Failure::Submit)?;
                startup::presented();
                self.runtime.frame_presented(Instant::now());
                Ok([surface_time, render_time, elapsed(started), Duration::ZERO])
            }
            Presenter::Composition(visual) => {
                let change = self
                    .renderer
                    .prepare_swap_chain(width, height, scale, frame.premultiplied)
                    .map_err(Failure::Renderer)?;
                if change != SurfaceChange::Unchanged
                    && let Some(profile) = &mut self.profile
                {
                    profile.surface_creations += 1;
                }
                let surface_time = elapsed(started);
                // Place dans la file de présentation (déjà prise par une image d'animation,
                // voir `runtime::wait_for_frame`) ; attente mesurée à part.
                self.renderer.wait_for_frame();
                let wait_time = self.renderer.frame_wait();
                let started = clock();
                self.renderer
                    .render_to_swap_chain(scene, width, height, scale, frame.premultiplied)
                    .map_err(Failure::Renderer)?;
                let render_time = elapsed(started);
                startup::mark("first frame rendered");
                let started = clock();
                self.renderer.present().map_err(|error| {
                    if is_device_lost(&error) {
                        Failure::Renderer(error)
                    } else {
                        Failure::Submit(error)
                    }
                })?;
                // Nouvelle chaîne (création, mode alpha, renderer recréé) : la rattacher.
                let chain = self
                    .renderer
                    .swap_chain()
                    .expect("swap chain just presented");
                visual.bind(chain).map_err(Failure::Submit)?;
                startup::presented();
                self.runtime.frame_presented(Instant::now());
                Ok([surface_time, render_time, elapsed(started), wait_time])
            }
        }
    }
}
struct Frame {
    width: u32,
    height: u32,
    scale: f32,
    premultiplied: bool,
    profiled: bool,
}
/// Seuls les échecs du renderer (dont la perte du périphérique) justifient sa recréation.
enum Failure {
    Renderer(crate::Error),
    Submit(crate::Error),
}
impl Failure {
    fn into_error(self) -> crate::Error {
        match self {
            Failure::Renderer(error) | Failure::Submit(error) => error,
        }
    }
}
fn elapsed(started: Option<Instant>) -> Duration {
    started.map_or(Duration::ZERO, |started| started.elapsed())
}
