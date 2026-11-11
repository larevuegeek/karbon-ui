//! Contrat d'un backend de plateforme : tout ce qu'un backend (Windows, macOS, Linux,
//! headless) doit fournir au moteur, réuni ici.
//!
//! Le moteur (`UiRuntime`, layout, styles, `Scene`) ne connaît aucun OS : il consomme des
//! `UiEvent`, mesure le texte via `TextMeasurer` et peint dans une `Scene`.
//! `platform/mod.rs` choisit l'implémentation (`Native`) en un seul endroit et toutes les
//! entrées publiques passent par elle.
//!
//! # Services typés
//!
//! - [`Backend::run_windows`] : boucle d'événements et fenêtres (obligations ci-dessous).
//! - [`Backend::offscreen`] : un [`Offscreen`] (mesure du texte, décodage d'images,
//!   polices embarquées, rastérisation d'une `Scene` en pixels) utilisé par le protocole
//!   commun des aperçus et des mesures (`platform/offscreen.rs`).
//! - [`Backend::process_sample`] : compteurs mémoire/CPU du processus pour les mesures.
//! - [`Backend::write_png`] : encodage PNG d'une image.
//!
//! # Cadence d'une image
//!
//! Le gros travail d'une image doit passer pendant l'attente de l'écran, pas après elle.
//! Un backend qui sait où est cette attente appelle, pour chaque image présentée :
//!
//! 1. `UiRuntime::begin_frame_work`, puis `UiRuntime::prepare` si
//!    `UiRuntime::rebuild_pending` : juste après la présentation, avant d'attendre ;
//! 2. `UiRuntime::hold_rebuild`, quand l'attente est finie : une reconstruction demandée
//!    ensuite passe à l'image suivante ;
//! 3. `UiEvent::Timer`, puis la préparation et la présentation de l'image.
//!
//! Windows le fait toujours (`events/dispatch.rs`, `FRAME_WAKE`), Wayland par défaut
//! (rappel de trame, mesuré ; `KARBON_FRAME_PACING=0` pour l'ôter). X11 et macOS, dont
//! l'attente n'est qu'estimée, le font avec `KARBON_FRAME_PACING=1`, tant que le gain n'y
//! est pas mesuré (`platform/shared/pacing.rs`). Chaque présentation appelle aussi
//! `UiRuntime::frame_presented` (sonde de défilement).
//!
//! # Obligations de la boucle d'événements
//!
//! Presse-papiers, minuteries, réveil et accessibilité ne sont pas des traits : ce sont des
//! détails internes de `run_windows`, écrits contre l'API publique du runtime. La colonne
//! Windows indique où lire l'implémentation de référence (`platform/windows/`).
//!
//! | Obligation | API du moteur | Windows |
//! | --- | --- | --- |
//! | Valider les descripteurs avant d'ouvrir | `window::validation` | `window/event_loop.rs`, `window/session.rs` |
//! | Créer, fermer et commander les fenêtres | `UiRuntime::take_window_commands`, `WindowCommand` | `window/` |
//! | Changer une fenêtre ouverte (`WindowCommand::Change`), remettre son état après chaque changement, honorer les poignées `data-resize-region` (même sur une fenêtre fixe, dont les bords ne redimensionnent pas ; geste suivi en coordonnées de l'écran par `window::change::grip_size` quand la plateforme le permet) | `window::change`, `UiRuntime::window_changed`, `UiRuntime::resize_region_at` | `window/change.rs`, `events/nonclient.rs`, `events/grip.rs` |
//! | Traduire les entrées natives (positions logiques, DPI appliqué) | `UiRuntime::event(UiEvent)` | `events/pointer.rs`, `events/keyboard.rs`, `events/nonclient.rs` |
//! | Honorer capture et libération du pointeur, curseur | `EventResult::{capture, release}`, `UiRuntime::cursor` | `events/`, `present/runtime.rs` |
//! | Saisie de texte, IME et presse-papiers (`Write`, puis `EditCommand::DeleteSelection` pour couper ; `Read` renvoyé en `UiEvent::Text`) | `EventResult::clipboard`, `ClipboardRequest` | `events/text.rs`, `events/clipboard.rs` |
//! | Programmer un réveil à l'échéance demandée et envoyer `UiEvent::Timer` | `UiRuntime::next_wake` | `window/timer.rs` |
//! | Créer un `TaskScope` par fenêtre, réveiller le thread UI, livrer les résultats | `TaskScope::new(wake)`, `UiRuntime::take_task_commands`, `task_completed` | `window/tasks.rs` |
//! | Exposer l'arbre accessible et appliquer les actions du lecteur d'écran | `UiRuntime::accessibility`, `accessibility_action` | `accessibility/` |
//! | Préparer, dessiner chaque `DrawCommand` de la `Scene` et présenter | `UiRuntime::prepare`, `UiRuntime::scene` | `present/`, `graphics/` |
//! | Test de contenu d'une fenêtre transparente (facultatif) | `ui::rendering::scene::coverage` | `present/runtime.rs` |
//!
//! Le backend headless (`platform/headless/`) implémente les services typés sans boucle
//! d'événements ; son harnais `HeadlessApp` joue le rôle de la boucle dans les tests.
use crate::{
    application::Resources,
    diagnostics::ProcessSample,
    ui::{element::TextMeasurer, font::FontAsset, scene::Image, scene::Scene},
    window::{ExitPolicy, RenderMode, WindowDescriptor},
};
use std::path::Path;

/// Services fournis par un backend de plateforme (Windows, macOS, Linux, headless).
/// Fonctions associées uniquement : un backend n'a pas d'état global exposé.
pub(crate) trait Backend {
    /// Moteur hors écran : texte, images et rastérisation.
    type Offscreen: Offscreen;
    /// Ouvrir `windows` et exécuter la boucle d'événements jusqu'à `exit_policy`.
    fn run_windows(windows: Vec<WindowDescriptor>, exit_policy: ExitPolicy) -> crate::Result<()>;
    /// Créer un moteur hors écran pour le mode de rendu demandé.
    fn offscreen(renderer: RenderMode) -> crate::Result<Self::Offscreen>;
    /// Compteurs du processus courant ; zéro quand la plateforme ne les fournit pas.
    fn process_sample() -> crate::Result<ProcessSample>;
    /// Encoder `image` en PNG dans `path`.
    fn write_png(image: &Image, path: &Path) -> crate::Result<()>;
}

/// Moteur hors écran d'un backend : il mesure le texte avec les mêmes polices qu'il
/// dessine, décode les images et rastérise une `Scene` en pixels BGRA prémultipliés.
pub(crate) trait Offscreen: TextMeasurer {
    /// Libellé du moteur, transmis à `ViewContext::backend`.
    fn label(&self) -> &'static str;
    /// Services passés à `Application::load`.
    fn resources(&self) -> &dyn Resources;
    /// Enregistrer les polices embarquées de l'application.
    fn load_fonts(&mut self, fonts: &[FontAsset]) -> crate::Result<()>;
    /// Rastériser `scene` à `scale` dans une surface de `width` × `height` pixels.
    fn render(
        &mut self,
        scene: &Scene,
        width: u32,
        height: u32,
        scale: f32,
    ) -> crate::Result<Vec<u8>>;
}
