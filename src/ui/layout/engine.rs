use super::*;
pub(super) use crate::ui::{element::*, scene::RectF};
pub(super) use std::collections::HashSet;

/// Thickness, in logical pixels, of the draggable divider between the two panes of a split.
pub const DIVIDER_SIZE: f32 = 10.;
pub(super) fn invalid(message: &'static str) -> crate::Error {
    crate::Error::invalid_input(message)
}
/// Width of `r`, never negative.
pub fn width(r: RectF) -> f32 {
    (r.right - r.left).max(0.)
}
/// Height of `r`, never negative.
pub fn height(r: RectF) -> f32 {
    (r.bottom - r.top).max(0.)
}
/// Returns `true` if the point lies inside `r` (left/top edges inclusive, right/bottom
/// exclusive).
pub fn contains(r: RectF, x: f32, y: f32) -> bool {
    x >= r.left && x < r.right && y >= r.top && y < r.bottom
}
/// Intersection of two rectangles; an empty (zero-sized) rectangle when they do not overlap.
pub fn intersect(a: RectF, b: RectF) -> RectF {
    let left = a.left.max(b.left);
    let top = a.top.max(b.top);
    RectF {
        left,
        top,
        right: a.right.min(b.right).max(left),
        bottom: a.bottom.min(b.bottom).max(top),
    }
}
pub(crate) fn node_clip(node: &Element, bounds: RectF, parent: RectF) -> RectF {
    let (x, y) = node
        .rules
        .as_ref()
        .map_or((Overflow::Clip, Overflow::Clip), |r| {
            // Révélation en cours : contenu rogné comme `overflow: hidden` (`.collapsing`) ;
            // `content-visibility` confine la peinture de même.
            if r.reveal.is_some() || containment::paint_contained(r) {
                (Overflow::Clip, Overflow::Clip)
            } else {
                (r.overflow_x, r.overflow_y)
            }
        });
    let mut clip = intersect(bounds, parent);
    if x == Overflow::Visible {
        clip.left = parent.left;
        clip.right = parent.right;
    }
    if y == Overflow::Visible {
        clip.top = parent.top;
        clip.bottom = parent.bottom;
    }
    clip
}
pub(crate) fn child_clip(node: &Element, area: RectF) -> RectF {
    node_clip(node, area, node.clip)
}
/// Explicit two-axis overflow clips to the rounded padding box. Legacy clipping
/// remains rectangular unless the caller opts in with overflow().
pub(crate) fn rounded_clip(node: &Element) -> Option<(RectF, [f32; 4])> {
    let rules = node.rules.as_ref()?;
    if !containment::paint_contained(rules)
        && (!rules.rounded_clip
            || rules.overflow_x == Overflow::Visible
            || rules.overflow_y == Overflow::Visible)
    {
        return None;
    }
    let radii = node
        .styles
        .as_ref()
        .and_then(|s| s.computed.local.corner_radii)
        .unwrap_or([node.style.radius; 4]);
    if radii.iter().all(|r| *r == 0.) {
        return None;
    }
    let b = node.border();
    let rect = RectF::new(
        node.rect.left + b.left,
        node.rect.top + b.top,
        (width(node.rect) - b.horizontal()).max(0.),
        (height(node.rect) - b.vertical()).max(0.),
    );
    Some((
        rect,
        [
            (radii[0] - b.left.max(b.top)).max(0.),
            (radii[1] - b.right.max(b.top)).max(0.),
            (radii[2] - b.right.max(b.bottom)).max(0.),
            (radii[3] - b.left.max(b.bottom)).max(0.),
        ],
    ))
}
pub(crate) fn contains_rounded(
    rect: RectF,
    mut radii: [f32; 4],
    p: crate::ui::scene::PointF,
) -> bool {
    if !contains(rect, p.x, p.y) {
        return false;
    }
    let w = width(rect);
    let h = height(rect);
    let mut factor = 1f32;
    for (size, sum) in [
        (w, radii[0] + radii[1]),
        (w, radii[3] + radii[2]),
        (h, radii[0] + radii[3]),
        (h, radii[1] + radii[2]),
    ] {
        if sum > 0. {
            factor = factor.min(size / sum);
        }
    }
    for r in &mut radii {
        *r *= factor;
    }
    for (r, cx, cy, corner) in [
        (radii[0], rect.left + radii[0], rect.top + radii[0], 0),
        (radii[1], rect.right - radii[1], rect.top + radii[1], 1),
        (radii[2], rect.right - radii[2], rect.bottom - radii[2], 2),
        (radii[3], rect.left + radii[3], rect.bottom - radii[3], 3),
    ] {
        let in_corner = match corner {
            0 => p.x < cx && p.y < cy,
            1 => p.x > cx && p.y < cy,
            2 => p.x > cx && p.y > cy,
            _ => p.x < cx && p.y > cy,
        };
        if in_corner && (p.x - cx).powi(2) + (p.y - cy).powi(2) > r * r {
            return false;
        }
    }
    true
}
/// Indices des enfants dans l'ordre de peinture ; aucun quand le contenu est sauté
/// (`content-visibility`) : ni peint ni touché.
pub(crate) fn paint_order(node: &Element) -> impl DoubleEndedIterator<Item = usize> + '_ {
    let count = if node.content_skipped {
        0
    } else {
        node.children.len()
    };
    (0..count).map(|i| {
        node.rules
            .as_ref()
            .filter(|r| !r.paint_order.is_empty())
            .map_or(i, |r| r.paint_order[i])
    })
}
pub(crate) fn update_paint_order(node: &mut Element) {
    // Rang d'un enfant : son `z-index`, sinon celui qu'un descendant de feuille de
    // framework fait remonter (un `.accordion-button:focus { z-index: 3 }` passe devant
    // l'élément d'accordéon suivant, comme dans un navigateur).
    let key = |c: &Element| {
        c.rules.as_ref().map_or(0, |r| {
            if r.z_index != 0 {
                r.z_index
            } else {
                r.lifted_z
            }
        })
    };
    if node.children.iter().any(|c| key(c) != 0) {
        let mut order = (0..node.children.len()).collect::<Vec<_>>();
        order.sort_by_key(|i| key(&node.children[*i]));
        node.layout_rules().paint_order = order;
    } else if let Some(r) = &mut node.rules {
        r.paint_order.clear();
    }
    // Les feuilles strictes gardent un `z-index` limité aux frères.
    let lifted = node
        .children
        .iter()
        .map(|c| {
            let framework = c.styles.as_ref().is_some_and(|s| s.globals.is_some());
            c.rules.as_ref().map_or(0, |r| match r.z_index {
                0 => r.lifted_z,
                z if framework => z,
                _ => 0,
            })
        })
        .max()
        .unwrap_or(0)
        .max(0);
    if lifted != 0 || node.rules.as_ref().is_some_and(|r| r.lifted_z != 0) {
        node.layout_rules().lifted_z = lifted;
    }
}
/// Content box of an arranged element: its rectangle minus padding and border.
pub fn content_rect(node: &Element) -> RectF {
    let p = node.style.padding;
    let b = node.border();
    RectF::new(
        node.rect.left + p.left + b.left,
        node.rect.top + p.top + b.top,
        (width(node.rect) - p.left - p.right - b.horizontal()).max(0.),
        (height(node.rect) - p.top - p.bottom - b.vertical()).max(0.),
    )
}
pub(super) fn box_edges(node: &Element) -> Insets {
    let p = node.style.padding;
    let b = node.border();
    Insets::edges(
        p.top + b.top,
        p.right + b.right,
        p.bottom + b.bottom,
        p.left + b.left,
    )
}
/// Résout les limites relatives (`max-width: 40%`, `calc(100vh - 330px)`) de `node`
/// contre la boîte de contenu de son parent (`containing`) et les écrit dans ses
/// limites en px. Un pourcentage d'une taille indéfinie ne limite rien.
pub(crate) fn resolve_limits(node: &mut Element, containing: SizeF) {
    let Some(rules) = node.rules.as_ref() else {
        return;
    };
    if rules.relative_limits.iter().all(Option::is_none) {
        return;
    }
    let (lengths, context) = (rules.relative_limits, rules.unit_context);
    let variables = node.styles.as_ref().and_then(|s| s.variables.clone());
    for (i, length) in lengths.into_iter().enumerate() {
        let Some(length) = length else {
            continue;
        };
        let reference = if i % 2 == 0 {
            containing.width
        } else {
            containing.height
        };
        let px = match length {
            Length::Px(n) => n,
            Length::Percent(n) if reference.is_finite() => reference * n / 100.,
            Length::Computed(v) if reference.is_finite() || !v.expression.relative() => {
                v.evaluate(context, reference, variables.as_ref())
            }
            _ => f32::NAN,
        };
        let px = if px.is_finite() {
            px.max(0.)
        } else if i < 2 {
            0.
        } else {
            f32::INFINITY
        };
        match i {
            0 => node.style.min_width = px,
            1 => node.style.min_height = px,
            2 => node.style.max_width = px,
            _ => node.style.max_height = px,
        }
    }
    // Comme en CSS, le minimum l'emporte sur un maximum plus petit.
    node.style.max_width = node.style.max_width.max(node.style.min_width);
    node.style.max_height = node.style.max_height.max(node.style.min_height);
}
pub(super) fn limits(node: &Element, row: bool) -> (f32, f32) {
    let e = box_edges(node);
    let extra = if node.content_box() {
        if row { e.horizontal() } else { e.vertical() }
    } else {
        0.
    };
    let (min, max) = if row {
        (node.style.min_width, node.style.max_width)
    } else {
        (node.style.min_height, node.style.max_height)
    };
    let floor = if node.rules.is_some() {
        if row { e.horizontal() } else { e.vertical() }
    } else {
        0.
    };
    // Comme en CSS, le minimum l'emporte sur un maximum plus petit : une règle d'état
    // (`md:max-w-…`) peut poser l'un après la validation de l'arbre, et les appelants
    // bornent par `clamp`, qui refuse des bornes inversées.
    let min = (min + extra).max(floor);
    (min, (max + extra).max(min))
}
pub(super) use super::arrange::arrange_in;
pub(super) use super::grid::placements;
pub(super) use super::position::LayoutContext;
