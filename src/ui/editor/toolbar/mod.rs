//! Barre d'outils (KUI-033) : boutons à icône, bascules, choix exclusifs, séparateurs,
//! bulles avec raccourci, et menu de débordement quand la place manque.
mod model;
#[cfg(test)]
mod tests;
mod view;

pub use model::ToolItem;
pub use view::{Toolbar, ToolbarMsg, ToolbarState};
