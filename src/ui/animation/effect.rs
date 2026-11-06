use crate::ui::{rendering::transform::Affine, scene::RectF};
/// Visual-only modification of an element: opacity plus a 2D transform applied around the
/// element's transform origin. It never affects layout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Effect {
    /// Opacity multiplier in `0.0..=1.0`.
    pub opacity: f32,
    /// Horizontal offset in logical pixels.
    pub x: f32,
    /// Vertical offset in logical pixels.
    pub y: f32,
    /// Clockwise rotation in degrees.
    pub rotation: f32,
    /// Horizontal scale factor (`1.0` = unchanged).
    pub scale_x: f32,
    /// Vertical scale factor (`1.0` = unchanged).
    pub scale_y: f32,
    /// Horizontal offset in percent of the element's own width, added to
    /// [`x`](Self::x) (`translate(-50%)`).
    pub x_percent: f32,
    /// Vertical offset in percent of the element's own height, added to [`y`](Self::y).
    pub y_percent: f32,
}
impl Default for Effect {
    fn default() -> Self {
        Self::NONE
    }
}
impl Effect {
    /// The identity effect: fully opaque, no transform.
    pub const NONE: Self = Self {
        opacity: 1.,
        x: 0.,
        y: 0.,
        rotation: 0.,
        scale_x: 1.,
        scale_y: 1.,
        x_percent: 0.,
        y_percent: 0.,
    };
    /// Returns [`Effect::NONE`], the starting point for the builder methods.
    pub const fn new() -> Self {
        Self::NONE
    }
    /// Sets the opacity (`0.0..=1.0`).
    pub const fn opacity(mut self, value: f32) -> Self {
        self.opacity = value;
        self
    }
    /// Sets the offset in logical pixels.
    pub const fn translate(mut self, x: f32, y: f32) -> Self {
        self.x = x;
        self.y = y;
        self
    }
    /// Sets the offset in percent of the element's own size (`translate(-50%, 0)`),
    /// added to the offset in pixels.
    pub const fn translate_percent(mut self, x: f32, y: f32) -> Self {
        self.x_percent = x;
        self.y_percent = y;
        self
    }
    /// Sets the clockwise rotation in degrees.
    pub const fn rotate(mut self, degrees: f32) -> Self {
        self.rotation = degrees;
        self
    }
    /// Sets the same scale factor on both axes.
    pub const fn scale(self, value: f32) -> Self {
        self.scale_xy(value, value)
    }
    /// Sets independent horizontal and vertical scale factors.
    pub const fn scale_xy(mut self, x: f32, y: f32) -> Self {
        self.scale_x = x;
        self.scale_y = y;
        self
    }
    pub(crate) fn valid(self) -> bool {
        [
            self.opacity,
            self.x,
            self.y,
            self.rotation,
            self.scale_x,
            self.scale_y,
            self.x_percent,
            self.y_percent,
        ]
        .iter()
        .all(|v| v.is_finite())
            && (0. ..=1.).contains(&self.opacity)
    }
    pub(crate) fn spatial(self) -> bool {
        self.x != 0.
            || self.y != 0.
            || self.x_percent != 0.
            || self.y_percent != 0.
            || self.rotation != 0.
            || self.scale_x != 1.
            || self.scale_y != 1.
    }
    pub(crate) fn mix(self, to: Self, t: f32) -> Self {
        let f = |a, b| a + (b - a) * t;
        Self {
            opacity: f(self.opacity, to.opacity).clamp(0., 1.),
            x: f(self.x, to.x),
            y: f(self.y, to.y),
            rotation: f(self.rotation, to.rotation),
            scale_x: f(self.scale_x, to.scale_x),
            scale_y: f(self.scale_y, to.scale_y),
            x_percent: f(self.x_percent, to.x_percent),
            y_percent: f(self.y_percent, to.y_percent),
        }
    }
    pub(crate) fn combine(self, other: Self) -> Self {
        Self {
            opacity: self.opacity * other.opacity,
            x: self.x + other.x,
            y: self.y + other.y,
            rotation: self.rotation + other.rotation,
            scale_x: self.scale_x * other.scale_x,
            scale_y: self.scale_y * other.scale_y,
            x_percent: self.x_percent + other.x_percent,
            y_percent: self.y_percent + other.y_percent,
        }
    }
    pub(crate) fn matrix(self, bounds: RectF, origin: (f32, f32)) -> Affine {
        let (s, c) = self.rotation.rem_euclid(360.).to_radians().sin_cos();
        let (x, y) = (
            bounds.left + (bounds.right - bounds.left) * origin.0,
            bounds.top + (bounds.bottom - bounds.top) * origin.1,
        );
        Affine::translation(-x, -y)
            .then(Affine {
                m11: self.scale_x * c,
                m12: self.scale_x * s,
                m21: -self.scale_y * s,
                m22: self.scale_y * c,
                dx: 0.,
                dy: 0.,
            })
            .then(Affine::translation(
                x + self.x + (bounds.right - bounds.left) * self.x_percent / 100.,
                y + self.y + (bounds.bottom - bounds.top) * self.y_percent / 100.,
            ))
    }
}
