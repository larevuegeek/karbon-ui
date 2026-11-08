//! Ancrage du défilement, comme `overflow-anchor: auto` des navigateurs : quand une
//! reconstruction ou une remise en page change ce qui précède la zone visible d'un
//! conteneur défilant (lignes ajoutées ou retirées au-dessus, fenêtre d'une liste
//! chargée par pages), l'élément regardé garde sa place à l'écran.
//!
//! Avant la disposition, chaque conteneur vertical défilé (décalage non nul) retient son
//! ancre : le premier élément visible, en descendant dans celui qui chevauche le haut de
//! la vue. Après la disposition, si l'ancre existe encore (même identifiant), le décalage
//! est corrigé de son déplacement. Un conteneur en haut (décalage nul) n'est pas ancré :
//! du contenu ajouté au-dessus reste visible, comme dans Chromium.
//!
//! Un conteneur qui apparaît part de sa cible `scroll-initial-target` éventuelle.
use super::*;
use std::collections::HashMap;

/// Ancre retenue pour un conteneur défilant.
pub(super) struct Anchor {
    scroller: ElementId,
    target: ElementId,
    /// Position du haut de l'ancre par rapport au haut de la vue.
    top: f32,
}

/// Ancres de l'arbre disposé `root`.
pub(super) fn capture(root: &Element) -> Vec<Anchor> {
    fn walk(node: &Element, anchors: &mut Vec<Anchor>) {
        if let Some(s) = node.scroll
            && s.axis == Axis::Vertical
            && s.offset > 0.
            && node.virtual_list.is_none()
            && let Some((target, top)) = node.children.first().and_then(|c| pick(c, s.viewport))
        {
            anchors.push(Anchor {
                scroller: node.id,
                target,
                top: top - s.viewport.top,
            });
        }
        for child in &node.children {
            walk(child, anchors);
        }
    }
    let mut anchors = Vec::new();
    walk(root, &mut anchors);
    anchors
}

/// Premier descendant de `node` visible dans `view` : le plus profond qui chevauche le
/// haut de la vue, ou le premier entièrement dessous.
fn pick(node: &Element, view: RectF) -> Option<(ElementId, f32)> {
    for child in &node.children {
        if layout::out_of_flow(child) || layout::hidden(child) {
            continue;
        }
        let r = child.rect;
        if layout::height(r) <= 0. || r.bottom <= view.top || r.top >= view.bottom {
            continue;
        }
        if r.top >= view.top || child.children.is_empty() || child.scroll.is_some() {
            return Some((child.id, r.top));
        }
        return pick(child, view).or(Some((child.id, r.top)));
    }
    None
}

/// `scroll-initial-target: nearest` : un conteneur vertical qui apparaît (absent de
/// `known`, les décalages retenus) part de sa cible, en tenant compte de sa
/// `scroll-margin` et du `scroll-padding` du conteneur.
pub(super) fn initial_targets(root: &mut Element, known: &HashMap<ElementId, f32>) {
    fn target(node: &Element) -> Option<&Element> {
        node.children.iter().find_map(|child| {
            if child
                .rules
                .as_ref()
                .is_some_and(|r| r.scroll_initial_target)
            {
                Some(child)
            } else if child.scroll.is_some() {
                None
            } else {
                target(child)
            }
        })
    }
    fn walk(node: &mut Element, known: &HashMap<ElementId, f32>) {
        if let Some(s) = node.scroll
            && s.axis == Axis::Vertical
            && !known.contains_key(&node.id)
            && let Some(t) = target(node)
        {
            let margin = t.rules.as_ref().map_or(0., |r| r.scroll_margin.top);
            let padding = node.rules.as_ref().map_or(0., |r| r.scroll_padding.top);
            let delta = t.rect.top - margin - (s.viewport.top + padding);
            scroll::set_offset(node, s.offset + delta);
        }
        for child in &mut node.children {
            walk(child, known);
        }
    }
    walk(root, known);
}

/// Corrige le décalage des conteneurs dont l'ancre s'est déplacée ; renvoie les
/// corrections appliquées (conteneur, déplacement), pour qu'un glissement en cours suive.
pub(super) fn apply(root: &mut Element, anchors: &[Anchor]) -> Vec<(ElementId, f32)> {
    let mut moved = Vec::new();
    for anchor in anchors {
        let Some(node) = root.find_mut(anchor.scroller) else {
            continue;
        };
        let Some(s) = node.scroll else {
            continue;
        };
        let Some(target) = node.find(anchor.target) else {
            continue;
        };
        let delta = target.rect.top - s.viewport.top - anchor.top;
        if delta.abs() > 0.01 {
            // Le contenu s'est déplacé de `delta` même si le décalage est borné.
            scroll::set_offset(node, s.offset + delta);
            moved.push((anchor.scroller, delta));
        }
    }
    moved
}
