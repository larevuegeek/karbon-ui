pub(super) use crate::application::{Action, Handler};
pub(super) use crate::ui::{
    element::*,
    layout,
    scene::{DrawCommand, PointF, RectF, Scene},
    theme::Color,
};
pub(super) use std::sync::Arc;

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
            fn paints_natural_size(&self) -> bool {
                true
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
pub(super) use super::drawing::{measure, paint};
