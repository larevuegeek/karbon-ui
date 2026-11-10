//! Valeurs CSS : nombres, longueurs, angles, couleurs, dégradés, ombres, filtres,
//! transformations, variables et expressions, puis traduction des propriétés
//! (`properties`) en propriétés du moteur (nom de champ, code Rust).
mod colors;
mod dynamic;
mod effects;
mod expression;
mod filters;
mod gradients;
mod grid;
mod properties;
mod shadows;
mod variables;
mod view;
pub use colors::color;
pub use dynamic::declaration;
pub use effects::transform;
pub use properties::property;
pub use variables::{custom_value, pseudo_content, resolve};
mod basics;
pub use basics::*;
