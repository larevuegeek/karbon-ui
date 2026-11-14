//! Portable background descriptions. Native backends retain their gradient brushes.
use crate::ui::{element::Element, theme::Color};
use std::sync::Arc;
/// Colour at a given position along a gradient.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GradientStop {
    /// Position in `0.0..=1.0`; stops must be in non-decreasing order.
    pub position: f32,
    /// Colour at this position.
    pub color: Color,
}
/// Linear gradient, like CSS `linear-gradient`.
#[derive(Clone, Debug, PartialEq)]
pub struct LinearGradient {
    /// Direction in degrees, clockwise from left-to-right.
    pub angle: f32,
    /// Colour stops (2 to 64).
    pub stops: Arc<[GradientStop]>,
    /// Direction toward a corner of the painted box (`to bottom right` is `[1, 1]`,
    /// `to top left` is `[-1, -1]`): the angle then depends on the box's aspect ratio,
    /// like CSS, and [`angle`](Self::angle) is ignored.
    pub corner: Option<[i8; 2]>,
}
/// How a [`RadialGradient`] sizes its ending shape, like the CSS extent keywords.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RadialExtent {
    /// Use [`RadialGradient::radius`] as fractions of the painted rectangle.
    Explicit,
    /// Use [`RadialGradient::radius`] as logical pixels (`circle 40px`, `30px 20px`).
    Absolute,
    /// Reach the side closest to the centre.
    ClosestSide,
    /// Reach the side farthest from the centre.
    FarthestSide,
    /// Reach the corner closest to the centre.
    ClosestCorner,
    /// Reach the corner farthest from the centre.
    FarthestCorner,
}
/// Radial gradient, like CSS `radial-gradient`.
#[derive(Clone, Debug, PartialEq)]
pub struct RadialGradient {
    /// Centre as `[x, y]` fractions of the painted rectangle.
    pub center: [f32; 2],
    /// Explicit radii as `[x, y]` fractions of the painted rectangle; must be positive.
    pub radius: [f32; 2],
    /// Colour stops (2 to 64).
    pub stops: Arc<[GradientStop]>,
    /// Sizing rule for the ending shape.
    pub extent: RadialExtent,
    /// `true` for a circle, `false` for an ellipse (only used by non-explicit extents).
    pub circle: bool,
}
impl RadialGradient {
    /// Center and radii are fractions of the painted rectangle.
    pub fn new(
        center: [f32; 2],
        radius: [f32; 2],
        stops: impl IntoIterator<Item = GradientStop>,
    ) -> Self {
        Self {
            center,
            radius,
            stops: stops.into_iter().collect(),
            extent: RadialExtent::Explicit,
            circle: false,
        }
    }
    /// Replace the sizing rule and choose between a circle and an ellipse.
    pub fn extent(mut self, value: RadialExtent, circle: bool) -> Self {
        self.extent = value;
        self.circle = circle;
        self
    }
    /// Resolve the `[x, y]` radii, in the same units as the `w` × `h` painted rectangle.
    pub fn radii(&self, w: f32, h: f32) -> [f32; 2] {
        if matches!(self.extent, RadialExtent::Explicit) {
            return [self.radius[0] * w, self.radius[1] * h];
        }
        if matches!(self.extent, RadialExtent::Absolute) {
            return self.radius;
        }
        let x = self.center[0] * w;
        let y = self.center[1] * h;
        let near = matches!(
            self.extent,
            RadialExtent::ClosestSide | RadialExtent::ClosestCorner
        );
        let pick = |a: f32, b: f32| {
            if near {
                a.abs().min(b.abs())
            } else {
                a.abs().max(b.abs())
            }
        };
        let (rx, ry) = (pick(x, w - x), pick(y, h - y));
        if self.circle {
            let r = if matches!(
                self.extent,
                RadialExtent::ClosestCorner | RadialExtent::FarthestCorner
            ) {
                rx.hypot(ry)
            } else {
                pick(rx, ry)
            };
            [r, r]
        } else {
            let factor = if matches!(
                self.extent,
                RadialExtent::ClosestCorner | RadialExtent::FarthestCorner
            ) {
                std::f32::consts::SQRT_2
            } else {
                1.
            };
            [rx * factor, ry * factor]
        }
    }
    /// Check the stops and the geometry; returns [`Error::InvalidInput`](crate::Error::InvalidInput) if invalid.
    pub fn validate(&self) -> crate::Result<()> {
        LinearGradient {
            angle: 0.,
            stops: Arc::clone(&self.stops),
            corner: None,
        }
        .validate()?;
        if self.center.iter().any(|n| !n.is_finite())
            || self.radius.iter().any(|n| !n.is_finite() || *n <= 0.)
        {
            return Err(crate::Error::invalid_input(
                "invalid radial gradient geometry",
            ));
        }
        Ok(())
    }
}
impl LinearGradient {
    /// Gradient with explicit stops; `angle` is in degrees, clockwise from left-to-right.
    pub fn new(angle: f32, stops: impl IntoIterator<Item = GradientStop>) -> Self {
        Self {
            angle,
            stops: stops.into_iter().collect(),
            corner: None,
        }
    }
    /// Gradient toward a corner (`to bottom right` is `(1, 1)`): the gradient line is
    /// perpendicular to the diagonal joining the two other corners, as in CSS.
    pub fn to_corner(x: i8, y: i8, stops: impl IntoIterator<Item = GradientStop>) -> Self {
        Self {
            corner: Some([x.signum(), y.signum()]),
            ..Self::new(0., stops)
        }
    }
    /// Direction in degrees (clockwise from left-to-right) painted in a `width` ×
    /// `height` box: [`angle`](Self::angle), or the corner direction for that box.
    pub fn angle_in(&self, width: f32, height: f32) -> f32 {
        match self.corner {
            Some([x, y]) => (f32::from(y) * width.max(0.))
                .atan2(f32::from(x) * height.max(0.))
                .to_degrees(),
            None => self.angle,
        }
    }
    /// Check that the angle is finite and that there are 2 to 64 sorted stops within
    /// `0.0..=1.0`; returns [`Error::InvalidInput`](crate::Error::InvalidInput) otherwise.
    pub fn validate(&self) -> crate::Result<()> {
        if !self.angle.is_finite()
            || !(2..=64).contains(&self.stops.len())
            || self
                .stops
                .iter()
                .any(|s| !s.position.is_finite() || !(0. ..=1.).contains(&s.position))
            || self.stops.windows(2).any(|p| p[0].position > p[1].position)
        {
            return Err(crate::Error::invalid_input(
                "invalid gradient angle, stop count or stop positions",
            ));
        }
        Ok(())
    }
}
/// Angle in degrees clockwise from left-to-right; colors have evenly spaced stops.
pub fn linear_gradient(angle: f32, colors: impl IntoIterator<Item = Color>) -> LinearGradient {
    let colors = colors.into_iter().collect::<Vec<_>>();
    let denominator = colors.len().saturating_sub(1).max(1) as f32;
    LinearGradient::new(
        angle,
        colors
            .into_iter()
            .enumerate()
            .map(|(i, color)| GradientStop {
                position: i as f32 / denominator,
                color,
            }),
    )
}
/// One background layer: a solid colour or a gradient. Compared by content.
#[derive(Clone, Debug, PartialEq)]
pub enum Background {
    /// Solid colour.
    Solid(Color),
    /// Linear gradient.
    Linear(Arc<LinearGradient>),
    /// Radial gradient.
    Radial(Arc<RadialGradient>),
}
impl From<RadialGradient> for Background {
    fn from(value: RadialGradient) -> Self {
        Self::Radial(Arc::new(value))
    }
}
impl Background {
    /// Validate the gradient, if any; solid colours are always valid.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidInput`](crate::Error::InvalidInput) for an invalid gradient.
    pub fn validate(&self) -> crate::Result<()> {
        match self {
            Self::Solid(_) => Ok(()),
            Self::Linear(v) => v.validate(),
            Self::Radial(v) => v.validate(),
        }
    }
    /// Push the draw command painting this layer into `rect` with a corner `radius`.
    pub fn paint(
        &self,
        scene: &mut crate::ui::scene::Scene,
        rect: crate::ui::scene::RectF,
        radius: f32,
    ) {
        use crate::ui::scene::DrawCommand;
        match self {
            Self::Solid(color) => scene.fill(rect, radius, *color),
            Self::Linear(gradient) => scene.commands.push(DrawCommand::Gradient {
                rect,
                radius,
                gradient: Arc::clone(gradient),
            }),
            Self::Radial(gradient) => scene.commands.push(DrawCommand::RadialGradient {
                rect,
                radius,
                gradient: Arc::clone(gradient),
            }),
        }
    }
}
impl From<Color> for Background {
    fn from(value: Color) -> Self {
        Self::Solid(value)
    }
}
impl From<LinearGradient> for Background {
    fn from(value: LinearGradient) -> Self {
        Self::Linear(Arc::new(value))
    }
}
impl From<Arc<LinearGradient>> for Background {
    fn from(value: Arc<LinearGradient>) -> Self {
        Self::Linear(value)
    }
}
impl Element {
    /// Set a single background layer (colour or gradient), replacing any previous ones.
    pub fn background(mut self, value: impl Into<Background>) -> Self {
        self.backgrounds = None;
        match value.into() {
            Background::Solid(color) => {
                self.gradient = None;
                self.style.background = Some(color);
            }
            Background::Linear(gradient) => {
                self.style.background = None;
                self.gradient = Some(gradient);
            }
            Background::Radial(gradient) => {
                self.gradient = None;
                self.style.background = None;
                self.backgrounds = Some(vec![Background::Radial(gradient)].into());
            }
        }
        self
    }
    /// Frontmost layer first, like CSS background-image.
    pub fn backgrounds(mut self, layers: Vec<Background>) -> Self {
        self.gradient = None;
        self.backgrounds = Some(layers.into());
        self
    }
}
