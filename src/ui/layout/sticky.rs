//! `position: sticky` : un élément reste dans le flux, puis il est tenu dans la zone visible
//! de la zone défilante la plus proche, à son décalage `top` (ou `bottom`), sans sortir de
//! la boîte de contenu de son parent. Le défilement translate tout le contenu
//! (`scroll::translate`) ; chaque élément collant est ensuite recalé depuis sa place dans le
//! flux (`LayoutRules::sticky_shift`), après une mise en page (`refresh`) comme après un
//! déplacement (`stick`).
use super::*;

/// L'élément est collant.
pub(crate) fn sticky(node: &Element) -> bool {
    node.rules
        .as_ref()
        .is_some_and(|r| r.position == Position::Sticky)
}

/// Recale les éléments collants de toutes les zones défilantes sous `node`.
pub(super) fn refresh(node: &mut Element) {
    if node.content_skipped || hidden(node) {
        return;
    }
    if node.scroll.is_some() {
        stick(node);
    }
    for child in &mut node.children {
        refresh(child);
    }
}

/// Recale les éléments collants de la zone défilante `scroller` (pas ceux des zones
/// défilantes qu'elle contient : elles ont leur propre vue).
pub(crate) fn stick(scroller: &mut Element) {
    let Some(s) = scroller.scroll else {
        return;
    };
    if s.axis != Axis::Vertical {
        return;
    }
    let clip = intersect(s.viewport, scroller.clip);
    let content = content_rect(scroller);
    for child in &mut scroller.children {
        walk(child, s.viewport, content, clip);
    }
}

fn walk(node: &mut Element, view: RectF, parent: RectF, clip: RectF) {
    if node.content_skipped || hidden(node) || out_of_flow(node) {
        return;
    }
    if sticky(node) {
        hold(node, view, parent, clip);
    }
    if node.scroll.is_some() {
        return;
    }
    let content = content_rect(node);
    let child_clip = child_clip(node, content);
    for child in &mut node.children {
        walk(child, view, content, child_clip);
    }
}

/// Place `node` (collant) dans `view`, sans sortir de la boîte `parent`.
fn hold(node: &mut Element, view: RectF, parent: RectF, clip: RectF) {
    let Some(rules) = node.rules.as_ref() else {
        return;
    };
    let reference = height(view);
    let vars = node.styles.as_ref().and_then(|s| s.variables.as_ref());
    let resolve = |i: usize| {
        super::position::offset(rules.offsets[i], reference, rules.unit_context, vars)
            .ok()
            .flatten()
    };
    let (top, bottom) = (resolve(0), resolve(2));
    if top.is_none() && bottom.is_none() {
        return;
    }
    let shift = rules.sticky_shift;
    let flow = node.rect.top - shift;
    let tall = height(node.rect) + node.style.margin.bottom;
    let mut target = flow;
    if let Some(top) = top {
        // Pas plus bas que le bas de son parent.
        target = target.max(view.top + top).min(parent.bottom - tall);
    } else if let Some(bottom) = bottom {
        // Pas plus haut que le haut de son parent.
        target = target
            .min(view.bottom - bottom - height(node.rect))
            .max(parent.top + node.style.margin.top);
    }
    // Jamais au-dessus de sa place dans le flux pour `top`, ni en dessous pour `bottom`.
    target = if top.is_some() {
        target.max(flow)
    } else {
        target.min(flow)
    };
    let delta = target - node.rect.top;
    if delta.abs() < 0.001 {
        return;
    }
    super::scroll::translate(node, 0., delta, clip);
    if let Some(rules) = node.rules.as_mut() {
        rules.sticky_shift = shift + delta;
    }
}
