//! Native computed styles. Declaration values are shared by CSS and Rust.
//! Styles resolve when the tree or interaction state changes, before layout.
/// Colours evaluated while styles resolve (`currentColor`, custom properties).
pub mod color;
mod layout;
mod media;
/// Painting helpers that honour per-corner radii from computed styles.
pub mod paint;
mod properties;
mod spacing;
pub(crate) mod variables;
pub use color::{
    BackgroundExpression, ColorExpression, MixSpace, ShadowExpression, ShadowExpressions,
    StopExpression, color_mix,
};
pub use layout::LayoutProperties;
pub use media::{Container, ContainerQuery, ContainerType, MediaQuery};
pub use spacing::SpacingExpressions;
pub use variables::{CssVariables, VariableValue};
pub(crate) mod resolve;
/// Text formatting, measurement and drawing driven by computed typography.
pub mod text;
/// Pre-tokenized values of framework stylesheets evaluated while styles resolve.
pub mod tokens;
pub use properties::*;
mod scheme;
pub use resolve::{
    AnimatedSizes, ElementColors, ElementSizes, Partial, StyleChanges, StyleEnvironment, resolve,
};
pub use scheme::color_scheme;
pub(crate) use scheme::set_color_scheme;
mod styles;
pub(crate) use styles::*;
