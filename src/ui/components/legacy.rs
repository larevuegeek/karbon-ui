//! Legacy helpers kept for backwards compatibility; the current composition API
//! lives in [`crate::ui::view`].
use super::*;

/// Legacy helper: a [`Text`] element with the given id. Prefer the
/// [`ui!`](crate::ui!) macro or [`crate::ui::view`] builders.
pub fn text(id: u64, value: impl Into<Arc<str>>) -> Element {
    Element::custom(id, Text::new(value))
}
/// Legacy helper: an auto-width [`Button`] element with the given id, emitting a
/// clone of `message` when activated. Prefer the [`ui!`](crate::ui!) macro or
/// [`crate::ui::view`] builders.
pub fn button<M: Clone + 'static>(id: u64, label: impl Into<Arc<str>>, message: M) -> Element {
    Element::custom(id, Button::new(label, message)).width(Length::Auto)
}
