use super::*;
pub(super) fn visit_tree(
    node: &mut Element,
    live: &mut FastSet<ElementId>,
    duplicate: &mut Option<ElementId>,
    order: &mut Vec<ElementId>,
    states: &ComponentStates,
    focused: Option<ElementId>,
) {
    if !live.insert(node.id) {
        *duplicate = Some(node.id);
    }
    let scroll = matches!(node.layout, Layout::Scroll(_));
    if scroll || matches!(node.layout, Layout::VirtualList) {
        node.scroll_offset = states.scrolls.offsets.get(&node.id).copied().unwrap_or(0.);
    }
    // Une liste virtuelle est sa propre cible (ses lignes ne sont pas toutes construites) ;
    // une zone défilante ne l'est que sans descendant focalisable, comme dans Chromium.
    if matches!(node.layout, Layout::VirtualList) {
        order.push(node.id);
    }
    if let Some(list) = &mut node.virtual_list {
        list.active = list.last_index().map(|last| {
            states
                .lists
                .positions
                .get(&node.id)
                .copied()
                .unwrap_or(0)
                .min(last)
        });
        list.pinned = if focused == Some(node.id) {
            list.active
        } else {
            None
        };
        return;
    }
    if !node.skip_tab && node.widget.as_ref().is_some_and(|w| w.focusable()) {
        order.push(node.id);
    }
    if let Layout::Split(mut split) = node.layout {
        if let Some(&fraction) = states.splits.fractions.get(&node.id) {
            split.fraction = fraction;
            node.layout = Layout::Split(split);
        }
        if split.resizable {
            order.push(node.id);
        }
    }
    let start = order.len();
    for child in &mut node.children {
        visit_tree(child, live, duplicate, order, states, focused);
    }
    if scroll && order.len() == start {
        order.push(node.id);
    }
}
