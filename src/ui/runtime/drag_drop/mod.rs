//! In-app drag and drop: stable identities, compatible groups and before/after drops.
mod interaction;
pub(super) use interaction::State;
mod model;
pub use model::*;
