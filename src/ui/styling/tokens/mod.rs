//! CSS values of framework stylesheets that depend on custom properties known only at
//! run time (`var(--bs-btn-bg)` set by `.btn-primary`,
//! `rgba(var(--bs-primary-rgb), var(--bs-text-opacity))`…).
//!
//! [`css_file!`](crate::css_file) splits such values into [`Token`](crate::ui::styling::tokens::Token)s at compile time.
//! While styles resolve, `var()` groups are replaced by the tokens of the custom
//! properties in scope and the property grammar is evaluated on the result; no CSS text
//! is parsed at run time.
mod apply;
mod background;
mod border;
mod color;
mod compatibility;
mod property;
mod reader;
mod shadow;
mod substitution;
#[cfg(test)]
mod tests;
mod token;
mod transition;

pub(crate) use apply::evaluate;
pub use apply::evaluation_count;
pub(crate) use color::color_of;
pub use compatibility::Compatibility;
pub use property::{DynamicDeclaration, DynamicProperty};
pub(crate) use reader::Evaluation;
pub use substitution::{Globals, Registry};
pub(crate) use substitution::{compute_custom, lookup, substitute, typed_tokens};
pub use token::{Token, Unit};
