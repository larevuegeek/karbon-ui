//! Briques d'exécution de la syntaxe HTML : texte enrichi, rôles sémantiques,
//! séparateur, groupes radio, animations `data-*` et passerelles vers l'API Rust.
use super::{Node, TemplateContext};
use crate::ui::{
    animation::Animation,
    components::RichText,
    element::*,
    scene::{DrawCommand, Scene},
};
use std::sync::Arc;

/// Builds one subtree of a template in its own stack frame.
///
/// Generated code wraps every element that has descendants in a call to this function,
/// so a page made of many `{@include}` fragments is not one huge function: without
/// optimisations every temporary of a function gets its own stack slot, and a whole
/// page in one frame overflows the 1 MiB main-thread stack of a debug build.
#[doc(hidden)]
#[inline(never)]
pub fn subtree<T>(build: impl FnOnce() -> T) -> T {
    build()
}

/// Empty rich text using the context font, at 15 logical pixels scaled by the context
/// scale; templates append the runs of `<p>`, headings and inline content to it.
pub fn rich<R>(cx: &TemplateContext<'_, R>) -> RichText {
    RichText::new().font(cx.font).size(15. * cx.scale)
}

impl Node<RichText> {
    /// Leaf node holding `text`, with one inline child per link.
    pub fn rich_text(id: ElementId, text: RichText) -> Self {
        let links = text.link_elements(id);
        let mut node = Self::widget(id, text);
        node.element.children = links;
        node
    }
}

/// Container widget exposing an accessibility role (and optional name) without
/// painting anything: `<main>`, `<nav>`, `<ul>`, `<li>`, `<section aria-label>`…
pub struct Semantic {
    role: Role,
    name: Arc<str>,
}
impl Semantic {
    /// Group with `role` and accessible `name` (may be empty).
    pub fn new(role: Role, name: impl Into<Arc<str>>) -> Self {
        Self {
            role,
            name: name.into(),
        }
    }
}
impl Widget for Semantic {
    fn measure(&self, _: SizeF, _: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
        Ok(SizeF::default())
    }
    fn paint(&self, _: &PaintContext, _: &mut Scene) {}
    fn semantics(&self) -> Semantics<'_> {
        Semantics {
            role: self.role,
            name: &self.name,
            enabled: true,
        }
    }
}

/// Widget whose accessibility role is replaced (`role="…"` in templates); everything
/// else is delegated to the wrapped widget.
pub struct WithRole {
    inner: Option<Box<dyn Widget>>,
    role: Role,
}
impl Widget for WithRole {
    fn apply_style(&mut self, style: &crate::ui::styling::ComputedStyle) -> bool {
        self.inner.as_mut().is_some_and(|w| w.apply_style(style))
    }
    fn validate(&self) -> crate::Result<()> {
        self.inner.as_ref().map_or(Ok(()), |w| w.validate())
    }
    fn measure(&self, available: SizeF, text: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
        self.inner
            .as_ref()
            .map_or(Ok(SizeF::default()), |w| w.measure(available, text))
    }
    fn paint(&self, c: &PaintContext, s: &mut Scene) {
        if let Some(inner) = &self.inner {
            inner.paint(c, s);
        }
    }
    fn action(&self) -> Option<crate::application::Action> {
        self.inner.as_ref().and_then(|w| w.action())
    }
    fn text_input(&self) -> Option<&crate::ui::text_input::TextInput> {
        self.inner.as_ref().and_then(|w| w.text_input())
    }
    fn choice(&self) -> Option<ChoiceState> {
        self.inner.as_ref().and_then(|w| w.choice())
    }
    fn slider(&self) -> Option<&crate::ui::slider::Slider> {
        self.inner.as_ref().and_then(|w| w.slider())
    }
    fn select(&self) -> Option<&crate::ui::select::Select> {
        self.inner.as_ref().and_then(|w| w.select())
    }
    fn modal(&self) -> Option<&crate::ui::modal::Modal> {
        self.inner.as_ref().and_then(|w| w.modal())
    }
    fn popover(&self) -> Option<&crate::ui::popover::Popover> {
        self.inner.as_ref().and_then(|w| w.popover())
    }
    fn focusable(&self) -> bool {
        self.inner.as_ref().is_some_and(|w| w.focusable())
    }
    fn semantics(&self) -> Semantics<'_> {
        let inner = self.inner.as_ref().map(|w| w.semantics());
        Semantics {
            role: self.role,
            name: inner.as_ref().map_or("", |s| s.name),
            enabled: inner.is_none_or(|s| s.enabled),
        }
    }
    fn inline_children(&self) -> usize {
        self.inner.as_ref().map_or(0, |w| w.inline_children())
    }
    fn inline_boxes(
        &mut self,
        area: crate::ui::scene::RectF,
        text: &mut dyn TextMeasurer,
    ) -> crate::Result<Vec<crate::ui::scene::RectF>> {
        self.inner
            .as_mut()
            .map_or(Ok(Vec::new()), |w| w.inline_boxes(area, text))
    }
    fn hit_test(&self, rect: crate::ui::scene::RectF, point: crate::ui::scene::PointF) -> bool {
        self.inner.as_ref().is_none_or(|w| w.hit_test(rect, point))
    }
}
/// Replace the accessibility role of a built element (`role="menuitem"`…).
pub fn role(mut element: Element, role: Role) -> Element {
    let inner = element.widget.take();
    element.widget = Some(Box::new(WithRole { inner, role }));
    element
}

/// Horizontal rule (`<hr>`): a one-pixel line in the theme border colour.
#[derive(Default)]
pub struct Separator {
    color: Option<crate::ui::theme::Color>,
}
impl Separator {
    /// Separator in the theme border colour.
    pub fn new() -> Self {
        Self::default()
    }
}
impl Widget for Separator {
    fn apply_style(&mut self, style: &crate::ui::styling::ComputedStyle) -> bool {
        self.color = style
            .text
            .color
            .filter(|_| style.local.border.is_none())
            .or(style.local.border);
        false
    }
    fn measure(&self, available: SizeF, _: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
        Ok(SizeF {
            width: available.width,
            height: 1.,
        })
    }
    fn paint(&self, c: &PaintContext, s: &mut Scene) {
        let y = c.rect.top + crate::ui::layout::height(c.rect) / 2.;
        s.commands.push(DrawCommand::Line {
            start: crate::ui::scene::PointF { x: c.rect.left, y },
            end: crate::ui::scene::PointF { x: c.rect.right, y },
            width: 1.,
            color: self.color.unwrap_or(c.theme.border),
        });
    }
    fn semantics(&self) -> Semantics<'_> {
        Semantics {
            role: Role::Separator,
            name: "",
            enabled: true,
        }
    }
}

/// Widget making a container the radio group of the radio buttons it holds
/// (`<input type="radio" name="…">`); `label` names the group.
pub fn radio_group(label: impl Into<Arc<str>>) -> impl Widget {
    crate::ui::components::choices::group_widget(label.into())
}

/// Delays the entry animation of each child by its index × `ms` milliseconds
/// (`data-stagger`).
pub fn stagger(children: &mut [Element], ms: u32) {
    for (index, child) in children.iter_mut().enumerate() {
        if let Some(motion) = child.motion.as_deref_mut()
            && let Some(animation) = &mut motion.animation
        {
            animation.timing.delay = animation
                .timing
                .delay
                .saturating_add(ms.saturating_mul(index as u32));
        }
    }
}

/// Stable key of a `data-replay` value: the entry animation restarts when it changes.
pub fn replay_key<T: std::hash::Hash + ?Sized>(value: &T) -> u64 {
    use std::hash::{BuildHasher, BuildHasherDefault};
    BuildHasherDefault::<std::collections::hash_map::DefaultHasher>::default().hash_one(value)
}
impl<W: Widget + 'static> Node<W> {
    /// Restarts the entry animation whenever `key` changes (`data-replay`).
    pub fn replay(mut self, key: u64) -> Self {
        if let Some(animation) = self
            .element
            .motion
            .as_deref_mut()
            .and_then(|m| m.animation.as_mut())
        {
            animation.key = key;
        }
        self
    }
    /// Text style declared on this node (size, family, weight, colour, decoration), used
    /// for inline runs styled by classes or `style="…"`.
    pub fn span_style(&self) -> crate::ui::styling::text::SpanStyle {
        crate::ui::styling::text::SpanStyle::from_typography(
            self.element
                .styles
                .as_ref()
                .map(|s| s.normal.text)
                .unwrap_or_default(),
        )
    }
}

/// Value accepted by `{@element …}`: an [`Element`] (keeps its own identity) or any
/// [`Widget`] (wrapped in a leaf with the template identity).
pub trait Embed {
    /// Converts the value into an element; `id` is the template identity.
    fn embed(self, id: ElementId) -> Element;
}
impl Embed for Element {
    fn embed(self, _: ElementId) -> Element {
        self
    }
}
impl<W: Widget + 'static> Embed for W {
    fn embed(self, id: ElementId) -> Element {
        Element::custom(id.0, self)
    }
}
/// Inserts a Rust element or widget (`{@element expr}`).
pub fn embed(value: impl Embed, id: ElementId) -> Element {
    value.embed(id)
}

/// Value of `<select value={…}>`: an option value or no selection.
pub trait SelectValue {
    /// Selected option value.
    fn selected(self) -> Option<u64>;
}
impl SelectValue for u64 {
    fn selected(self) -> Option<u64> {
        Some(self)
    }
}
impl SelectValue for Option<u64> {
    fn selected(self) -> Option<u64> {
        self
    }
}
impl SelectValue for &u64 {
    fn selected(self) -> Option<u64> {
        Some(*self)
    }
}
impl SelectValue for &Option<u64> {
    fn selected(self) -> Option<u64> {
        *self
    }
}
/// Converts a `<select value={…}>` value.
pub fn selected(value: impl SelectValue) -> Option<u64> {
    value.selected()
}

/// Drop target of `ondrop`: the target's `(group, key)` and the message handler.
pub fn drop_target<M: 'static>(
    key: impl Into<crate::ui::drag_drop::DragSource>,
    handler: impl Fn(crate::ui::drag_drop::DropEvent) -> M + 'static,
) -> crate::ui::drag_drop::DropTarget {
    let key = key.into();
    crate::ui::drag_drop::DropTarget::new(key.group, key.key, handler)
}

impl Element {
    /// Animation played by a non-interactive copy of this element when it disappears
    /// from the view between two rebuilds (template `data-exit`).
    pub fn exit_animation(mut self, animation: Animation) -> Self {
        self.motion.get_or_insert_with(Default::default).exit = Some(animation);
        self
    }
}
