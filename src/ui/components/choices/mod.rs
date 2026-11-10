//! Application-controlled choices: checkboxes, switches and radio groups.
#[macro_use]
mod choice_widget;
mod checkbox;
mod drawing;
mod radio;
mod style;
mod switch;
pub use checkbox::Checkbox;
use choice_widget::*;
pub(crate) use radio::group_widget;
pub(crate) use radio::radio_groups;
pub use radio::{RadioButton, radio_group};
pub use style::ChoiceStyle;
pub use switch::Switch;
