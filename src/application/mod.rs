//! The [`Application`] trait implemented by every window's application, and the
//! services and context it receives from the runtime.
mod colors;
mod message;
pub use colors::{ColorPreferences, ColorScheme};
pub use message::Action;
pub(crate) use message::{Handler, Message};
mod contract;
pub use contract::*;
