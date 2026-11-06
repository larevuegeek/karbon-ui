use super::{Animation, Easing, Effect};
/// Timing used to interpolate between effect states (see [`Element::transition`]).
/// Defaults to 180 ms, no delay, [`Easing::EaseOut`].
///
/// [`Element::transition`]: crate::ui::element::Element::transition
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transition {
    pub(crate) duration: u32,
    pub(crate) delay: u32,
    pub(crate) easing: Easing,
    /// Interpole l'opacité et la transformation (vrai par défaut).
    pub(crate) effects: bool,
    /// Interpole les couleurs CSS (texte, fond, bordure).
    pub(crate) colors: bool,
    /// Interpole les dimensions CSS en pixels (`width`, `height`) : relance le layout
    /// à chaque image.
    pub(crate) sizes: bool,
}
impl Default for Transition {
    fn default() -> Self {
        Self::new()
    }
}
impl Transition {
    /// Returns the default timing: 180 ms, no delay, [`Easing::EaseOut`].
    pub const fn new() -> Self {
        Self {
            duration: 180,
            delay: 0,
            easing: Easing::EaseOut,
            effects: true,
            colors: false,
            sizes: false,
        }
    }
    /// Chooses which values are interpolated: `effects` (opacity and transform, the
    /// default) and/or `colors` (CSS text, background and border colours).
    pub const fn properties(mut self, effects: bool, colors: bool) -> Self {
        self.effects = effects;
        self.colors = colors;
        self
    }
    /// `true` when CSS colours are interpolated.
    pub const fn animates_colors(self) -> bool {
        self.colors
    }
    /// Also interpolates the CSS `width` and `height` of the element when both the old
    /// and the new value are pixel lengths (`transition: width 200ms`). Each frame of
    /// the transition lays the tree out again; other lengths change immediately.
    pub const fn sizes(mut self, sizes: bool) -> Self {
        self.sizes = sizes;
        self
    }
    /// `true` when CSS `width` and `height` are interpolated.
    pub const fn animates_sizes(self) -> bool {
        self.sizes
    }
    /// Sets the duration in milliseconds.
    pub const fn duration(mut self, ms: u32) -> Self {
        self.duration = ms;
        self
    }
    /// Sets the delay before the interpolation starts, in milliseconds.
    pub const fn delay(mut self, ms: u32) -> Self {
        self.delay = ms;
        self
    }
    /// Sets the easing curve.
    pub const fn easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }
}
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct ReplacedProperties {
    pub opacity: bool,
    pub transform: bool,
}
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct Motion {
    pub base: Effect,
    pub hover: Option<Effect>,
    pub pressed: Option<Effect>,
    pub hover_replace: Option<ReplacedProperties>,
    pub pressed_replace: Option<ReplacedProperties>,
    pub origin: (f32, f32),
    pub transition: Option<Transition>,
    pub animation: Option<Animation>,
    /// Animation de sortie jouée par un fantôme quand l'élément disparaît.
    pub exit: Option<Animation>,
    /// Hauteur animée entre 0 et la hauteur naturelle quand l'élément est affiché ou
    /// masqué (`display: none`) : voir [`Element::reveal`](crate::ui::element::Element::reveal).
    pub reveal: Option<Transition>,
    /// Effet d'une règle d'état CSS active (`.groupe:hover .x`), posé par la résolution
    /// des styles au-dessus de `base`.
    pub state: Option<(Effect, ReplacedProperties)>,
}
impl Default for Motion {
    fn default() -> Self {
        Self {
            base: Effect::NONE,
            hover: None,
            pressed: None,
            hover_replace: None,
            pressed_replace: None,
            origin: (0.5, 0.5),
            transition: None,
            animation: None,
            exit: None,
            reveal: None,
            state: None,
        }
    }
}
impl Motion {
    pub fn target(self, hot: bool, down: bool) -> Effect {
        fn apply(mut base: Effect, value: Effect, replace: Option<ReplacedProperties>) -> Effect {
            if let Some(p) = replace {
                if p.opacity {
                    base.opacity = value.opacity;
                }
                if p.transform {
                    base.x = value.x;
                    base.y = value.y;
                    base.rotation = value.rotation;
                    base.scale_x = value.scale_x;
                    base.scale_y = value.scale_y;
                    base.x_percent = value.x_percent;
                    base.y_percent = value.y_percent;
                }
                base
            } else {
                base.combine(value)
            }
        }
        let hover = if hot { self.hover } else { None };
        // L'effet d'une règle d'état CSS remplace d'abord les parties déclarées de la base.
        let base = self.state.map_or(self.base, |(effect, replace)| {
            apply(self.base, effect, Some(replace))
        });
        if down && let Some(pressed) = self.pressed {
            let base = if self.pressed_replace.is_some() {
                hover.map_or(base, |h| apply(base, h, self.hover_replace))
            } else {
                base
            };
            apply(base, pressed, self.pressed_replace)
        } else {
            hover.map_or(base, |h| apply(base, h, self.hover_replace))
        }
    }
    pub fn spatial(self) -> bool {
        self.state.is_some_and(|(e, _)| e.spatial())
            || self.base.spatial()
            || self.hover.is_some_and(Effect::spatial)
            || self.pressed.is_some_and(Effect::spatial)
            || self.animation.is_some_and(Animation::spatial)
    }
    /// `true` quand seules une animation de sortie et/ou une révélation de hauteur sont
    /// déclarées : aucune piste d'effet à suivre.
    pub fn exit_only(self) -> bool {
        self == Self {
            exit: self.exit,
            reveal: self.reveal,
            ..Self::default()
        }
    }
    pub fn valid(self) -> bool {
        self.exit.is_none_or(Animation::valid)
            && self.base.valid()
            && self.hover.is_none_or(Effect::valid)
            && self.pressed.is_none_or(Effect::valid)
            && [self.origin.0, self.origin.1]
                .iter()
                .all(|v| v.is_finite() && (0. ..=1.).contains(v))
            && self.transition.is_none_or(|t| t.easing.valid())
            && self.reveal.is_none_or(|t| t.easing.valid())
            && self.animation.is_none_or(Animation::valid)
    }
}
