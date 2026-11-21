//! `TextInput::autofocus` : le champ qui apparaît dans une vue prend le clavier.
use super::*;

/// Premier champ `autofocus` de `root`, dans l'ordre du document, absent de la vue
/// précédente (`previous`).
pub(super) fn target(root: &Element, previous: Option<&Element>) -> Option<ElementId> {
    fn walk(node: &Element, previous: Option<&Element>) -> Option<ElementId> {
        let input = node.widget.as_ref().and_then(|w| w.text_input());
        if input.is_some_and(|i| i.autofocus && i.enabled)
            && previous.is_none_or(|old| old.find(node.id).is_none())
        {
            return Some(node.id);
        }
        node.children.iter().find_map(|child| walk(child, previous))
    }
    walk(root, previous)
}
