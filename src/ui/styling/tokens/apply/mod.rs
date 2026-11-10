//! Évaluation des déclarations dynamiques : substitution des `var()` puis application de
//! la valeur à la propriété, répartie par famille de propriétés.
mod border;
mod control;
mod effect;
mod evaluate;
mod layout;
mod paint;
mod text;
pub use evaluate::*;
