//! Sous-arbres sans pixel à peindre : entièrement hors de leur découpe.
//!
//! Un élément en `overflow: visible` hérite la découpe de son parent : dans une liste
//! défilante, une ligne sortie de la vue garde la découpe de toute la liste et n'est donc
//! pas écartée par le test de découpe vide de `paint_inner`. Sans ce tri, une liste de
//! 200 lignes peignait ses 200 lignes à chaque image (un rognage par élément, un calque
//! par bouton masqué), pour une quinzaine de visibles.
use super::*;
use crate::ui::animation::timeline::Timeline;
use crate::ui::rendering::transform::Affine;

/// Marge autour de l'encre d'un élément : lissage des bords, débord des glyphes, calage
/// sur les pixels physiques.
const INK_MARGIN: f32 = 4.;

/// `node` ou l'un de ses descendants (hors éléments `fixed`, peints à part) peut-il
/// laisser des pixels dans `clip` ? Dans le doute (transformation, contrôle natif), oui.
///
/// Tous les descendants peignent dans une découpe incluse dans `clip` (la leur se déduit
/// de celle de leurs ancêtres) : un sous-arbre dont aucune encre ne coupe `clip` ne
/// change aucun pixel. Le premier élément qui touche `clip` arrête le parcours, si bien
/// qu'un élément visible coûte un test et qu'un sous-arbre écarté est parcouru une fois.
pub(super) fn reaches(node: &Element, clip: RectF, motions: &Timeline) -> bool {
    if motions
        .local(node)
        .is_some_and(|(matrix, _)| matrix != Affine::IDENTITY)
    {
        return true;
    }
    if node
        .widget
        .as_ref()
        .is_some_and(|w| w.text_input().is_some() || w.slider().is_some() || w.select().is_some())
    {
        return true;
    }
    if touches(ink(node), clip) {
        return true;
    }
    !node.content_skipped
        && node
            .children
            .iter()
            .any(|child| !layout::fixed(child) && reaches(child, clip, motions))
}

/// Zone où `node` peint lui-même : boîte peinte, ombres, contour, et contenu de son widget
/// (un texte plus grand que sa boîte déborde en `overflow: visible`).
fn ink(node: &Element) -> RectF {
    let mut r = union(node.rect, painted(node));
    if let Some(styles) = &node.styles {
        r = styles.shadow_cache.bounds(node.rect, r);
        let local = &styles.computed.local;
        if let Some(width) = local.outline_width.filter(|w| *w > 0.) {
            r = inflate(r, local.outline_offset.unwrap_or(0.).abs() + width);
        }
    }
    if node.widget.is_some() {
        r = union(
            r,
            RectF {
                left: node.rect.left,
                top: node.rect.top,
                right: node.rect.left + node.intrinsic.width,
                bottom: node.rect.top + node.intrinsic.height,
            },
        );
    }
    inflate(r, INK_MARGIN)
}

fn touches(a: RectF, b: RectF) -> bool {
    let i = layout::intersect(a, b);
    layout::width(i) > 0. && layout::height(i) > 0.
}

fn union(a: RectF, b: RectF) -> RectF {
    RectF {
        left: a.left.min(b.left),
        top: a.top.min(b.top),
        right: a.right.max(b.right),
        bottom: a.bottom.max(b.bottom),
    }
}

fn inflate(r: RectF, by: f32) -> RectF {
    RectF {
        left: r.left - by,
        top: r.top - by,
        right: r.right + by,
        bottom: r.bottom + by,
    }
}
