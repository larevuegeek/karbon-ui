//! Declarative composition: the same elements and runtime, without an extra rendering layer.
mod lazy;
mod style;
pub use lazy::{lazy, memo_key, memo_placeholder, memo_reusable};
pub(crate) use lazy::{previous_keys, transplant, with_previous};
pub use style::ViewStyle;
mod constructors;
#[cfg(test)]
mod tests;
pub use constructors::*;
