//! Core building blocks: elements, geometry, style recipes and themes.
/// Element tree, layout constraints and widget extension points.
pub mod element;
/// Integer and floating-point geometry primitives.
pub mod geometry;
/// Hachage rapide des tables internes du moteur.
pub(crate) mod hash;
/// Typed, shareable style recipes for the built-in components.
pub mod style;
/// Colours and the semantic [`Theme`](theme::Theme) palette.
pub mod theme;
