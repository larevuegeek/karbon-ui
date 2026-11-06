//! Composable native components and their presentation recipes.
/// Push button component.
pub mod button;
mod legacy;
/// Paragraph made of styled runs and links.
pub mod rich_text;
/// Compiled inline SVG drawing.
pub mod svg;
/// Text label component.
pub mod text;
pub use self::choices::{Checkbox, ChoiceStyle, RadioButton, Switch, radio_group};
pub use self::select::{Select, SelectOption, SelectStyle};
pub use self::slider::{Slider, SliderStyle};
pub use self::text_input::{InputStyle, TextInput};
use crate::application::Action;
pub use crate::ui::core::style::{
    ButtonColors, ButtonStyle, StyleColor, SurfaceStyle, TextStyle, ThemeColor,
};
pub use crate::ui::overlays::popover::{MenuItem, MenuSeparator};
pub use crate::ui::overlays::tooltip::{Tooltip, TooltipStyle};
use crate::ui::{
    element::*,
    layout::{height, width},
    scene::{DrawCommand, RectF, Scene},
    theme::Color,
};
pub use button::{Button, ButtonVariant};
pub use rich_text::{RichText, TextRun};
use std::{cell::Cell, sync::Arc};
pub use svg::Svg;
pub use text::Text;

pub use legacy::{button, text};

pub mod choices;
pub mod select;
pub mod slider;
pub mod text_input;
/// Custom window title bar for frameless windows.
pub mod title_bar;
