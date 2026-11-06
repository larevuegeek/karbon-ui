//! Animations and transitions: monotonic time, visual effects and declarative entry animations.
//!
//! Effects are purely visual (opacity and 2D transform): they never change layout. Builders on
//! [`Element`] attach an [`Effect`], hover/pressed variants, a [`Transition`] or an [`Animation`].
//! The tree is validated before layout: invalid values (non-finite numbers, opacity outside
//! `0..=1`, malformed keyframes) are rejected with an error.
mod easing;
mod effect;
mod keyframes;
mod play;
mod spec;
pub(crate) mod timeline;
mod timing;
use crate::ui::element::Element;
pub use easing::Easing;
pub use effect::Effect;
pub use keyframes::Keyframe;
pub use play::{Animation, FillMode};
pub(crate) use spec::Motion;
pub(crate) use spec::ReplacedProperties;
pub use spec::Transition;
pub(crate) use timing::Due;
impl Animation {
    pub(crate) fn scaled_distance(mut self, scale: f32) -> Self {
        self.distance_scale *= scale;
        self.from.x *= scale;
        self.from.y *= scale;
        self.to.x *= scale;
        self.to.y *= scale;
        self
    }
}
impl Element {
    fn motion_mut(&mut self) -> &mut Motion {
        self.motion
            .get_or_insert_with(|| Box::new(Motion::default()))
    }
    /// Sets the resting visual [`Effect`] of this element, replacing any previous base effect.
    pub fn effect(mut self, effect: Effect) -> Self {
        self.motion_mut().base = effect;
        self
    }
    /// Sets the resting opacity (`0.0..=1.0`) of the element and its subtree.
    pub fn opacity(mut self, opacity: f32) -> Self {
        let m = self.motion_mut();
        m.base = m.base.opacity(opacity);
        self
    }
    /// Visually offsets the element by `(x, y)` logical pixels without affecting layout.
    pub fn translate(mut self, x: f32, y: f32) -> Self {
        let m = self.motion_mut();
        m.base = m.base.translate(x, y);
        self
    }
    /// Rotates the element clockwise by `degrees` around its transform origin.
    pub fn rotate(mut self, degrees: f32) -> Self {
        let m = self.motion_mut();
        m.base = m.base.rotate(degrees);
        self
    }
    /// Uniformly scales the element around its transform origin (`1.0` = unchanged).
    pub fn scale(mut self, scale: f32) -> Self {
        let m = self.motion_mut();
        m.base = m.base.scale(scale);
        self
    }
    /// Sets the transform origin as fractions of the element bounds, each in `0.0..=1.0`
    /// (default `(0.5, 0.5)`, the centre).
    pub fn transform_origin(mut self, x: f32, y: f32) -> Self {
        self.motion_mut().origin = (x, y);
        self
    }
    /// Effect combined with the base effect while the pointer is over the element or one of its
    /// descendants (opacities and scales multiply, translations and rotations add).
    pub fn hover(mut self, effect: Effect) -> Self {
        self.motion_mut().hover = Some(effect);
        self.motion_mut().hover_replace = None;
        self
    }
    /// Effect combined with the base effect while the element or one of its descendants is
    /// pressed; it takes precedence over the [`hover`](Self::hover) effect.
    pub fn pressed(mut self, effect: Effect) -> Self {
        self.motion_mut().pressed = Some(effect);
        self.motion_mut().pressed_replace = None;
        self
    }
    /// Interpolates between effect targets (base, hover, pressed) with the given timing instead
    /// of switching instantly. Ignored when the system requests reduced motion.
    pub fn transition(mut self, transition: Transition) -> Self {
        self.motion_mut().transition = Some(transition);
        self
    }
    /// Animates the height of the element between `0` and its natural (`auto`) height when
    /// it is shown or hidden, like Bootstrap's collapse or CSS
    /// `interpolate-size: allow-keywords` with `transition: height`.
    ///
    /// When a style rule or [`hidden`](crate::ui::element::LayoutRules::hidden) switches
    /// the element between `display: none` and displayed, the element stays displayed
    /// while its border-box height grows from `0` (or shrinks to `0`) with `transition`'s
    /// duration, delay and easing; its content keeps its natural layout and is clipped to
    /// the animated box. Siblings follow the animated height. An interrupted reveal
    /// reverses from its current height. The first appearance is not animated, and
    /// reduced motion applies the final state at once.
    ///
    /// ```
    /// use karbon_ui::ui::prelude::*;
    ///
    /// let panel = column("details", [text("details.body", "More")])
    ///     .reveal(Transition::new().duration(350).easing(Easing::Ease));
    /// # let _ = panel;
    /// ```
    pub fn reveal(mut self, transition: Transition) -> Self {
        self.motion_mut().reveal = Some(transition);
        self
    }
    /// Plays `animation` on this element, starting when the element first appears or when the
    /// animation value changes.
    pub fn animate(mut self, animation: Animation) -> Self {
        self.motion_mut().animation = Some(animation);
        self
    }
}
