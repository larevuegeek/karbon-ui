//! Session d'une fenêtre : `NSWindow` et sa vue, runtime UI de l'application, renderer,
//! tâches de fond ; préparation, entrées et présentation de l'image.
use super::{
    super::{bitmap::Bitmap, graphics::Renderer, imaging::Imaging, metal},
    events,
    view::{Delegate, View},
};
use crate::{
    application::ViewContext,
    task::{TaskCommand, TaskError, TaskExecutor, TaskResult, TaskScope},
    ui::{
        runtime::{ChromeState, Cursor, EventResult, UiEvent, UiRuntime},
        text_input::{ClipboardRequest, EditCommand},
        theme::Theme,
    },
    window::{WindowDescriptor, WindowId, WindowOptions},
};
use objc2::{MainThreadMarker, MainThreadOnly, rc::Retained, runtime::ProtocolObject};
use objc2_app_kit::{
    NSBackingStoreType, NSWindow, NSWindowButton, NSWindowStyleMask, NSWindowTitleVisibility,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
use objc2_quartz_core::kCAGravityTopLeft;
use std::time::Instant;

mod paint;

use paint::{Path, gpu};

pub(super) struct Session {
    pub id: WindowId,
    pub window: Retained<NSWindow>,
    view: Retained<View>,
    _delegate: Retained<Delegate>,
    // Le runtime (donc l'application) est détruit avant le renderer.
    pub(super) runtime: UiRuntime,
    renderer: Renderer,
    /// Chemin GPU : renderer Metal et calque de la fenêtre (absents en mode logiciel ou
    /// sans GPU Metal).
    gpu: Option<(metal::Renderer, metal::Surface)>,
    /// Chemin de la dernière image présentée.
    path: Path,
    /// Diagnostic `KARBON_METAL_COMPARE` (voir `metal/compare.rs`).
    compare: metal::Comparison,
    pub(super) options: WindowOptions,
    scope: TaskScope,
    surface: Option<Bitmap>,
    /// Surface en demi-résolution des images de mouvement (voir `paint`).
    motion: Option<Bitmap>,
    /// Fin du défilement en cours : il compte comme un mouvement.
    scrolling_until: Option<Instant>,
    /// Dernière image présentée : taille en pixels, échelle et numéro de peinture.
    presented: Option<(usize, usize, u32, u64)>,
    /// Une nouvelle image doit être préparée et présentée.
    pub dirty: bool,
    cursor: Option<Cursor>,
    /// Texte marqué de la composition en cours (touche morte, IME).
    pub(super) marked: Option<String>,
    /// Pont NSAccessibility (`accessible.rs`).
    pub(super) access: super::accessible::Access,
    /// Redessin exact attendu après des images approchées (animation).
    refine_at: Option<Instant>,
    /// Dernière mémoire Metal écrite par `--profile`.
    memory_line: String,
    /// Allègement des ressources GPU attendu (une seconde après la dernière image).
    trim_at: Option<Instant>,
    /// Dernier état remis à `Application::window_changed` (`change.rs`).
    pub(super) reported: Option<crate::window::WindowState>,
}
impl Session {
    /// Charge l'application, puis ouvre sa fenêtre (au centre de l'écran).
    pub fn open(descriptor: WindowDescriptor, mtm: MainThreadMarker) -> crate::Result<Self> {
        let WindowDescriptor {
            id,
            options,
            application,
        } = descriptor;
        if let Some(reason) = crate::window::validation::invalid_options(&options) {
            return Err(crate::Error::InvalidWindowOptions {
                reason: reason.into(),
            });
        }
        let mut renderer = Renderer::new()?;
        renderer.load_fonts(application.fonts())?;
        let mut runtime = UiRuntime::from_dyn(application, &Imaging)?;
        runtime.set_reduced_motion(options.reduced_motion);
        let mut style = NSWindowStyleMask::Titled
            | NSWindowStyleMask::Closable
            | NSWindowStyleMask::Miniaturizable
            | NSWindowStyleMask::Resizable;
        if !options.title_bar {
            // L'application dessine sa barre et ses boutons : contenu sous la barre de titre.
            style |= NSWindowStyleMask::FullSizeContentView;
        }
        let rect = NSRect::new(
            NSPoint::new(0., 0.),
            NSSize::new(f64::from(options.width), f64::from(options.height)),
        );
        // SAFETY : fenêtre créée sur le thread principal ; libérée par Rust, pas à la fermeture.
        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                NSWindow::alloc(mtm),
                rect,
                style,
                NSBackingStoreType::Buffered,
                false,
            )
        };
        // SAFETY : la fenêtre reste possédée par la session.
        unsafe { window.setReleasedWhenClosed(false) };
        window.setTitle(&NSString::from_str(options.title));
        if !options.title_bar {
            window.setTitlebarAppearsTransparent(true);
            window.setTitleVisibility(NSWindowTitleVisibility::Hidden);
            for button in [
                NSWindowButton::CloseButton,
                NSWindowButton::MiniaturizeButton,
                NSWindowButton::ZoomButton,
            ] {
                if let Some(button) = window.standardWindowButton(button) {
                    button.setHidden(true);
                }
            }
        }
        window.setContentMinSize(NSSize::new(
            f64::from(options.min_width),
            f64::from(options.min_height),
        ));
        window.setAcceptsMouseMovedEvents(true);
        // Fenêtre en sRGB comme les images du renderer : Core Animation n'a plus à les
        // convertir sur le CPU à chaque image (le serveur de fenêtres gère l'écran).
        window.setColorSpace(Some(&objc2_app_kit::NSColorSpace::sRGBColorSpace()));
        let view = View::new(mtm, id, options.transparent);
        window.setContentView(Some(&view));
        window.makeFirstResponder(Some(&view));
        let delegate = Delegate::new(mtm, id);
        window.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
        place(&window, &options, mtm);
        if options.topmost {
            window.setLevel(objc2_app_kit::NSFloatingWindowLevel);
        }
        if options.transparent {
            // Fond transparent : seuls les pixels peints par l'application sont visibles.
            window.setOpaque(false);
            window.setBackgroundColor(Some(&objc2_app_kit::NSColor::clearColor()));
            window.setHasShadow(false);
            window.setIgnoresMouseEvents(options.click_through);
        }
        if let Some(layer) = view.layer() {
            // SAFETY : constante de Core Animation.
            layer.setContentsGravity(unsafe { kCAGravityTopLeft });
        }
        let gpu = gpu(&options, &view);
        Ok(Self {
            id,
            window,
            view,
            _delegate: delegate,
            runtime,
            renderer,
            gpu,
            path: Path::None,
            compare: Default::default(),
            options,
            scope: TaskScope::new(super::wake),
            surface: None,
            motion: None,
            scrolling_until: None,
            presented: None,
            dirty: true,
            cursor: None,
            refine_at: None,
            memory_line: String::new(),
            trim_at: None,
            marked: None,
            access: Default::default(),
            reported: None,
        })
    }
    pub fn show(&self) {
        if self.options.activate {
            self.window.makeKeyAndOrderFront(None);
        } else {
            self.window.orderFront(None);
        }
    }
    /// Contexte de vue, taille en pixels et échelle de la zone de contenu.
    fn context(&self) -> (ViewContext, usize, usize, f32) {
        let bounds = self.view.bounds();
        let scale = self.window.backingScaleFactor().max(1.) as f32;
        let (width, height) = (bounds.size.width as f32, bounds.size.height as f32);
        let context = ViewContext {
            width,
            height,
            theme: if self.options.light {
                Theme::LIGHT
            } else {
                Theme::DARK
            },
            transparent: self.options.transparent,
            backend: match (self.path, &self.gpu) {
                (Path::Metal, _) | (Path::None, Some(_)) => metal::Renderer::LABEL,
                _ => Renderer::LABEL,
            },
            scale,
        };
        let pixels = |v: f32| ((v * scale).round() as usize).clamp(1, 16384);
        (context, pixels(width), pixels(height), scale)
    }
    /// Apparence sombre de la fenêtre (réglage du système, sauf apparence imposée).
    fn system_dark(&self) -> bool {
        use objc2_app_kit::{
            NSAppearanceCustomization, NSAppearanceNameAqua, NSAppearanceNameDarkAqua,
        };
        // SAFETY : constantes d'AppKit, valides pendant toute la vie du processus.
        let (light, dark) = unsafe { (NSAppearanceNameAqua, NSAppearanceNameDarkAqua) };
        let names = objc2_foundation::NSArray::from_slice(&[light, dark]);
        self.window
            .effectiveAppearance()
            .bestMatchFromAppearancesWithNames(&names)
            .is_some_and(|best| &*best == dark)
    }
    pub fn prepare(&mut self) -> crate::Result<()> {
        let (context, ..) = self.context();
        // Option de l'application ou réglage « Réduire les animations » du système.
        let reduce = self.options.reduced_motion
            || objc2_app_kit::NSWorkspace::sharedWorkspace()
                .accessibilityDisplayShouldReduceMotion();
        self.runtime.set_reduced_motion(reduce);
        self.runtime.set_system_dark(self.system_dark());
        // macOS arrondit lui-même les coins de la fenêtre : fond carré, sans barre du moteur.
        let chrome = ChromeState {
            visible: false,
            hovered: None,
            pressed: None,
            maximized: true,
        };
        self.runtime
            .prepare_at(context, chrome, &mut self.renderer, Instant::now())
    }
    /// Envoie `event` au runtime et applique sa réponse (presse-papiers, curseur).
    pub fn input(&mut self, event: UiEvent) -> crate::Result<EventResult> {
        // Déplacements et réveils : l'arbre de l'image précédente suffit ; la préparation
        // attend la prochaine image (comme sous Windows).
        let deferred = matches!(event, UiEvent::PointerMove(_) | UiEvent::Timer(_));
        if !deferred {
            self.prepare()?;
        }
        if matches!(event, UiEvent::Scroll { .. } | UiEvent::Wheel { .. }) {
            self.scrolling_until = Some(Instant::now() + std::time::Duration::from_millis(150));
        }
        let paints = self.runtime.stats().paints;
        let mut result = self.runtime.event(event);
        match result.clipboard.take() {
            Some(ClipboardRequest::Write { text, cut }) => {
                events::write_clipboard(&text);
                if cut {
                    self.runtime
                        .event(UiEvent::Edit(EditCommand::DeleteSelection));
                }
                result.repaint = true;
            }
            Some(ClipboardRequest::Read) => {
                if let Some(text) = events::read_clipboard() {
                    self.runtime.event(UiEvent::Text(text.into()));
                }
                result.repaint = true;
            }
            None => {}
        }
        result.repaint |= self.runtime.has_state_styles() || self.runtime.stats().paints != paints;
        self.dirty |= result.repaint;
        self.update_cursor();
        Ok(result)
    }
    fn update_cursor(&mut self) {
        let cursor = self.runtime.cursor();
        if self.cursor != Some(cursor) {
            events::cursor(cursor).set();
            self.cursor = Some(cursor);
        }
    }
    /// Remet le curseur courant (après un passage par la barre de titre ou une autre
    /// fenêtre).
    pub fn refresh_cursor(&mut self) {
        self.cursor = None;
        self.update_cursor();
    }
    pub(super) fn view_object(&self) -> Retained<View> {
        self.view.clone()
    }
    pub(super) fn runtime_accessibility_version(
        &self,
    ) -> (u64, u64, u64, u64, Option<crate::ui::element::ElementId>) {
        self.runtime.accessibility_version()
    }
    pub(super) fn runtime_accessibility(&self) -> Vec<crate::ui::runtime::AccessibleElement> {
        self.runtime.accessibility()
    }
    /// Action d'un lecteur d'écran, appliquée sur une image à jour.
    pub(super) fn accessibility_action(
        &mut self,
        action: crate::ui::runtime::AccessibilityAction,
    ) -> crate::Result<bool> {
        self.prepare()?;
        let applied = self.runtime.accessibility_action(action).is_ok();
        self.prepare()?;
        self.dirty = true;
        Ok(applied)
    }
    pub fn drag_region_at(&self, point: crate::ui::scene::PointF) -> bool {
        self.runtime.drag_region_at(point)
    }
    pub fn next_wake(&self) -> Option<Instant> {
        if self.window.isMiniaturized() || !self.window.isVisible() {
            return None;
        }
        [self.runtime.next_wake(), self.refine_at, self.trim_at]
            .into_iter()
            .flatten()
            .min()
    }
    /// Une image vient d'être présentée : avant l'attente de l'écran, une tranche du
    /// travail étalé et la reconstruction demandée (voir `platform/backend.rs`).
    pub fn frame_presented(&mut self) -> crate::Result<()> {
        let now = Instant::now();
        self.runtime.frame_presented(now);
        if crate::platform::shared::pacing::enabled() {
            self.runtime.begin_frame_work(now);
            if self.runtime.rebuild_pending() {
                self.prepare()?;
                self.dirty = true;
            }
        }
        Ok(())
    }
    /// L'attente de l'écran est finie : ce qui est demandé maintenant attend l'image suivante.
    pub fn frame_wait_over(&mut self, now: Instant) {
        if crate::platform::shared::pacing::enabled() {
            self.runtime.hold_rebuild(now);
        }
    }
    /// Échéances atteintes : réveil du runtime (animations, infobulles) et redessin
    /// exact après une animation.
    pub fn tick(&mut self, now: Instant) -> crate::Result<()> {
        if self.refine_at.is_some_and(|at| at <= now) {
            self.refine_at = None;
            // Mouvement terminé : la surface en demi-résolution n'est plus utile.
            self.motion = None;
            self.presented = None;
            self.dirty = true;
        }
        if self.trim_at.is_some_and(|at| at <= now) {
            self.trim_at = None;
            if let Some((gpu, _)) = &mut self.gpu {
                gpu.trim(metal::Trim::Idle);
            }
        }
        if self.runtime.next_wake().is_some_and(|wake| wake <= now) {
            self.frame_wait_over(now);
            self.input(UiEvent::Timer(now))?;
        }
        Ok(())
    }
    /// Fenêtre réduite dans le Dock ou entièrement cachée : ressources GPU, surfaces et
    /// caches Core Graphics libérés (l'image affichée reste celle du calque). La prochaine
    /// image les recrée.
    pub fn hidden(&mut self) {
        if let Some((gpu, _)) = &mut self.gpu {
            gpu.trim(metal::Trim::Hidden);
        }
        self.trim_at = None;
        self.renderer.release();
        self.surface = None;
        self.motion = None;
    }
    pub fn take_window_commands(&mut self) -> Vec<crate::window::WindowCommand> {
        self.runtime.take_window_commands()
    }
    /// Lance, annule et livre les tâches de fond ; `true` si l'application a changé.
    pub fn tasks(&mut self, executor: &mut Option<TaskExecutor>) -> bool {
        let mut changed = false;
        loop {
            let commands: Vec<TaskCommand> = self.runtime.take_task_commands();
            for command in &commands {
                match command {
                    TaskCommand::Spawn(task) => self.scope.cancel(task.id),
                    TaskCommand::Cancel(id) => self.scope.cancel(*id),
                    TaskCommand::CancelAll => self.scope.cancel_all(),
                }
            }
            for command in commands {
                let TaskCommand::Spawn(task) = command else {
                    continue;
                };
                if executor.is_none() {
                    match TaskExecutor::for_ui() {
                        Ok(pool) => *executor = Some(pool),
                        Err(error) => {
                            let failure = TaskResult::failure(
                                task.id,
                                TaskError::StartFailed(error.to_string()),
                            );
                            changed |= self.runtime.task_completed(failure);
                            continue;
                        }
                    }
                }
                let pool = executor.as_ref().expect("executor created above");
                if let Some(failure) = self.scope.spawn(task, pool) {
                    changed |= self.runtime.task_completed(failure);
                }
            }
            let Some(result) = self.scope.take_next() else {
                break;
            };
            changed |= self.runtime.task_completed(result);
        }
        self.dirty |= changed;
        changed
    }
}
/// Coin haut-gauche de la fenêtre à (`x`, `y`) points de celui de la zone utile de l'écran
/// principal (repère de Windows, y vers le bas), ramené dans l'écran.
fn place(window: &NSWindow, options: &WindowOptions, mtm: MainThreadMarker) {
    let Some(screen) = objc2_app_kit::NSScreen::mainScreen(mtm) else {
        window.center();
        return;
    };
    let area = screen.visibleFrame();
    // Fenêtre plus haute que l'écran : réduite tout de suite (AppKit le ferait à
    // l'affichage), pour que la première image soit dessinée à la bonne taille.
    let frame = window.constrainFrameRect_toScreen(window.frame(), Some(&screen));
    window.setFrame_display(frame, false);
    let frame = window.frame();
    let max_x = (area.size.width - frame.size.width).max(0.);
    let max_y = (area.size.height - frame.size.height).max(0.);
    let x = area.origin.x + f64::from(options.x).clamp(0., max_x);
    let top = area.origin.y + area.size.height - f64::from(options.y).clamp(0., max_y);
    window.setFrameTopLeftPoint(NSPoint::new(x, top));
}
