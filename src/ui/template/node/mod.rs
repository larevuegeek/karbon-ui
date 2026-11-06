//! Nœud typé construit par le code généré : modèle, construction et finalisation. Les
//! méthodes de style sont réparties par thème dans les sous-modules.
use super::TemplateContext;
use crate::ui::element::*;
mod cascade;
mod container;
mod motion;
mod sizing;
mod surface;
mod text;

/// A typed widget is boxed only after its classes and attributes have been applied.
pub struct Node<W: Widget> {
    pub(super) element: Element,
    widget: Option<W>,
    action: Option<crate::application::Action>,
}
/// Placeholder widget for nodes without content (plain containers); measures
/// zero and paints nothing.
pub struct Empty;
/// Capabilities reserved for built-in layouts and rendered component roots.
pub trait ContainerStyle: Widget {}
impl ContainerStyle for Empty {}
impl Widget for Empty {
    fn measure(&self, _: SizeF, _: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
        Ok(SizeF::default())
    }
    fn paint(&self, _: &PaintContext, _: &mut crate::ui::scene::Scene) {}
}
impl Node<Empty> {
    /// Wraps an already built element.
    pub fn element(element: Element) -> Self {
        Self {
            element,
            widget: None,
            action: None,
        }
    }
    /// Attaches a widget that paints the container's content.
    pub fn painter(mut self, value: impl Widget + 'static) -> Self {
        self.element.widget = Some(Box::new(value));
        self
    }
}
impl<W: Widget + 'static> Node<W> {
    /// Creates a leaf node with element identifier `id` holding `widget`.
    pub fn widget(id: ElementId, widget: W) -> Self {
        let mut element = Element::column(id.0, Vec::new());
        element.layout = Layout::Leaf;
        Self {
            element,
            widget: Some(widget),
            action: None,
        }
    }
    /// Transforms the typed widget before it is boxed.
    pub fn map_widget(mut self, f: impl FnOnce(W) -> W) -> Self {
        self.widget = self.widget.map(f);
        self
    }
    /// Finishes the node: boxes the widget, applies the base style and wraps it
    /// in a clickable area when [`on_click`](Self::on_click) was set.
    pub fn build(mut self) -> Element {
        if let Some(widget) = self.widget {
            self.element.widget = Some(Box::new(widget));
        }
        // HTML : `overflow: visible` par défaut. Un conteneur sans overflow déclaré ne rogne
        // pas ses enfants (`::after` hors de l'hôte, ombres, débordements) ; les feuilles
        // (texte, contrôles), dialogues et popovers gardent leur clip.
        let explicit = self.element.rules.as_ref().is_some_and(|r| r.rounded_clip);
        let layer = self
            .element
            .widget
            .as_ref()
            .is_some_and(|w| w.popover().is_some() || w.modal().is_some());
        if !explicit && !layer && !matches!(self.element.layout, Layout::Leaf) {
            let rules = self.element.layout_rules();
            rules.overflow_x = Overflow::Visible;
            rules.overflow_y = Overflow::Visible;
        }
        scroll_container(&mut self.element);
        crate::ui::styling::resolve::local(&mut self.element);
        if let Some(action) = self.action {
            self.element.widget = Some(Box::new(super::action::ActionArea::new(
                self.element.widget.take(),
                action,
            )));
        }
        self.element
    }
    /// Identity of the element being built.
    pub fn element_id(&self) -> ElementId {
        self.element.id
    }
    /// Inserts a generated box (`::before` as first child, `::after` as last child).
    /// Leaf nodes (text, controls) have no children slot: the box is not added.
    pub fn pseudo_element(mut self, after: bool, element: Element) -> Self {
        if matches!(self.element.layout, Layout::Leaf) {
            return self;
        }
        let children = &mut self.element.children;
        children.retain(|c| c.id != element.id);
        if after {
            children.push(element);
        } else {
            children.insert(0, element);
        }
        self
    }
    /// Make the node clickable: activation emits a clone of `message`.
    pub fn on_click<M: Clone + 'static>(mut self, message: M) -> Self {
        self.action = Some(crate::application::Action::new(message));
        self
    }
    /// Shows `value` in a tooltip on hover.
    pub fn tooltip(mut self, value: impl Into<std::sync::Arc<str>>) -> Self {
        self.element = self.element.tooltip(value);
        self
    }
    pub(super) fn properties(&mut self) -> &mut crate::ui::styling::Properties {
        &mut self
            .element
            .styles
            .get_or_insert_with(Default::default)
            .normal
    }
}
/// `overflow: auto | scroll` : le conteneur devient une zone défilante. La boîte
/// (taille, marges, fond, bordure, padding, classes) reste sur la zone ; le contenu
/// défilant reçoit les enfants et leur disposition (direction, grille, gap, alignements).
fn scroll_container(element: &mut Element) {
    let Some(rules) = element.rules.as_deref() else {
        return;
    };
    let scrolls = |o: Overflow| matches!(o, Overflow::Auto | Overflow::Scroll);
    if !matches!(
        element.layout,
        Layout::Row | Layout::Column | Layout::Grid { .. }
    ) || !(scrolls(rules.overflow_x) || scrolls(rules.overflow_y))
    {
        return;
    }
    let axis = if scrolls(rules.overflow_y) {
        Axis::Vertical
    } else {
        Axis::Horizontal
    };
    let mut content = Element::column(
        element.id.child("scroll-content").0,
        std::mem::take(&mut element.children),
    );
    content.layout = element.layout;
    content.style.gap = element.style.gap;
    content.style.row_gap = element.style.row_gap;
    content.style.column_gap = element.style.column_gap;
    content.style.align = element.style.align;
    content.style.justify = element.style.justify;
    if axis == Axis::Horizontal {
        content.style.width = Length::Auto;
        content.style.height = Length::Fill(1.);
    }
    let inner = content.layout_rules();
    inner.css_flex = rules.css_flex;
    inner.flex_wrap = rules.flex_wrap;
    inner.flex_reverse = rules.flex_reverse;
    inner.columns = rules.columns.clone();
    element.layout = Layout::Scroll(axis);
    element.children = vec![content];
}
/// Applies a style function (generated by [`styles!`](crate::styles) or
/// [`css_file!`](crate::css_file)) to `node`.
pub fn style<R, W: Widget + 'static>(
    node: Node<W>,
    cx: &TemplateContext<'_, R>,
    class: impl FnOnce(Node<W>, &TemplateContext<'_, R>) -> Node<W>,
) -> Node<W> {
    class(node, cx)
}
/// Style function that leaves `node` unchanged. Compiled class lists use it for the
/// positions left free once Tailwind utilities are merged into one function.
#[doc(hidden)]
pub fn unstyled<R, W: Widget + 'static>(node: Node<W>, _: &TemplateContext<'_, R>) -> Node<W> {
    node
}
