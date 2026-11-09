//! Carousel.
use super::{css, events::OnChange, icons::Icon, props::Number};
use crate::ui::{
    animation::Effect,
    element::{Element, Length},
    template::{Component, Slots, TemplateContext},
};

/// Slides that move sideways; each child is one slide and the application keeps `value`.
///
/// ```ignore
/// <Carousel value={self.slide} onchange={Msg::Slide} per_view={3} loop_around>
///     <Card title="One" />
///     <Card title="Two" />
/// </Carousel>
/// ```
///
/// Slides may hold text, images and buttons; inputs, selects and scrolling areas are not
/// supported inside a moving track.
#[derive(Default)]
pub struct Carousel {
    /// Index of the first visible slide.
    pub value: usize,
    /// Sent with the index to show.
    pub onchange: OnChange<usize>,
    /// Number of slides visible at once, 1 by default.
    pub per_view: Number,
    /// The last slide leads back to the first.
    pub loop_around: bool,
    /// Hides the arrows.
    pub no_arrows: bool,
    /// Hides the dots.
    pub no_dots: bool,
}
impl<R> Component<R> for Carousel {
    fn render(self, cx: &TemplateContext<'_, R>, mut slots: Slots) -> Element {
        let slides = slots.take("");
        let count = slides.len().max(1);
        let per_view = (self.per_view.0 as usize).clamp(1, count);
        let last = count - per_view;
        let value = self.value.min(last);
        let width = Length::Percent(100. / per_view as f32);
        let shift = -(value as f32) * 100. / per_view as f32;
        let track = Effect::new().translate_percent(shift, 0.);
        let previous = match value {
            0 if self.loop_around => Some(last),
            0 => None,
            other => Some(other - 1),
        };
        let next = match value {
            other if other < last => Some(other + 1),
            _ if self.loop_around => Some(0),
            _ => None,
        };
        let change = self.onchange;
        let (arrows, dots) = (!self.no_arrows && last > 0, !self.no_dots && last > 0);
        kit_ui!(cx;
            <div class="k-carousel" role="group" aria-label="Carousel">
                <div class="k-car-viewport">
                    <div class="k-car-track" style:effect={track}>
                        {#each slides.into_iter().enumerate() as slide (slide.0)}
                            <div class="k-slide" style:width={width}>{@element slide.1}</div>
                        {/each}
                    </div>
                </div>
                {#if arrows}
                    <button class="k-car-arrow k-car-prev" aria-label="Previous" disabled={previous.is_none()}
                        onclick={change.action(previous.unwrap_or(value))}><Icon name="chevron-left" size={18} /></button>
                    <button class="k-car-arrow k-car-next" aria-label="Next" disabled={next.is_none()}
                        onclick={change.action(next.unwrap_or(value))}><Icon name="chevron-right" size={18} /></button>
                {/if}
                {#if dots}
                    <div class="k-car-dots">
                        {#each 0..=last as index (index)}
                            <button class="k-car-dot" class:k-car-dot-active={index == value}
                                aria-label="Go to slide" onclick={change.action(index)}><div></div></button>
                        {/each}
                    </div>
                {/if}
            </div>
        )
    }
}
