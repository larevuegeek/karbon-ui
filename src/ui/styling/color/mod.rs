//! Couleurs calculées pendant la résolution des styles : variables héritées,
//! `currentColor` et fonctions de couleur dont un canal dépend d'une variable.
//! Les couleurs constantes sont évaluées par le compilateur CSS. `layers` : fonds en
//! dégradé et ombres dont les couleurs lisent des variables.
mod expression;
mod layers;
#[cfg(test)]
mod tests;
pub use expression::*;
pub use layers::*;
