//! Entrées brutes pour l'application : les trois boutons de la souris sur un élément
//! ([`Element::on_pointer`](crate::ui::element::Element::on_pointer)) et toutes les touches
//! du clavier ([`Application::key`](crate::application::Application::key)).
//!
//! Elles s'ajoutent aux événements du runtime sans rien leur retirer : un bouton gauche,
//! une touche de navigation font toujours ce qu'ils faisaient, et l'application les voit
//! en plus. Un outil de dessin, un jeu, une vue 3D s'en servent pour les gestes que les
//! contrôles ordinaires ne connaissent pas (bouton droit glissé, raccourcis par lettre).
mod keys;
mod physical;
mod pointer;

pub use keys::{KeyCode, KeyPress};
pub use physical::PhysicalKey;
pub use pointer::{PointerButton, PointerEvent, PointerPhase};
// `PointerInput` ne sert qu'au backend Windows (et aux tests) pour l'instant.
#[cfg_attr(not(windows), allow(unused_imports))]
pub(crate) use pointer::{PointerHandler, PointerInput, RawPointerState};
