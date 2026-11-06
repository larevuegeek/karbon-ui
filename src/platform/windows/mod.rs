//! Backend Windows : Win32, Direct2D/DirectWrite, DirectComposition, GDI+ et UI Automation.
//!
//! Carte des modules, à reproduire pour un futur backend macOS ou Linux :
//!
//! | Module | Rôle |
//! | --- | --- |
//! | `window/` | boucle d'événements, fenêtres natives (création, état, commandes), effets DWM, ombre, minuteries, réveil des tâches, trace de démarrage |
//! | `events/` | procédure de fenêtre : messages Win32 traduits en `UiEvent` (pointeur, clavier, zone non cliente), saisie de texte, IME et presse-papiers |
//! | `present/` | état de dessin d'une fenêtre, peinture et présentation (DirectComposition ou surface layered), mesures de présentation |
//! | `accessibility/` | pont UI Automation : arbre publié, fournisseurs COM et patterns |
//! | `graphics/` | renderer Direct2D/DirectWrite : texte, formes, effets, chaîne d'échange |
//! | `fonts.rs` | collection DirectWrite des polices embarquées |
//! | `imaging.rs` | décodage d'images et export PNG par GDI+ |
//! | `offscreen.rs` | services hors écran (`Offscreen`) et compteurs du processus |
//! | `ffi.rs` | types et conversions Win32 partagés, appartement COM |
//! | `tests/` | tests du backend sur des fenêtres cachées |
mod accessibility;
mod events;
mod ffi;
mod fonts;
mod graphics;
mod imaging;
mod offscreen;
mod present;
#[cfg(test)]
mod tests;
mod window;

// Surface layered utilisée par les tests de `graphics` (chemin `windows::surface`).
#[cfg(test)]
use present::surface;

/// Implémentation de la frontière [`Backend`](super::Backend) pour Windows.
pub(crate) struct Windows;
impl super::Backend for Windows {
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
