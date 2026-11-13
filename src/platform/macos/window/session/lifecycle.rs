pub(super) use super::super::{
    super::{bitmap::Bitmap, graphics::Renderer, imaging::Imaging, metal},
    events,
    view::{Delegate, View},
};
pub(super) use crate::{
    application::ViewContext,
    task::{TaskCommand, TaskError, TaskExecutor, TaskResult, TaskScope},
    ui::{
        runtime::{ChromeState, Cursor, EventResult, UiEvent, UiRuntime},
        text_input::{ClipboardRequest, EditCommand},
        theme::Theme,
    },
    window::{WindowDescriptor, WindowId, WindowOptions},
};
pub(super) use objc2::{MainThreadMarker, MainThreadOnly, rc::Retained, runtime::ProtocolObject};
pub(super) use objc2_app_kit::{
    NSBackingStoreType, NSWindow, NSWindowButton, NSWindowStyleMask, NSWindowTitleVisibility,
};
pub(super) use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
pub(super) use objc2_quartz_core::kCAGravityTopLeft;
pub(super) use std::time::Instant;

pub(super) use super::frame_link::FrameLink;
pub(super) use super::paint::{Path, gpu};

pub(crate) struct Session {
    pub id: WindowId,
    pub window: Retained<NSWindow>,
    pub(super) view: Retained<View>,
    pub(super) _delegate: Retained<Delegate>,
    // Le runtime (donc l'application) est détruit avant le renderer.
    pub(crate) runtime: UiRuntime,
    pub(super) renderer: Renderer,
    /// Chemin GPU : renderer Metal et calque de la fenêtre (absents en mode logiciel ou
    /// sans GPU Metal).
    pub(super) gpu: Option<(metal::Renderer, metal::Surface)>,
    /// Chemin de la dernière image présentée.
    pub(super) path: Path,
    /// Diagnostic `KARBON_METAL_COMPARE` (voir `metal/compare.rs`).
    pub(super) compare: metal::Comparison,
    pub(crate) options: WindowOptions,
    pub(super) scope: TaskScope,
    pub(super) surface: Option<Bitmap>,
    /// Surface en demi-résolution des images de mouvement (voir `paint`).
    pub(super) motion: Option<Bitmap>,
    /// Fin du défilement en cours : il compte comme un mouvement.
    pub(super) scrolling_until: Option<Instant>,
    /// Dernière image présentée : taille en pixels, échelle et numéro de peinture.
    pub(super) presented: Option<(usize, usize, u32, u64)>,
    /// Une nouvelle image doit être préparée et présentée.
    pub dirty: bool,
    pub(super) cursor: Option<Cursor>,
    /// Texte marqué de la composition en cours (touche morte, IME).
    pub(crate) marked: Option<String>,
    /// Pont NSAccessibility (`accessible.rs`).
    pub(crate) access: super::super::accessible::Access,
    /// Redessin exact attendu après des images approchées (animation).
    pub(super) refine_at: Option<Instant>,
    /// Dernière mémoire Metal écrite par `--profile`.
    pub(super) memory_line: String,
    /// Allègement des ressources GPU attendu (une seconde après la dernière image).
    pub(super) trim_at: Option<Instant>,
    /// Dernier état remis à `Application::window_changed` (`change.rs`).
    pub(crate) reported: Option<crate::window::WindowState>,
    /// Horloge de l'écran (`frame_link.rs`) ; absente avant macOS 14.
    pub(super) link: Option<FrameLink>,
    /// Un battement de l'écran vient d'arriver : la prochaine image part de lui.
    pub(super) frame_clock: bool,
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
        let link = FrameLink::new(&view);
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
            scope: TaskScope::new(super::super::wake),
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
            link,
            frame_clock: false,
        })
    }
    /// Contexte de vue, taille en pixels et échelle de la zone de contenu.
    pub(super) fn context(&self) -> (ViewContext, usize, usize, f32) {
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
    pub(super) fn system_dark(&self) -> bool {
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
    pub(super) fn update_cursor(&mut self) {
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
    pub(crate) fn view_object(&self) -> Retained<View> {
        self.view.clone()
    }
    pub(crate) fn runtime_accessibility_version(
        &self,
    ) -> (u64, u64, u64, u64, Option<crate::ui::element::ElementId>) {
        self.runtime.accessibility_version()
    }
    pub(crate) fn runtime_accessibility(&self) -> Vec<crate::ui::runtime::AccessibleElement> {
        self.runtime.accessibility()
    }
    /// Action d'un lecteur d'écran, appliquée sur une image à jour.
    pub(crate) fn accessibility_action(
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
    /// Affichée mais entièrement couverte (autres fenêtres, autre bureau) : rien à
    /// peindre ni à animer ; l'image est refaite quand elle redevient visible.
    pub(super) fn covered(&self) -> bool {
        self.window.isVisible()
            && !self
                .window
                .occlusionState()
                .contains(objc2_app_kit::NSWindowOcclusionState::Visible)
    }
    pub fn next_wake(&self) -> Option<Instant> {
        if self.window.isMiniaturized() || !self.window.isVisible() || self.covered() {
            return None;
        }
        // Image présentée, battement attendu : il réveillera la boucle (ou son délai).
        let runtime = match self.link.as_ref().and_then(FrameLink::timeout) {
            Some(timeout) => Some(timeout),
            None => self.runtime.next_wake(),
        };
        [runtime, self.refine_at, self.trim_at]
            .into_iter()
            .flatten()
            .min()
    }
    /// Fenêtre réduite dans le Dock ou entièrement cachée : ressources GPU, surfaces et
    /// caches Core Graphics libérés (l'image affichée reste celle du calque). La prochaine
    /// image les recrée.
    pub fn hidden(&mut self) {
        if let Some((gpu, surface)) = &mut self.gpu {
            gpu.trim(metal::Trim::Hidden);
            surface.release();
        }
        // L'image suivante est refaite en entier, sur des drawables à la bonne taille.
        self.presented = None;
        self.trim_at = None;
        self.renderer.release();
        self.surface = None;
        self.motion = None;
    }
    /// Pression mémoire du système (`window/memory.rs`). Avertissement : ressources GPU
    /// allégées comme au repos. Critique : tout ce qui se recrée, drawables compris pour
    /// une fenêtre que l'on ne voit pas ; une fenêtre visible garde son image affichée.
    pub fn relieve_memory(&mut self, critical: bool) {
        if self.options.profile {
            eprintln!(
                "karbon-ui: pression mémoire du système : {}",
                if critical {
                    "critique"
                } else {
                    "avertissement"
                }
            );
        }
        let visible = self.window.isVisible() && !self.window.isMiniaturized() && !self.covered();
        if critical && !visible {
            self.hidden();
            return;
        }
        if let Some((gpu, _)) = &mut self.gpu {
            gpu.trim(if critical {
                metal::Trim::Hidden
            } else {
                metal::Trim::Idle
            });
        }
        if critical {
            // Surfaces et caches de Core Graphics : la prochaine image est dessinée en
            // entier (l'image affichée reste celle du calque).
            self.renderer.release();
            self.surface = None;
            self.motion = None;
        }
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
pub(super) fn place(window: &NSWindow, options: &WindowOptions, mtm: MainThreadMarker) {
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
