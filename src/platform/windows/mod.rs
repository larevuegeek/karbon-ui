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
mod backend;
mod events;
mod ffi;
mod fonts;
mod graphics;
mod imaging;
mod offscreen;
mod present;
pub(crate) mod secrets;
#[cfg(test)]
mod tests;
mod window;
pub(crate) use backend::*;
