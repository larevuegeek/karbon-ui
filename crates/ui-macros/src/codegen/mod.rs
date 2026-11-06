//! Briques communes de la génération de code : assemblage des jetons produits et
//! traduction des propriétés de style en appels de nœud.
mod output;
mod properties;
pub use output::Output;
pub use properties::{StyleProp, canonical, classes, property};
