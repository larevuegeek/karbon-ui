pub(super) use super::super::{
    element::*,
    modal::Modal,
    popover::{MenuItem, Popover},
    widgets::{Button, Text, TextInput},
};
pub(super) use std::sync::Arc;

/// Natural size of the content.
pub const AUTO: Length = Length::Auto;
/// Take all the remaining space (`fraction(1.0)`).
pub const FILL: Length = Length::Fill(1.);
/// Percentage of the parent, from 0 to 100.
pub const fn percent(value: f32) -> Length {
    Length::Percent(value)
}
/// Share of the remaining space, like a CSS `fr` fraction.
pub const fn fraction(value: f32) -> Length {
    Length::Fill(value)
}

/// Horizontal container.
pub fn row(id: impl Into<ElementId>, children: impl IntoIterator<Item = Element>) -> Element {
    Element::row(id.into().0, children.into_iter().collect())
}
/// Vertical container.
pub fn column(id: impl Into<ElementId>, children: impl IntoIterator<Item = Element>) -> Element {
    Element::column(id.into().0, children.into_iter().collect())
}
/// Grid with `columns` equal columns.
pub fn grid(
    id: impl Into<ElementId>,
    columns: usize,
    children: impl IntoIterator<Item = Element>,
) -> Element {
    Element::grid(id.into().0, columns, children.into_iter().collect())
}
/// Root layer stack: `page`, then dialogs and popovers drawn above it.
pub fn layers(
    id: impl Into<ElementId>,
    page: Element,
    overlays: impl IntoIterator<Item = Element>,
) -> Element {
    Element::layers(id.into().0, page, overlays.into_iter().collect())
}
/// Scroll container showing `content` through a viewport along `axis`.
pub fn scroll(id: impl Into<ElementId>, axis: Axis, content: Element) -> Element {
    Element::scroll(id.into().0, axis, content)
}
/// Two-pane container; see [`Split`].
pub fn split(id: impl Into<ElementId>, options: Split, first: Element, second: Element) -> Element {
    Element::split(id.into().0, options, first, second)
}
/// Modal dialog wrapping `content`.
pub fn modal(id: impl Into<ElementId>, options: Modal, content: Element) -> Element {
    Element::modal(id.into().0, options, content)
}
/// Popover wrapping `content`.
pub fn popover(id: impl Into<ElementId>, options: Popover, content: Element) -> Element {
    Element::popover(id.into().0, options, content)
}
/// Element driven by a custom [`Widget`].
pub fn widget(id: impl Into<ElementId>, widget: impl Widget + 'static) -> Element {
    Element::custom(id.into().0, widget)
}
/// Static text.
pub fn text(id: impl Into<ElementId>, value: impl Into<Arc<str>>) -> Element {
    widget(id, Text::new(value))
}
/// Button emitting a clone of `message` when activated.
pub fn button<M: Clone + 'static>(
    id: impl Into<ElementId>,
    label: impl Into<Arc<str>>,
    message: M,
) -> Element {
    widget(id, Button::new(label, message)).width(AUTO)
}
/// Text field: each edit sends the new text through `on_change`.
pub fn input<M: 'static>(
    id: impl Into<ElementId>,
    value: impl Into<Arc<str>>,
    on_change: impl Fn(String) -> M + 'static,
) -> Element {
    widget(id, TextInput::new(value, on_change))
}
/// Multiline text field (`<textarea>`): each edit sends the new text through
/// `on_change`; see [`TextInput::multiline`].
pub fn textarea<M: 'static>(
    id: impl Into<ElementId>,
    value: impl Into<Arc<str>>,
    on_change: impl Fn(String) -> M + 'static,
) -> Element {
    widget(id, TextInput::new(value, on_change).multiline())
}
/// Menu item emitting a clone of `message` when chosen.
pub fn menu_item<M: Clone + 'static>(
    id: impl Into<ElementId>,
    label: impl Into<Arc<str>>,
    message: M,
) -> Element {
    widget(id, MenuItem::new(label, message))
}
/// Empty space; set `.height(18)` or `.width(20)` depending on its parent.
pub fn space(id: impl Into<ElementId>) -> Element {
    column(id, [])
}

/// Dress and configure a widget before giving it its identity.
pub trait WidgetExt: Widget + Sized + 'static {
    /// Wrap the widget in an element with the given `id`.
    fn element(self, id: impl Into<ElementId>) -> Element {
        widget(id, self)
    }
}
impl<T: Widget + Sized + 'static> WidgetExt for T {}
impl Element {
    /// Append a child.
    pub fn child(mut self, child: Element) -> Self {
        self.children.push(child);
        self
    }
    /// Build the child only when the condition is true.
    pub fn child_if(self, condition: bool, child: impl FnOnce() -> Element) -> Self {
        if condition { self.child(child()) } else { self }
    }
    /// Container decoration painted before its children. Does not replace its layout.
    pub fn painter(mut self, painter: impl Widget + 'static) -> Self {
        self.widget = Some(Box::new(painter));
        self
    }
}
