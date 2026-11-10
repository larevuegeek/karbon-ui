//! Couleurs calculées pendant la résolution des styles : variables héritées,
//! `currentColor` et fonctions de couleur dont un canal dépend d'une variable.
//! Les couleurs constantes sont évaluées par le compilateur CSS.
mod expression;
#[cfg(test)]
mod tests;
pub use expression::*;
