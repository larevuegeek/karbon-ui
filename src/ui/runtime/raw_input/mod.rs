//! Entrées brutes pour l'application : les trois boutons de la souris sur un élément
//! ([`Element::on_pointer`](crate::ui::element::Element::on_pointer)) et toutes les touches
//! du clavier ([`Application::key`](crate::application::Application::key)).
//!
//! Elles s'ajoutent aux événements du runtime sans rien leur retirer : un bouton gauche,
//! une touche de navigation font toujours ce qu'ils faisaient, et l'application les voit
//! en plus. Un outil de dessin, un jeu, une vue 3D s'en servent pour les gestes que les
//! contrôles ordinaires ne connaissent pas (bouton droit glissé, raccourcis par lettre).
mod cursor;
mod file_drop;
mod keys;
mod lock;
mod panel_keys;
mod pen;
mod physical;
mod pointer;
mod text;

pub use cursor::{CursorImage, CursorShape};
pub use file_drop::{FileDropEvent, FileDropPhase};
pub use keys::{KeyCode, KeyPress};
pub use physical::PhysicalKey;
pub use pointer::{PointerButton, PointerEvent, PointerPhase};
pub use text::TextEvent;
pub(crate) use text::TextHandler;
// `PointerInput` ne sert qu'au backend Windows (et aux tests) pour l'instant.
pub(crate) use file_drop::{FILE_DROP_BUILT, FileDropHandler};
pub(crate) use panel_keys::{KeyFocus, KeyHandler};
#[cfg_attr(not(windows), allow(unused_imports))]
pub(crate) use pen::{PenSample, pen_pressure};
pub(crate) use pointer::target as pointer_target;
#[cfg_attr(not(windows), allow(unused_imports))]
pub(crate) use pointer::{PointerHandler, PointerInput, RawPointerState};
