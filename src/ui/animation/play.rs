//! Animation déclarative jouée sur un élément (modèle des animations CSS) : étapes,
//! minutage, direction, remplissage, pause et listes d'animations.
use super::{Easing, Effect, Keyframe, Transition, keyframes};

/// Declarative effect animation played on an element (see [`Element::animate`]), modelled on
/// CSS animations. Defaults: 350 ms, [`Easing::EaseOut`], one iteration, [`FillMode::Both`].
///
/// [`Element::animate`]: crate::ui::element::Element::animate
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Animation {
    pub(crate) from: Effect,
    pub(crate) to: Effect,
    pub(crate) timing: Transition,
    pub(crate) iterations: Option<u32>,
    pub(crate) alternate: bool,
    /// Itérations jouées à rebours (`reverse`, `alternate-reverse` avec `alternate`).
    pub(crate) reverse: bool,
    /// Avance au démarrage en millisecondes (`animation-delay` négatif).
    pub(crate) advance: u32,
    /// Horloge figée (`animation-play-state: paused`).
    pub(crate) paused: bool,
    pub(crate) key: u64,
    pub(crate) frames: &'static [Keyframe],
    /// Animations jouées ensemble (liste CSS `animation: a 1s, b 2s`) ; vide pour une
    /// animation simple.
    pub(crate) group: &'static [Animation],
    /// Propriétés animées (bits de `keyframes::property`).
    pub(crate) properties: u8,
    pub(crate) distance_scale: f32,
    pub(crate) fill: FillMode,
}
/// Which effect applies outside the active period of an [`Animation`], like CSS
/// `animation-fill-mode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillMode {
    /// No animation effect during the delay or after the end.
    None,
    /// Keeps the final effect after the animation ends.
    Forwards,
    /// Applies the first effect during the delay.
    Backwards,
    /// Combines [`Forwards`](Self::Forwards) and [`Backwards`](Self::Backwards).
    Both,
}
impl Animation {
    /// Animates from `from` to `to` with the default timing.
    pub const fn from_to(from: Effect, to: Effect) -> Self {
        Self {
            from,
            to,
            timing: Transition::new().duration(350),
            iterations: Some(1),
            alternate: false,
            reverse: false,
            advance: 0,
            paused: false,
            key: 0,
            frames: &[],
            group: &[],
            properties: keyframes::property::EFFECTS,
            distance_scale: 1.,
            fill: FillMode::Both,
        }
    }
    /// Builds an animation from keyframes sorted by offset (at most 128). A keyframe may
    /// declare some properties only: each property interpolates between the keyframes that
    /// declare it, and takes the element's own value at `0.0` or `1.0` when no keyframe
    /// declares it there, like CSS `@keyframes`. Two keyframes may share an offset if they
    /// declare different properties. No per-frame allocation.
    pub const fn keyframes(frames: &'static [Keyframe]) -> Self {
        let mut value = Self::from_to(Effect::NONE, Effect::NONE);
        value.frames = frames;
        value.properties = keyframes::declared(frames);
        value
    }
    /// Plays several animations at once, like the CSS list `animation: a 1s, b 2s`. Each
    /// member keeps its own timing; when two members animate the same property, the later
    /// one in the list wins. The group's [`delay`](Self::delay) postpones every member,
    /// its [`key`](Self::key) restarts them all; the group's other timing settings are
    /// ignored. Members cannot be groups themselves (1 to 16 members).
    pub const fn group(members: &'static [Animation]) -> Self {
        let mut value = Self::from_to(Effect::NONE, Effect::NONE);
        value.group = members;
        value.properties = 0;
        let mut i = 0;
        while i < members.len() {
            value.properties |= members[i].properties;
            i += 1;
        }
        value
    }
    /// Fades from transparent to the resting effect.
    pub const fn fade_in() -> Self {
        Self::from_to(Effect::new().opacity(0.), Effect::NONE)
    }
    /// Fades from the resting effect to transparent.
    pub const fn fade_out() -> Self {
        Self::from_to(Effect::NONE, Effect::new().opacity(0.))
    }
    /// Fades in while moving up from `distance` logical pixels below.
    pub const fn slide_up(distance: f32) -> Self {
        Self::from_to(
            Effect::new().translate(0., distance).opacity(0.),
            Effect::NONE,
        )
    }
    /// Fades in while moving down from `distance` logical pixels above.
    pub const fn slide_down(distance: f32) -> Self {
        Self::slide_up(-distance)
    }
    /// Fades in while moving left from `distance` logical pixels to the right.
    pub const fn slide_left(distance: f32) -> Self {
        Self::from_to(
            Effect::new().translate(distance, 0.).opacity(0.),
            Effect::NONE,
        )
    }
    /// Fades in while moving right from `distance` logical pixels to the left.
    pub const fn slide_right(distance: f32) -> Self {
        Self::slide_left(-distance)
    }
    /// Rotates from the resting effect to `degrees`.
    pub const fn rotate(degrees: f32) -> Self {
        Self::from_to(Effect::NONE, Effect::new().rotate(degrees))
    }
    /// Fades in while growing from 90% scale.
    pub const fn scale_in() -> Self {
        Self::from_to(Effect::new().scale(0.9).opacity(0.), Effect::NONE)
    }
    /// Sets the duration of one iteration in milliseconds.
    pub const fn duration(mut self, ms: u32) -> Self {
        self.timing.duration = ms;
        self
    }
    /// Sets the delay before the first iteration, in milliseconds.
    pub const fn delay(mut self, ms: u32) -> Self {
        self.timing.delay = ms;
        self
    }
    /// Starts the animation `ms` milliseconds into its timeline, like a negative CSS
    /// `animation-delay`: `advance(500)` on a 1 s animation shows its middle at once.
    pub const fn advance(mut self, ms: u32) -> Self {
        self.advance = ms;
        self
    }
    /// Sets the easing of each iteration (and of keyframe segments without their own).
    pub const fn easing(mut self, easing: Easing) -> Self {
        self.timing.easing = easing;
        self
    }
    /// Total number of iterations, like CSS `animation-iteration-count`; must be at least 1.
    pub const fn repeat(mut self, count: u32) -> Self {
        self.iterations = Some(count);
        self
    }
    /// Repeats the animation indefinitely.
    pub const fn forever(mut self) -> Self {
        self.iterations = None;
        self
    }
    /// Plays every other iteration in reverse, like CSS `animation-direction: alternate`.
    pub const fn alternate(mut self) -> Self {
        self.alternate = true;
        self
    }
    /// Plays the iterations backwards, like CSS `animation-direction: reverse`; with
    /// [`alternate`](Self::alternate), like `alternate-reverse` (the first iteration runs
    /// backwards, the next forwards…).
    pub const fn reverse(mut self) -> Self {
        self.reverse = true;
        self
    }
    /// Freezes the animation at its current point, like CSS
    /// `animation-play-state: paused`. On an element with a stable identity, changing only
    /// this flag pauses or resumes the animation without restarting it.
    pub const fn paused(mut self, paused: bool) -> Self {
        self.paused = paused;
        self
    }
    /// A new key restarts the animation on an element with a stable identity.
    pub const fn key(mut self, key: u64) -> Self {
        self.key = key;
        self
    }
    /// Sets the [`FillMode`].
    pub const fn fill_mode(mut self, value: FillMode) -> Self {
        self.fill = value;
        self
    }
}
