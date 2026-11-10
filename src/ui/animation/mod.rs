//! Animations and transitions: monotonic time, visual effects and declarative entry animations.
//!
//! Effects are purely visual (opacity and 2D transform): they never change layout. Builders on
//! [`Element`](crate::ui::element::Element) attach an [`Effect`], hover/pressed variants, a [`Transition`] or an [`Animation`].
//! The tree is validated before layout: invalid values (non-finite numbers, opacity outside
//! `0..=1`, malformed keyframes) are rejected with an error.
mod easing;
mod effect;
mod keyframes;
mod overrides;
mod play;
mod spec;
pub(crate) mod timeline;
mod timing;
pub use easing::Easing;
pub use effect::Effect;
pub use keyframes::Keyframe;
pub use overrides::AnimationOverride;
pub use play::{Animation, FillMode};
pub(crate) use spec::Motion;
pub(crate) use spec::ReplacedProperties;
pub use spec::Transition;
pub(crate) use timing::Due;
mod declaration;
