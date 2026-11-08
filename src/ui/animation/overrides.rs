//! Réglages d'animation d'une règle d'état ou de media (`.piste.inverse {
//! animation-direction: reverse }`, `.pause .piste { animation-play-state: paused }`) :
//! ils remplacent ceux de l'animation de l'élément tant que la règle s'applique.
use super::Animation;

/// Animation longhands declared by a state or media rule, such as
/// `.track.reverse { animation-direction: reverse }` or
/// `.paused .track { animation-play-state: paused }`: while the rule matches, they replace
/// the matching settings of the element's animation (set by its classes).
///
/// Only `animation-play-state` keeps the animation running from where it was; the other
/// settings restart it, like a new animation. On a list of animations (`animation: a, b`)
/// the delay and the play state apply to the whole list, the duration and the direction
/// to none of its members.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AnimationOverride {
    /// `animation-delay` in milliseconds; negative starts the animation already advanced.
    pub delay: Option<i32>,
    /// `animation-duration` in milliseconds.
    pub duration: Option<u32>,
    /// `animation-direction` as `(alternate, reverse)`: `reverse` is `(false, true)`,
    /// `alternate-reverse` is `(true, true)`.
    pub direction: Option<(bool, bool)>,
    /// `animation-play-state`: `Some(true)` for `paused`, `Some(false)` for `running`.
    pub paused: Option<bool>,
    /// `animation: none` (or `animation-name: none`): no animation while the rule
    /// matches, as in `@media (prefers-reduced-motion: reduce) { .x { animation: none } }`.
    pub none: bool,
}
impl AnimationOverride {
    /// `true` when no setting is declared.
    pub fn is_empty(self) -> bool {
        self == Self::default()
    }
    /// Layers `self` over `base`: each setting declared by `self` wins.
    pub fn over(self, base: Self) -> Self {
        Self {
            delay: self.delay.or(base.delay),
            duration: self.duration.or(base.duration),
            direction: self.direction.or(base.direction),
            paused: self.paused.or(base.paused),
            none: self.none || base.none,
        }
    }
    /// `animation` with these settings (`None` for `animation: none`).
    pub(crate) fn apply(self, mut animation: Animation) -> Option<Animation> {
        if self.none {
            return None;
        }
        if let Some(ms) = self.delay {
            animation = animation.with_delay(ms as i64);
        }
        if animation.group.is_empty() {
            if let Some(ms) = self.duration {
                animation = animation.duration(ms);
            }
            if let Some((alternate, reverse)) = self.direction {
                animation.alternate = alternate;
                animation.reverse = reverse;
            }
        }
        Some(animation.with_play_state(self.paused))
    }
}
