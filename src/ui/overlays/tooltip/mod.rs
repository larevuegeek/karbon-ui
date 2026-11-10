//! Delayed textual help, independent of control layout and hit-testing.
mod config;
mod drawing;
mod state;
pub use config::{Tooltip, TooltipStyle};
mod model;
pub use model::*;
