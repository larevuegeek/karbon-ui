//! Sélecteurs complets des feuilles de frameworks : types, universel, classes, id,
//! attributs, pseudo-classes connues du moteur, `:is()`, `:where()`, `:not()`,
//! combinateurs et pseudo-éléments. Spécificité (a, b, c) de Selectors 4 ; évaluation
//! à trois valeurs du composé sujet contre les faits statiques d'un élément de
//! template, et code `StyleState` des parties évaluées à l'exécution.
mod matching;
mod parse;

pub use matching::{Facts, Tri, subject_and_context};
pub use parse::{parse, split_list};
mod model;
pub use model::*;
