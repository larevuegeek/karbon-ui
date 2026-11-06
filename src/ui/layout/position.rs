use super::*;
#[derive(Clone, Copy)]
pub(super) struct LayoutContext {
    pub viewport: RectF,
    pub containing: RectF,
    pub parent: RectF,
}
pub(crate) fn out_of_flow(node: &Element) -> bool {
    node.rules.as_ref().is_some_and(|r| {
        (r.hidden && r.reveal.is_none())
            || matches!(r.position, Position::Absolute | Position::Fixed)
    })
}
/// `display: none` : l'élément et son sous-arbre ne sont ni placés ni dessinés. Un
/// élément masqué dont la révélation se referme reste affiché jusqu'à sa fin.
pub(crate) fn hidden(node: &Element) -> bool {
    node.rules
        .as_ref()
        .is_some_and(|r| r.hidden && r.reveal.is_none())
}
/// Boîte vide pour un sous-arbre masqué : clip nul, donc ni dessin ni clic.
pub(super) fn collapse(node: &mut Element, left: f32, top: f32) {
    node.rect = RectF::new(left, top, 0., 0.);
    node.clip = node.rect;
    node.scroll = None;
    node.divider = None;
    for child in &mut node.children {
        collapse(child, left, top);
    }
}
/// Ordre de layout (`order`) : trie les enfants de façon stable et renvoie la
/// permutation à défaire après le layout.
pub(super) fn reorder(node: &mut Element) -> Option<Vec<usize>> {
    let order = |c: &Element| c.rules.as_ref().map_or(0, |r| r.order);
    if node.children.iter().all(|c| order(c) == 0) {
        return None;
    }
    let mut indices = (0..node.children.len()).collect::<Vec<_>>();
    indices.sort_by_key(|&i| order(&node.children[i]));
    let mut slots = std::mem::take(&mut node.children)
        .into_iter()
        .map(Some)
        .collect::<Vec<_>>();
    node.children = indices.iter().map(|&i| slots[i].take().unwrap()).collect();
    Some(indices)
}
pub(super) fn unorder(node: &mut Element, indices: Option<Vec<usize>>) {
    let Some(indices) = indices else {
        return;
    };
    let mut slots = (0..indices.len())
        .map(|_| None)
        .collect::<Vec<Option<Element>>>();
    for (child, index) in std::mem::take(&mut node.children).into_iter().zip(indices) {
        slots[index] = Some(child);
    }
    node.children = slots.into_iter().map(Option::unwrap).collect();
}
pub(crate) fn fixed(node: &Element) -> bool {
    node.rules
        .as_ref()
        .is_some_and(|r| r.position == Position::Fixed)
}
pub(super) fn take(node: &mut Element) -> Vec<(usize, Element)> {
    if !node.children.iter().any(out_of_flow) {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (index, child) in std::mem::take(&mut node.children).into_iter().enumerate() {
        if out_of_flow(&child) {
            out.push((index, child));
        } else {
            node.children.push(child);
        }
    }
    out
}
pub(super) fn restore(node: &mut Element, out: Vec<(usize, Element)>) {
    for (index, child) in out {
        node.children.insert(index, child);
    }
}
fn offset(
    length: Option<Length>,
    reference: f32,
    context: UnitContext,
    variables: Option<&crate::ui::styling::CssVariables>,
) -> crate::Result<Option<f32>> {
    let n = match length {
        None | Some(Length::Auto) => return Ok(None),
        Some(Length::Px(n)) => n,
        Some(Length::Percent(n)) => reference * n / 100.,
        Some(Length::Computed(n)) => n.evaluate(context, reference, variables),
        Some(Length::Fill(_)) => return Err(invalid("an offset cannot be a fraction")),
    };
    if !n.is_finite() {
        return Err(invalid("non-finite offset"));
    }
    Ok(Some(n))
}
pub(super) fn relative(
    node: &Element,
    bounds: RectF,
    context: LayoutContext,
) -> crate::Result<RectF> {
    let Some(r) = node
        .rules
        .as_ref()
        .filter(|r| r.position == Position::Relative)
    else {
        return Ok(bounds);
    };
    let vars = node.styles.as_ref().and_then(|s| s.variables.as_ref());
    let ctx = r.unit_context;
    let x = offset(r.offsets[3], width(context.parent), ctx, vars)?
        .unwrap_or(-offset(r.offsets[1], width(context.parent), ctx, vars)?.unwrap_or(0.));
    let y = offset(r.offsets[0], height(context.parent), ctx, vars)?
        .unwrap_or(-offset(r.offsets[2], height(context.parent), ctx, vars)?.unwrap_or(0.));
    Ok(RectF::new(
        bounds.left + x,
        bounds.top + y,
        bounds.right - bounds.left,
        bounds.bottom - bounds.top,
    ))
}
pub(super) fn placed(
    node: &mut Element,
    context: LayoutContext,
    clip: RectF,
    text: &mut dyn TextMeasurer,
) -> crate::Result<()> {
    let r = node.rules.as_ref().unwrap();
    let area = if fixed(node) {
        context.viewport
    } else {
        context.containing
    };
    let (w, h) = (width(area), height(area));
    let vars = node.styles.as_ref().and_then(|s| s.variables.as_ref());
    let ctx = r.unit_context;
    let left = offset(r.offsets[3], w, ctx, vars)?;
    let right = offset(r.offsets[1], w, ctx, vars)?;
    let top = offset(r.offsets[0], h, ctx, vars)?;
    let bottom = offset(r.offsets[2], h, ctx, vars)?;
    let original = (
        node.style.width,
        node.style.height,
        node.style.margin,
        node.style.max_width,
        node.style.max_height,
    );
    let edges = box_edges(node);
    let m = node.style.margin;
    let auto_w = matches!(original.0, Length::Auto | Length::Fill(_));
    let auto_h = matches!(original.1, Length::Auto | Length::Fill(_));
    if auto_w {
        node.style.width = if let (Some(l), Some(r)) = (left, right) {
            Length::Px(
                (w - l
                    - r
                    - m.horizontal()
                    - if node.content_box() {
                        edges.horizontal()
                    } else {
                        0.
                    })
                .max(0.),
            )
        } else {
            Length::Auto
        };
    }
    if auto_h {
        node.style.height = if let (Some(t), Some(b)) = (top, bottom) {
            Length::Px(
                (h - t
                    - b
                    - m.vertical()
                    - if node.content_box() {
                        edges.vertical()
                    } else {
                        0.
                    })
                .max(0.),
            )
        } else {
            Length::Auto
        };
    }
    let result = (|| {
        if auto_w {
            node.style.max_width = node.style.max_width.min(
                (w - left.unwrap_or(0.)
                    - right.unwrap_or(0.)
                    - m.horizontal()
                    - if node.content_box() {
                        edges.horizontal()
                    } else {
                        0.
                    })
                .max(node.style.min_width),
            );
        }
        if auto_h {
            node.style.max_height = node.style.max_height.min(
                (h - top.unwrap_or(0.)
                    - bottom.unwrap_or(0.)
                    - m.vertical()
                    - if node.content_box() {
                        edges.vertical()
                    } else {
                        0.
                    })
                .max(node.style.min_height),
            );
        }
        let mut size = measure(
            node,
            SizeF {
                width: w,
                height: h,
            },
            text,
        )?;
        let auto = node.auto_margins();
        if let (Some(left), Some(right)) = (left, right)
            && auto.count(true) > 0
        {
            let free = (w - left - right - size.width).max(0.);
            node.style.margin = auto.add(node.style.margin, true, free / auto.count(true) as f32);
            size.width += free;
        }
        if let (Some(top), Some(bottom)) = (top, bottom)
            && auto.count(false) > 0
        {
            let free = (h - top - bottom - size.height).max(0.);
            node.style.margin = auto.add(node.style.margin, false, free / auto.count(false) as f32);
            size.height += free;
        }
        let x = left
            .map(|n| area.left + n)
            .or_else(|| right.map(|n| area.right - n - size.width))
            .unwrap_or(context.parent.left);
        let y = top
            .map(|n| area.top + n)
            .or_else(|| bottom.map(|n| area.bottom - n - size.height))
            .unwrap_or(context.parent.top);
        arrange_in(
            node,
            RectF::new(x, y, size.width, size.height),
            if fixed(node) { context.viewport } else { clip },
            text,
            context,
        )
    })();
    node.style.width = original.0;
    node.style.height = original.1;
    node.style.margin = original.2;
    node.style.max_width = original.3;
    node.style.max_height = original.4;
    result
}
