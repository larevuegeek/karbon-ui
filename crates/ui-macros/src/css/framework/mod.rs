//! Mode « framework » de `css_file!` : une feuille existante (Bootstrap, sortie de
//! Tailwind…) est lue avec tolérance. Les constructions que le moteur ne sait pas
//! rendre sont sautées et comptées dans un rapport au lieu d'arrêter la compilation ;
//! la cascade (spécificité, ordre, `!important`, `@layer`) est résolue élément par
//! élément quand un template compile ses balises (voir [`element`]).
//!
//! Parcours : `syntax` lit le texte, `walker` aplatit règles et at-rules (conditions
//! évaluées par `conditions`), `compile` construit la [`Sheet`] (classement des valeurs
//! par `values`, sélecteurs par `selectors`), `keyframes` compile les animations et
//! `element` calcule la cascade d'un élément de template.
mod compile;
mod conditions;
pub mod element;
pub mod images;
mod keyframes;
mod report;
pub mod selectors;
pub mod syntax;
pub mod values;
mod walker;

pub use report::Report;

mod model;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tools;
pub use model::*;
