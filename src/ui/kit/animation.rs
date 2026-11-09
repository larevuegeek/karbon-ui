//! Animation of any content by attributes.
use super::{css, props::Number};
use crate::ui::{
    animation::{Animation, Easing, Effect, Keyframe},
    element::Element,
    template::{Component, Slots, TemplateContext},
};

const PULSE: &[Keyframe] = &[
    Keyframe::at(0.).transform(Effect::new().scale(1.)),
    Keyframe::at(0.5).transform(Effect::new().scale(1.05)),
    Keyframe::at(1.).transform(Effect::new().scale(1.)),
];
const FLOAT: &[Keyframe] = &[
    Keyframe::at(0.).transform(Effect::new().translate(0., 0.)),
    Keyframe::at(0.5).transform(Effect::new().translate(0., -10.)),
    Keyframe::at(1.).transform(Effect::new().translate(0., 0.)),
];
const BOUNCE: &[Keyframe] = &[
    Keyframe::at(0.).transform(Effect::new().translate(0., -24.)),
    Keyframe::at(0.45).transform(Effect::new().translate(0., 0.)),
    Keyframe::at(0.65).transform(Effect::new().translate(0., -8.)),
    Keyframe::at(0.85).transform(Effect::new().translate(0., 0.)),
    Keyframe::at(1.).transform(Effect::new().translate(0., 0.)),
];
const SHAKE: &[Keyframe] = &[
    Keyframe::at(0.).transform(Effect::new().translate(0., 0.)),
    Keyframe::at(0.2).transform(Effect::new().translate(-6., 0.)),
    Keyframe::at(0.4).transform(Effect::new().translate(6., 0.)),
    Keyframe::at(0.6).transform(Effect::new().translate(-4., 0.)),
    Keyframe::at(0.8).transform(Effect::new().translate(4., 0.)),
    Keyframe::at(1.).transform(Effect::new().translate(0., 0.)),
];

/// Animates its content: `<Animate up delay={80}>…</Animate>`.
///
/// Entrances (`fade`, `up`, `down`, `left`, `right`, `zoom`, `bounce`, `shake`) may hold any
/// content. Loops (`pulse`, `float`, `spin`) keep moving, so they may not hold inputs,
/// selects or scrolling areas.
#[derive(Default)]
pub struct Animate {
    /// Fades in.
    pub fade: bool,
    /// Rises into place.
    pub up: bool,
    /// Falls into place.
    pub down: bool,
    /// Slides in from the right.
    pub left: bool,
    /// Slides in from the left.
    pub right: bool,
    /// Grows into place.
    pub zoom: bool,
    /// Drops and bounces.
    pub bounce: bool,
    /// Shakes sideways.
    pub shake: bool,
    /// Breathes, forever.
    pub pulse: bool,
    /// Hovers up and down, forever.
    pub float: bool,
    /// Turns, forever.
    pub spin: bool,
    /// Wait before starting, in milliseconds.
    pub delay: Number,
    /// Length in milliseconds.
    pub duration: Option<f32>,
    /// A new value plays the animation again.
    pub replay: Option<u64>,
}
impl Animate {
    fn animation(&self) -> Animation {
        let spring = Easing::CubicBezier(0.22, 1., 0.36, 1.);
        let time = |default: u32| self.duration.map_or(default, |ms| ms.max(1.) as u32);
        let base = if self.pulse {
            Animation::keyframes(PULSE)
                .duration(time(1800))
                .easing(Easing::EaseInOut)
                .forever()
        } else if self.float {
            Animation::keyframes(FLOAT)
                .duration(time(2800))
                .easing(Easing::EaseInOut)
                .forever()
        } else if self.spin {
            Animation::rotate(360.)
                .duration(time(1000))
                .easing(Easing::Linear)
                .forever()
        } else if self.bounce {
            Animation::keyframes(BOUNCE).duration(time(700))
        } else if self.shake {
            Animation::keyframes(SHAKE).duration(time(450))
        } else if self.up {
            Animation::slide_up(18.).duration(time(420)).easing(spring)
        } else if self.down {
            Animation::slide_down(18.)
                .duration(time(420))
                .easing(spring)
        } else if self.left {
            Animation::slide_left(24.)
                .duration(time(420))
                .easing(spring)
        } else if self.right {
            Animation::slide_right(24.)
                .duration(time(420))
                .easing(spring)
        } else if self.zoom {
            Animation::scale_in().duration(time(320)).easing(spring)
        } else {
            Animation::fade_in().duration(time(320))
        };
        base.delay(self.delay.0.max(0.) as u32)
            .key(self.replay.unwrap_or(0))
    }
}
impl<R> Component<R> for Animate {
    fn render(self, cx: &TemplateContext<'_, R>, mut slots: Slots) -> Element {
        let animation = self.animation();
        kit_ui!(cx, slots; <div class="k-flex" style:animate={animation}><slot /></div>)
    }
}
