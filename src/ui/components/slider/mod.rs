//! Numeric adjustment: drawing, range and provisional interaction, independent
//! of the OS.
mod drawing;
mod events;
mod geometry;
mod state;
mod style;
mod widget;
pub use style::SliderStyle;
pub use widget::Slider;
mod model;
pub use model::*;
