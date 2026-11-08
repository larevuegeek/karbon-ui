//! Chaîne survolée lue par `:hover` : l'élément peint au-dessus sous le pointeur et ses
//! ancêtres, relevés dans l'ordre inverse de la peinture comme le hit-test d'un
//! navigateur. Un élément recouvert (calque `fixed` ou absolu de `z-index` supérieur,
//! menu ouvert) n'est pas survolé à travers ce qui le recouvre.
use super::*;
use crate::ui::animation::timeline::Timeline;

/// Le point touche la forme de `node` : sa boîte, affinée par son widget (fragments
/// d'un lien sur plusieurs lignes).
pub(super) fn covers(node: &Element, local: PointF) -> bool {
    layout::contains(node.rect, local.x, local.y)
        && node
            .widget
            .as_ref()
            .is_none_or(|w| w.hit_test(layout::content_rect(node), local))
}
/// Chemin (élément touché d'abord, puis ses ancêtres jusqu'à `node`) de l'élément peint
/// au-dessus au point `world` dans le sous-arbre de `node`, hors éléments `fixed`.
fn walk(node: &Element, world: PointF, motions: &Timeline, events: bool) -> Option<Vec<ElementId>> {
    if !layout::contains(motions.clip(node), world.x, world.y) {
        return None;
    }
    let local = motions.point(node.id, world)?;
    if layout::rounded_clip(node)
        .is_some_and(|(r, radii)| !layout::contains_rounded(r, radii, local))
    {
        return None;
    }
    let events = hit_test::pointer_events(node, events);
    for index in layout::paint_order(node).rev() {
        let child = &node.children[index];
        if layout::fixed(child) {
            continue;
        }
        if let Some(mut path) = walk(child, world, motions, events) {
            path.push(node.id);
            return Some(path);
        }
    }
    (events && covers(node, local)).then(|| vec![node.id])
}
/// Comme `walk` pour les éléments `fixed` du sous-arbre de `node`, peints après les
/// autres : le dernier peint d'abord.
fn fixed(
    node: &Element,
    world: PointF,
    motions: &Timeline,
    events: bool,
) -> Option<Vec<ElementId>> {
    let events = hit_test::pointer_events(node, events);
    for index in layout::paint_order(node).rev() {
        let child = &node.children[index];
        let found = fixed(child, world, motions, events).or_else(|| {
            layout::fixed(child)
                .then(|| walk(child, world, motions, events))
                .flatten()
        });
        if let Some(mut path) = found {
            path.push(node.id);
            return Some(path);
        }
    }
    None
}
/// Chemin de l'élément peint au-dessus dans un calque : ses éléments `fixed`, puis le
/// reste.
fn layer(node: &Element, world: PointF, motions: &Timeline) -> Option<Vec<ElementId>> {
    fixed(node, world, motions, true).or_else(|| walk(node, world, motions, true))
}
impl UiRuntime {
    /// Éléments survolés : l'élément peint au-dessus sous le pointeur et ses ancêtres
    /// (vide sans pointeur). Un dialogue ou un menu ouvert retient le pointeur comme les
    /// clics (voir `hit`) : les calques dessous ne sont pas survolés, comme sous le fond
    /// transparent qui ferme un menu d'une page web.
    pub(super) fn hover_chain(&self) -> Vec<ElementId> {
        let (Some(root), Some(p)) = (self.root.as_ref(), self.interaction.pointer) else {
            return Vec::new();
        };
        if !matches!(root.layout, Layout::Layers) {
            return layer(root, p, &self.motions).unwrap_or_default();
        }
        // Calques peints dans l'ordre : du dernier au calque actif (le contenu sans
        // dialogue ni menu), les listes de suggestions au-dessus comprises.
        let active = layers::active(root);
        let first = root
            .children
            .iter()
            .position(|n| std::ptr::eq(n, active))
            .unwrap_or(0);
        root.children[first..]
            .iter()
            .rev()
            .find_map(|n| layer(n, p, &self.motions))
            .map(|mut path| {
                path.push(root.id);
                path
            })
            .unwrap_or_default()
    }
}
