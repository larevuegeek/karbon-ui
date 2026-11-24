//! Champ numérique qu'on fait glisser, et vecteurs X, Y, Z (KUI-031) : glisser change la
//! valeur, un clic donne la saisie au clavier (expressions simples), bornes, pas, unité.
mod expression;
#[cfg(test)]
mod payload_tests;
mod state;
#[cfg(test)]
mod tab_tests;
#[cfg(test)]
mod tests;
mod view;

pub use state::{FieldChange, FieldEvent, FieldId, FieldMsg, FieldReply, FieldState};
pub use view::{NumberField, VectorField};
