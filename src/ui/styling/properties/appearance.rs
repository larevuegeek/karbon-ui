//! Valeurs d'apparence d'un élément : ajustement d'image, pièces de contrôle dessinées en
//! CSS et effet d'une règle d'état.
use crate::ui::theme::Color;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
/// How an image fills its box (CSS `object-fit`).
pub enum ImageFit {
    /// Stretch to fill the box, ignoring the aspect ratio (default).
    #[default]
    Fill,
    /// Scale to fit inside the box, keeping the aspect ratio.
    Contain,
    /// Scale to cover the box, keeping the aspect ratio and cropping overflow.
    Cover,
    /// Keep the intrinsic size.
    None,
    /// Like [`Contain`](Self::Contain), but never scale up.
    ScaleDown,
}
/// Box of a native control part drawn from CSS (slider thumb or track).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ControlPart {
    /// Fill colour.
    pub background: Option<Color>,
    /// Width in pixels.
    pub width: Option<f32>,
    /// Height in pixels.
    pub height: Option<f32>,
    /// Corner radius in pixels.
    pub radius: Option<f32>,
    /// Border width in pixels.
    pub border_width: Option<f32>,
    /// Border colour.
    pub border: Option<Color>,
    /// Shadows (only offsets and spreads are drawn).
    pub shadows: Option<crate::ui::rendering::component_shadow::Shadows>,
}
impl ControlPart {
    /// Layers `self` over `base`: each property set on `self` wins.
    pub fn over(self, base: Self) -> Self {
        Self {
            background: self.background.or(base.background),
            width: self.width.or(base.width),
            height: self.height.or(base.height),
            radius: self.radius.or(base.radius),
            border_width: self.border_width.or(base.border_width),
            border: self.border.or(base.border),
            shadows: self.shadows.or(base.shadows),
        }
    }
    pub(super) fn valid(&self) -> bool {
        [self.width, self.height, self.radius, self.border_width]
            .into_iter()
            .flatten()
            .all(|n| n.is_finite() && n >= 0.)
            && self.shadows.as_ref().is_none_or(|s| s.valid())
    }
}
/// Visual effect of a matching state rule: which parts of [`effect`](Self::effect)
/// replace the element's base opacity and transform.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StateEffect {
    /// Opacity and transform values.
    pub effect: crate::ui::animation::Effect,
    /// The rule declares `opacity`.
    pub opacity: bool,
    /// The rule declares `transform`.
    pub transform: bool,
}
impl StateEffect {
    /// Layers `self` over `base`: declared parts of `self` win.
    pub fn over(self, base: Self) -> Self {
        let mut effect = base.effect;
        if self.opacity {
            effect.opacity = self.effect.opacity;
        }
        if self.transform {
            let e = self.effect;
            (effect.x, effect.y, effect.rotation) = (e.x, e.y, e.rotation);
            (effect.scale_x, effect.scale_y) = (e.scale_x, e.scale_y);
            (effect.x_percent, effect.y_percent) = (e.x_percent, e.y_percent);
        }
        Self {
            effect,
            opacity: self.opacity || base.opacity,
            transform: self.transform || base.transform,
        }
    }
}
