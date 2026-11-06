use super::TemplateContext;
use crate::ui::element::Element;

/// Owned child elements, moved into named slots; no duplicated widget state.
#[derive(Default)]
pub struct Slots {
    groups: Vec<(&'static str, Vec<Element>)>,
}
impl Slots {
    /// Creates an empty slot set.
    pub fn new() -> Self {
        Self::default()
    }
    /// Appends `node` to slot `name`.
    pub fn push(&mut self, name: &'static str, node: Element) {
        if let Some((_, nodes)) = self.groups.iter_mut().find(|(key, _)| *key == name) {
            nodes.push(node);
        } else {
            self.groups.push((name, vec![node]));
        }
    }
    /// Appends every node to slot `name`, in order.
    pub fn extend(&mut self, name: &'static str, nodes: impl IntoIterator<Item = Element>) {
        for node in nodes {
            self.push(name, node);
        }
    }
    /// Moves out the nodes of slot `name`; returns an empty vector for an
    /// unknown or already taken slot.
    pub fn take(&mut self, name: &str) -> Vec<Element> {
        self.groups
            .iter_mut()
            .find(|(key, _)| *key == name)
            .map_or_else(Vec::new, |(_, nodes)| std::mem::take(nodes))
    }
    /// Returns `true` when no slot holds any node.
    pub fn is_empty(&self) -> bool {
        self.groups.iter().all(|(_, nodes)| nodes.is_empty())
    }
    /// Iterates over the names of slots that still hold nodes (not yet taken).
    pub fn unused_names(&self) -> impl Iterator<Item = &str> {
        self.groups
            .iter()
            .filter(|(_, nodes)| !nodes.is_empty())
            .map(|(key, _)| *key)
    }
}
/// Reusable template component, rendered with the caller's context and slots.
///
/// `R` is the application resource type made available through
/// [`TemplateContext::resources`].
pub trait Component<R = ()> {
    /// Builds the component's element tree, consuming its props and slot content.
    fn render(self, cx: &TemplateContext<'_, R>, slots: Slots) -> Element;
}
