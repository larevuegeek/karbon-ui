//! Common imports for writing a view: `use karbon_ui::ui::prelude::*;`.
//!
//! Brings in [`Application`], [`Error`] and [`Result`], the element builders,
//! components, layout and style types, and the template macros. Backends and the
//! runtime stay explicit imports.
pub use super::template::{Component, Slots, TemplateContext};
pub use super::{
    animation::{Animation, Easing, Effect, FillMode, Keyframe, Transition},
    components::{
        Button, ButtonColors, ButtonStyle, ButtonVariant, Checkbox, ChoiceStyle, InputStyle,
        RadioButton, RichText, Select, SelectOption, SelectStyle, Slider, SliderStyle, StyleColor,
        SurfaceStyle, Switch, Text, TextInput, TextStyle, ThemeColor,
    },
    core::{
        element::{
            Align, Axis, BoxSizing, Element, ElementId, FlexWrap, Insets, Justify, Length,
            Overflow, Position, Split, Widget,
        },
        theme::{Color, Theme},
    },
    layout::{ComputedLength, Expression, GridTrack, TrackBreadth},
    overlays::{
        modal::{Modal, ModalPlacement},
        popover::{Anchor, ContextMenuRequest, Popover},
    },
    rendering::brush::{
        Background, GradientStop, LinearGradient, RadialExtent, RadialGradient, linear_gradient,
    },
    rendering::component_shadow::{BoxShadow, Shadows},
    rendering::filter::{Filter, Filters},
    rendering::font::{Font, FontAsset},
    styling::{
        CssVariables, LineHeight, Properties as StyleProperties, StyleState, TextAlign,
        TextDecoration, VariableValue, color_scheme,
    },
    view::*,
};
pub use crate::{
    Error, Result,
    application::{Action, Application, ColorPreferences, ColorScheme, ViewContext},
    css_file, styles, ui, ui_file,
};
