//! Effets visuels du nœud : opacité, transformations, états survolé/pressé,
//! transitions et animations.
use super::Node;
use crate::ui::{
    animation::{Animation, Effect, Transition},
    element::*,
};
impl<W: Widget + 'static> Node<W> {
    /// Sets the group opacity, from `0.` to `1.`.
    pub fn opacity(mut self, value: f32) -> Self {
        self.element = self.element.opacity(value);
        self
    }
    /// Sets the base visual effect (transform and opacity).
    pub fn effect(mut self, value: Effect) -> Self {
        self.element = self.element.effect(value);
        self
    }
    /// Replace the spatial transform while keeping the independent group opacity.
    pub fn transform(mut self, mut value: Effect) -> Self {
        value.opacity = self.element.motion.as_ref().map_or(1., |m| m.base.opacity);
        self.element = self.element.effect(value);
        self
    }
    /// Rotates the node clockwise by `value` degrees around its transform origin.
    pub fn rotate(mut self, value: f32) -> Self {
        self.element = self.element.rotate(value);
        self
    }
    /// Scales the node uniformly.
    pub fn scale(mut self, value: f32) -> Self {
        self.element = self.element.scale(value);
        self
    }
    /// Translates the node by `(x, y)` pixels.
    pub fn translate(mut self, value: (f32, f32)) -> Self {
        self.element = self.element.translate(value.0, value.1);
        self
    }
    /// Sets the transform origin as fractions of the node bounds (`(0.5, 0.5)` is the centre).
    pub fn origin(mut self, value: (f32, f32)) -> Self {
        self.element = self.element.transform_origin(value.0, value.1);
        self
    }
    /// Sets the effect applied while hovered.
    pub fn hover(mut self, value: Effect) -> Self {
        self.element = self.element.hover(value);
        self
    }
    /// Sets the hover effect from CSS, replacing only the opacity and/or transform
    /// that the rule declares.
    pub fn css_hover(mut self, value: Effect, opacity: bool, transform: bool) -> Self {
        self.element = self.element.hover(value);
        self.element.motion.as_mut().unwrap().hover_replace =
            Some(crate::ui::animation::ReplacedProperties { opacity, transform });
        self
    }
    /// Sets the effect applied while pressed.
    pub fn pressed(mut self, value: Effect) -> Self {
        self.element = self.element.pressed(value);
        self
    }
    /// Sets the pressed effect from CSS, replacing only the opacity and/or transform
    /// that the rule declares.
    pub fn css_pressed(mut self, value: Effect, opacity: bool, transform: bool) -> Self {
        self.element = self.element.pressed(value);
        self.element.motion.as_mut().unwrap().pressed_replace =
            Some(crate::ui::animation::ReplacedProperties { opacity, transform });
        self
    }
    /// Sets the transition used when effects change.
    pub fn transition(mut self, value: Transition) -> Self {
        self.element = self.element.transition(value);
        self
    }
    /// Starts a keyframe animation.
    pub fn animate(mut self, value: Animation) -> Self {
        self.element = self.element.animate(value);
        self
    }
    /// Animates the height between `0` and the natural height when the node is shown or
    /// hidden (see [`Element::reveal`](crate::ui::element::Element::reveal)).
    pub fn reveal(mut self, value: Transition) -> Self {
        self.element = self.element.reveal(value);
        self
    }
}
