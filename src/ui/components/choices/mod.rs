//! Application-controlled choices: checkboxes, switches and radio groups.
use crate::application::{Action, Handler};
use crate::ui::{
    element::*,
    layout,
    scene::{DrawCommand, PointF, RectF, Scene},
    theme::Color,
};
use std::sync::Arc;

macro_rules! choice_widget {
    ($type:ty,$role:expr,$switch:expr,$state:expr) => {
        impl Widget for $type {
            fn apply_style(&mut self, value: &crate::ui::styling::ComputedStyle) -> bool {
                crate::ui::styling::text::update(&mut self.css, value)
            }
            fn measure(&self, _: SizeF, text: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
                measure(&self.label, $switch, self.css.as_deref(), text)
            }
            fn paint(&self, ctx: &PaintContext, scene: &mut Scene) {
                paint(
                    drawing::ChoicePaint {
                        label: &self.label,
                        choice: ($state)(self),
                        switch: $switch,
                        enabled: self.enabled,
                        style: &self.style,
                        css: self.css.as_deref(),
                    },
                    ctx,
                    scene,
                );
            }
            fn action(&self) -> Option<Action> {
                self.enabled.then(|| self.activation())
            }
            fn choice(&self) -> Option<ChoiceState> {
                Some(($state)(self))
            }
            fn semantics(&self) -> Semantics<'_> {
                Semantics {
                    role: $role,
                    name: &self.label,
                    enabled: self.enabled,
                }
            }
        }
    };
}
mod checkbox;
mod drawing;
mod radio;
mod style;
mod switch;
pub use checkbox::Checkbox;
use drawing::{measure, paint};
pub(crate) use radio::group_widget;
pub(crate) use radio::radio_groups;
pub use radio::{RadioButton, radio_group};
pub use style::ChoiceStyle;
pub use switch::Switch;
