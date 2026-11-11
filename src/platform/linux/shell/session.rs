//! Fenêtre vue du moteur, commune aux boucles Wayland et X11 : application (runtime),
//! renderers (Cairo, et Vulkan quand il présente), taille logique et échelle, barre de
//! titre du moteur (survol, appui, états), tâches de fond, réveils (animations, image
//! exacte après une animation, allègement du GPU), focus et erreur de l'application, et la
//! fenêtre vue par le pont AT-SPI (`Host`, délégué par la fenêtre de chaque backend).
use crate::{
    application::ViewContext,
    platform::linux::{
        graphics::Renderer,
        portal::{Dialogs, Waker},
        vulkan,
    },
    task::{TaskCommand, TaskError, TaskExecutor, TaskId, TaskResult, TaskScope},
    ui::{
        runtime::{ChromeState, EventResult, UiEvent, UiRuntime},
        theme::Theme,
        title_bar::CaptionButton,
    },
    window::{FileDialog, WindowId, WindowOptions},
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

pub(in crate::platform::linux) struct Session {
    pub id: WindowId,
    // Le runtime (donc l'application) est détruit avant les renderers.
    pub runtime: UiRuntime,
    pub renderer: Renderer,
    /// Chemin GPU : renderer Vulkan et surface de la fenêtre (absents en mode logiciel,
    /// sans Vulkan présentable, ou après une erreur du GPU).
    pub gpu: Option<(vulkan::Renderer, vulkan::Surface)>,
    pub options: WindowOptions,
    scope: TaskScope,
    /// Dialogues de fichiers ouverts (portail XDG) ; leurs choix sont remis avec les
    /// résultats des tâches.
    dialogs: Dialogs,
    /// Réveil de la boucle depuis un autre thread (tâches, dialogues).
    wake: Waker,
    /// Taille logique de la surface.
    pub width: i32,
    pub height: i32,
    /// Échelle de la surface (pixels par unité logique), tenue à jour par la boucle.
    pub scale: f32,
    /// Taille en pixels imposée par le système (fenêtre X11), sinon déduite de la taille
    /// logique.
    pub physical: Option<(usize, usize)>,
    /// Dernière image envoyée : taille en pixels, échelle et numéro de peinture.
    pub presented: Option<(usize, usize, u32, u64)>,
    /// Une nouvelle image doit être préparée et envoyée.
    pub dirty: bool,
    pub maximized: bool,
    /// Bords collés à l'écran ou plein écran : pas de redimensionnement par les bords.
    pub tiled: bool,
    /// Le système dessine les décorations (barre de titre du moteur masquée).
    pub server_decorated: bool,
    /// Les bords se tirent hors du contenu (bordure Wayland) : pas de bande intérieure.
    pub edges_outside: bool,
    pub hovered: Option<CaptionButton>,
    pub pressed: Option<CaptionButton>,
    /// Dernier appui (instant et position) pour reconnaître un double clic.
    pub last_press: Option<(Instant, crate::ui::scene::PointF)>,
    /// Redessin exact attendu après des images approchées (animation).
    pub refine_at: Option<Instant>,
    /// Défilement en cours jusqu'à cet instant (images réduites possibles, `wayland`).
    pub scrolling_until: Option<Instant>,
    /// Dernier thème du bureau transmis : un changement demande une image.
    system_dark: bool,
    /// Allègement des ressources GPU attendu (une seconde après la dernière image).
    pub trim_at: Option<Instant>,
    /// La fenêtre a le clavier.
    pub focused: bool,
    /// Erreur ou panique de l'application pendant une action du lecteur d'écran : la
    /// boucle ferme la fenêtre.
    pub failure: Option<crate::Error>,
    /// Position, plein écran, bornes de taille (`facts.rs`).
    pub facts: super::facts::Facts,
}
impl Session {
    pub fn new(
        id: WindowId,
        options: WindowOptions,
        (runtime, renderer): (UiRuntime, Renderer),
        wake: impl Fn() + Send + Sync + 'static,
    ) -> Self {
        let wake: Waker = Arc::new(wake);
        crate::platform::linux::desktop::follow_color_scheme(Arc::clone(&wake));
        let scope = {
            let wake = Arc::clone(&wake);
            TaskScope::new(move || wake())
        };
        Self {
            id,
            runtime,
            renderer,
            gpu: None,
            width: options.width.max(1),
            height: options.height.max(1),
            options,
            scope,
            dialogs: Dialogs::new(),
            wake,
            scale: 1.,
            physical: None,
            presented: None,
            dirty: true,
            maximized: false,
            tiled: false,
            server_decorated: false,
            edges_outside: false,
            hovered: None,
            pressed: None,
            last_press: None,
            refine_at: None,
            scrolling_until: None,
            system_dark: false,
            trim_at: None,
            focused: false,
            failure: None,
            facts: Default::default(),
        }
    }
    /// La barre de titre du moteur est dessinée (pas de décorations du système).
    pub fn chrome_visible(&self) -> bool {
        self.options.title_bar && !self.server_decorated
    }
    /// Taille logique de la zone de contenu.
    pub fn logical_size(&self) -> (i32, i32) {
        (self.width, self.height)
    }
    /// Contexte de vue, taille en pixels et échelle.
    pub fn context(&self) -> (ViewContext, usize, usize, f32) {
        let scale = self.scale;
        let context = ViewContext {
            width: self.width as f32,
            height: self.height as f32,
            theme: if self.options.light {
                Theme::LIGHT
            } else {
                Theme::DARK
            },
            transparent: self.options.transparent,
            backend: if self.gpu.is_some() {
                vulkan::Renderer::LABEL
            } else {
                Renderer::LABEL
            },
            scale,
        };
        let pixels = |v: i32| ((v as f32 * scale).round() as usize).clamp(1, 16384);
        let (width, height) = self
            .physical
            .unwrap_or((pixels(self.width), pixels(self.height)));
        (context, width, height, scale)
    }
    pub fn prepare(&mut self) -> crate::Result<()> {
        let (context, ..) = self.context();
        // Option de l'application, ou animations désactivées par le bureau.
        let reduce =
            self.options.reduced_motion || crate::platform::linux::desktop::reduced_motion();
        self.runtime.set_reduced_motion(reduce);
        self.runtime
            .set_system_dark(crate::platform::linux::desktop::dark());
        let chrome = ChromeState {
            visible: self.chrome_visible(),
            hovered: self.hovered,
            pressed: self.pressed,
            maximized: self.maximized,
        };
        self.runtime
            .prepare_at(context, chrome, &mut self.renderer, Instant::now())
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
    /// Envoie `event` au runtime ; la boucle applique les demandes de presse-papiers.
    pub fn input(&mut self, event: UiEvent) -> crate::Result<EventResult> {
        // Déplacements et réveils : l'arbre de l'image précédente suffit ; la préparation
        // attend la prochaine image (comme sous Windows et macOS).
        if !matches!(event, UiEvent::PointerMove(_) | UiEvent::Timer(_)) {
            self.prepare()?;
        }
        if matches!(event, UiEvent::Scroll { .. } | UiEvent::Wheel { .. }) {
            self.scrolling_until = Some(Instant::now() + Duration::from_millis(150));
        }
        let paints = self.runtime.stats().paints;
        let mut result = self.runtime.event(event);
        result.repaint |= self.runtime.has_state_styles() || self.runtime.stats().paints != paints;
        self.dirty |= result.repaint;
        Ok(result)
    }
    pub fn next_wake(&self) -> Option<Instant> {
        [self.runtime.next_wake(), self.refine_at, self.trim_at]
            .into_iter()
            .flatten()
            .min()
    }
    /// Échéances atteintes : réveil du runtime (animations, infobulles), redessin exact
    /// après une animation, allègement du GPU une seconde après la dernière image.
    pub fn tick(&mut self, now: Instant) -> crate::Result<()> {
        let dark = crate::platform::linux::desktop::dark();
        if dark != self.system_dark {
            self.system_dark = dark;
            self.dirty = true;
        }
        if self.refine_at.is_some_and(|at| at <= now) {
            self.refine_at = None;
            self.presented = None;
            self.dirty = true;
        }
        if self.trim_at.is_some_and(|at| at <= now) {
            self.trim_at = None;
            if let Some((gpu, _)) = &mut self.gpu {
                gpu.trim(vulkan::Trim::Idle);
            }
        }
        if self.runtime.next_wake().is_some_and(|wake| wake <= now) {
            self.input(UiEvent::Timer(now))?;
        }
        Ok(())
    }
    pub fn take_window_commands(&mut self) -> Vec<crate::window::WindowCommand> {
        self.runtime.take_window_commands()
    }
    /// Montre un dialogue de fichiers (fenêtre parente pour le portail : `x11:<XID>`, ou
    /// vide) sans bloquer la boucle ; le choix est remis par `tasks`, comme un résultat de
    /// tâche, après un réveil.
    pub fn file_dialog(&self, task: TaskId, dialog: FileDialog, parent: String) {
        self.dialogs
            .open(task, dialog, parent, Arc::clone(&self.wake));
    }
    /// Lance, annule et livre les tâches de fond et les choix des dialogues de fichiers ;
    /// `true` si l'application a changé.
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
            let Some(result) = self.scope.take_next().or_else(|| self.dialogs.take_next()) else {
                break;
            };
            changed |= self.runtime.task_completed(result);
        }
        self.dirty |= changed;
        changed
    }
}
impl crate::platform::linux::accessibility::Host for Session {
    fn id(&self) -> WindowId {
        self.id
    }
    fn title(&self) -> &str {
        self.options.title
    }
    fn size(&self) -> (i32, i32) {
        self.logical_size()
    }
    fn runtime(&self) -> &UiRuntime {
        &self.runtime
    }
    fn active(&self) -> bool {
        self.focused
    }
    /// Action d'un lecteur d'écran, appliquée sur un arbre à jour.
    fn act(&mut self, action: crate::ui::runtime::AccessibilityAction) -> crate::Result<bool> {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.prepare()?;
            let applied = self.runtime.accessibility_action(action).is_ok();
            self.prepare()?;
            self.dirty = true;
            Ok(applied)
        }));
        let error = match outcome {
            Ok(Ok(applied)) => return Ok(applied),
            Ok(Err(error)) => error,
            Err(_) => crate::Error::Panicked {
                message: "application panicked during an accessibility action".into(),
            },
        };
        self.failure.get_or_insert(error);
        Ok(false)
    }
}
