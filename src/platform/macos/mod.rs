//! Backend macOS : Core Graphics, Core Text et ImageIO (liaisons de la famille `objc2`).
//!
//! Services hors écran (texte, images, rastérisation, PNG, compteurs du processus) et
//! boucle AppKit : fenêtres, souris, clavier et IME, défilement, curseurs, presse-papiers,
//! barre de menus, minuteries, tâches de fond et accessibilité (VoiceOver).
//!
//! Carte des modules, en miroir de `platform/windows/` :
//!
//! | Module | Rôle |
//! | --- | --- |
//! | `accessibility/` | pont NSAccessibility : rôles, valeurs, actions, élément accessible |
//! | `window/` | boucle AppKit, sessions de fenêtre, vue et délégué Objective-C, traduction des entrées |
//! | `graphics/` | renderer Core Graphics/Core Text : canvas, formes, dégradés, texte, filtres CPU |
//! | `metal/` | renderer GPU Metal des fenêtres (repli et rendu hors écran : `graphics/`) |
//! | `fonts.rs` | polices embarquées (Core Text, WOFF/WOFF2 compris) et familles système |
//! | `imaging.rs` | décodage d'images et export PNG par ImageIO |
//! | `offscreen.rs` | services hors écran (`Offscreen`) et compteurs du processus |
//! | `ffi.rs` | conversions Core Foundation/Core Graphics partagées |
//! | `tests/` | suite minimale du backend |
mod accessibility;
mod bitmap;
mod ffi;
mod fonts;
mod graphics;
mod imaging;
mod metal;
mod offscreen;
#[cfg(test)]
mod tests;
mod window;

/// Implémentation de la frontière [`Backend`](super::Backend) pour macOS.
pub(crate) struct MacOs;
impl super::Backend for MacOs {
    type Offscreen = offscreen::NativeOffscreen;
    fn run_windows(
        windows: Vec<crate::window::WindowDescriptor>,
        exit_policy: crate::window::ExitPolicy,
    ) -> crate::Result<()> {
        window::run_windows(windows, exit_policy)
    }
    fn offscreen(renderer: crate::window::RenderMode) -> crate::Result<Self::Offscreen> {
        offscreen::NativeOffscreen::new(renderer)
    }
    fn process_sample() -> crate::Result<crate::diagnostics::ProcessSample> {
        offscreen::sample()
    }
    fn write_png(image: &crate::ui::scene::Image, path: &std::path::Path) -> crate::Result<()> {
        imaging::write_png(image, path)
    }
}
