//! Placement incrémental : un conteneur disposé et inchangé depuis (sous-arbre repris
//! d'une vue mémorisée, ou image sans changement de style dessous), placé avec les mêmes
//! entrées (taille, bloc conteneur, fenêtre), garde son placement ; déplacé d'un bloc, il
//! est translaté sans être redisposé (comme un défilement). Les mesures sont reprises par
//! `memo::carried`.
use super::position::LayoutContext;
use super::*;
use crate::ui::core::element::memo::MemoArranged;

fn same_size(a: RectF, b: RectF) -> bool {
    width(a) == width(b) && height(a) == height(b)
}
/// `true` si le placement de l'image précédente sert encore (rien à faire de plus).
pub(super) fn reuse(
    node: &mut Element,
    bounds: RectF,
    clip: RectF,
    context: LayoutContext,
) -> bool {
    if !memo::carrying() {
        return false;
    }
    let Some(m) = node.layout_cache.as_deref() else {
        return false;
    };
    let Some(a) = m.arranged.filter(|_| m.settled && !m.missed) else {
        return false;
    };
    let (dx, dy) = (bounds.left - a.bounds.left, bounds.top - a.bounds.top);
    let moved = |r: RectF| RectF {
        left: r.left + dx,
        top: r.top + dy,
        right: r.right + dx,
        bottom: r.bottom + dy,
    };
    if !same_size(a.bounds, bounds)
        || a.viewport != context.viewport
        || a.containing != context.containing
        || moved(a.parent) != context.parent
    {
        return false;
    }
    // Un descendant `absolute` ancré hors du sous-arbre ne suit pas son déplacement :
    // placement refait. Le parcours ne coûte pas plus que la translation qu'il évite.
    let moved_by = dx != 0. || dy != 0.;
    if moved_by && anchored_outside(node) {
        return false;
    }
    if moved_by || a.clip != clip {
        super::scroll::translate(node, dx, dy, clip);
    }
    if let Some(m) = node.layout_cache.as_deref_mut() {
        m.arranged = Some(MemoArranged {
            bounds,
            clip,
            parent: context.parent,
            ..a
        });
    }
    true
}
/// Le sous-arbre de `node` contient un élément `absolute` dont le bloc conteneur est hors
/// de lui (aucun élément positionné entre eux, `node` compris). Un élément `fixed` est
/// placé sur la fenêtre et la translation le laisse en place.
fn anchored_outside(node: &Element) -> bool {
    fn below(node: &Element) -> bool {
        node.children
            .iter()
            .any(|child| match child.rules.as_ref().map(|r| r.position) {
                Some(Position::Absolute) => true,
                None | Some(Position::Static) => below(child),
                // Bloc conteneur de ses descendants (ou placé sur la fenêtre).
                Some(Position::Relative | Position::Sticky | Position::Fixed) => false,
            })
    }
    let positioned = node
        .rules
        .as_ref()
        .is_some_and(|r| r.position != Position::Static);
    !positioned && below(node)
}
/// Oublie les mesures et placements retenus de tout l'arbre (disposition changée hors
/// des styles : séparateur déplacé).
pub(crate) fn unsettle(node: &mut Element) {
    if let Some(cache) = node.layout_cache.as_deref_mut() {
        cache.settled = false;
    }
    for child in &mut node.children {
        unsettle(child);
    }
}
/// Retient les entrées d'un placement complet ; le sous-arbre est désormais réutilisable.
pub(super) fn record(node: &mut Element, bounds: RectF, clip: RectF, context: LayoutContext) {
    if let Some(m) = node.layout_cache.as_deref_mut() {
        m.arranged = Some(MemoArranged {
            bounds,
            clip,
            viewport: context.viewport,
            containing: context.containing,
            parent: context.parent,
        });
        m.settled = true;
    }
}
