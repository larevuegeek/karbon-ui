//! Fonds et ombres dont les couleurs lisent des propriétés personnalisées : compilés par
//! `css_file!`, évalués à la résolution des styles dans la portée de l'élément (variables
//! héritées des ancêtres, `style:--h={…}` du template, puis variables de l'application).
use super::ColorExpression;
use crate::ui::{
    rendering::{
        brush::{Background, GradientStop, LinearGradient, RadialExtent, RadialGradient},
        component_shadow::{BoxShadow, Shadows},
    },
    styling::CssVariables,
    theme::Color,
};

/// Gradient stop whose colour is evaluated while styles resolve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StopExpression {
    /// Position in `0.0..=1.0`, in non-decreasing order along the gradient.
    pub position: f32,
    /// Colour at this position.
    pub color: ColorExpression,
}

/// Background layer evaluated while styles resolve, built by
/// [`css_file!`](crate::css_file) when a gradient reads custom properties
/// (`radial-gradient(oklch(0.8 0.13 var(--h) / 0.4), transparent)`).
///
/// It follows the element's scope like a solid colour: an ancestor's or the template's
/// `style:--h={…}` retints it without rebuilding the view's code.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BackgroundExpression {
    /// Solid colour layer.
    Solid(ColorExpression),
    /// `linear-gradient()`.
    Linear {
        /// Direction in degrees, clockwise from left-to-right.
        angle: f32,
        /// Direction toward a corner (`to bottom right` is `[1, 1]`), overriding `angle`.
        corner: Option<[i8; 2]>,
        /// Colour stops (2 to 64).
        stops: &'static [StopExpression],
    },
    /// `radial-gradient()`.
    Radial {
        /// Centre as `[x, y]` fractions of the painted rectangle.
        center: [f32; 2],
        /// Radii, read according to `extent`.
        radius: [f32; 2],
        /// Sizing rule of the ending shape.
        extent: RadialExtent,
        /// Circle rather than ellipse (for the keyword extents).
        circle: bool,
        /// Colour stops (2 to 64).
        stops: &'static [StopExpression],
    },
}

impl BackgroundExpression {
    /// The layer with its colours evaluated; `current` is the element's text colour.
    pub fn evaluate(&self, current: Color, variables: Option<&CssVariables>) -> Background {
        let stops = |stops: &[StopExpression]| -> std::sync::Arc<[GradientStop]> {
            stops
                .iter()
                .map(|s| GradientStop {
                    position: s.position,
                    color: s.color.evaluate(current, variables),
                })
                .collect()
        };
        match *self {
            Self::Solid(color) => Background::Solid(color.evaluate(current, variables)),
            Self::Linear {
                angle,
                corner,
                stops: list,
            } => LinearGradient {
                angle,
                stops: stops(list),
                corner,
            }
            .into(),
            Self::Radial {
                center,
                radius,
                extent,
                circle,
                stops: list,
            } => RadialGradient {
                center,
                radius,
                stops: stops(list),
                extent,
                circle,
            }
            .into(),
        }
    }
    /// `true` when the colours are valid and the geometry can be painted.
    pub fn valid(&self) -> bool {
        let stops = |s: &[StopExpression]| {
            (2..=64).contains(&s.len())
                && s.iter()
                    .all(|s| s.color.valid() && (0. ..=1.).contains(&s.position))
                && s.windows(2).all(|p| p[0].position <= p[1].position)
        };
        match self {
            Self::Solid(color) => color.valid(),
            Self::Linear {
                angle, stops: s, ..
            } => angle.is_finite() && stops(s),
            Self::Radial {
                center,
                radius,
                stops: s,
                ..
            } => {
                center.iter().all(|n| n.is_finite())
                    && radius.iter().all(|n| n.is_finite() && *n > 0.)
                    && stops(s)
            }
        }
    }
}

/// One `box-shadow` layer whose colour is evaluated while styles resolve. Lengths are in
/// logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShadowExpression {
    /// Horizontal offset.
    pub x: f32,
    /// Vertical offset.
    pub y: f32,
    /// Blur radius (non-negative).
    pub blur: f32,
    /// Growth (positive) or shrink (negative) of the shadow shape.
    pub spread: f32,
    /// Shadow colour.
    pub color: ColorExpression,
    /// Draw inside the border box.
    pub inset: bool,
}

/// `box-shadow` list whose colours read custom properties, with the template scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShadowExpressions {
    /// Layers, frontmost first (at most 16).
    pub values: &'static [ShadowExpression],
    /// Multiplier for all lengths (layout scale).
    pub scale: f32,
}

impl ShadowExpressions {
    /// Layers at scale `1.0`; the template applies its own scale.
    pub const fn new(values: &'static [ShadowExpression]) -> Self {
        Self { values, scale: 1. }
    }
    /// The shadow list with its colours evaluated.
    pub fn evaluate(&self, current: Color, variables: Option<&CssVariables>) -> Shadows {
        let mut shadows = Shadows::shared(
            self.values
                .iter()
                .map(|s| BoxShadow {
                    x: s.x,
                    y: s.y,
                    blur: s.blur,
                    spread: s.spread,
                    color: s.color.evaluate(current, variables),
                    inset: s.inset,
                })
                .collect(),
        );
        shadows.scale = self.scale;
        shadows
    }
    /// `true` when the list can be rendered once evaluated.
    pub fn valid(&self) -> bool {
        self.scale.is_finite()
            && self.scale > 0.
            && self.values.len() <= 16
            && self.values.iter().all(|s| {
                [s.x, s.y, s.blur, s.spread].iter().all(|n| n.is_finite())
                    && s.blur >= 0.
                    && s.blur * self.scale <= 512.
                    && s.color.valid()
            })
    }
}
