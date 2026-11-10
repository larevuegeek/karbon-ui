//! Présentation d'une fenêtre : état de dessin (renderer, runtime UI, surface),
//! préparation de l'image et choix du renderer. La peinture est dans `paint`, les
//! requêtes du reste du backend au runtime dans `runtime`.
//!
//! Le backend reçoit une application ; il ne connaît aucune galerie ni page métier.
pub(crate) mod composition;
mod drawing;
pub(crate) mod paint;
mod recorder;
pub(crate) mod runtime;
pub(super) mod surface;
pub(crate) use drawing::*;
