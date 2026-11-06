use super::*;
/// Second layout pass: positions `node` and its descendants inside `bounds`, clipped to `clip`.
/// `bounds` also serves as the viewport and containing block for absolutely/fixed positioned
/// descendants. Fails with [`Error::InvalidInput`](crate::Error::InvalidInput) on invalid layout values.
pub fn arrange(
    node: &mut Element,
    bounds: RectF,
    clip: RectF,
    text: &mut dyn TextMeasurer,
) -> crate::Result<()> {
    arrange_in(
        node,
        bounds,
        clip,
        text,
        LayoutContext {
            viewport: bounds,
            containing: bounds,
            parent: bounds,
        },
    )
}
pub(super) fn arrange_in(
    node: &mut Element,
    bounds: RectF,
    clip: RectF,
    text: &mut dyn TextMeasurer,
    context: LayoutContext,
) -> crate::Result<()> {
    let bounds = super::position::relative(node, bounds, context)?;
    let m = node.style.margin;
    let bounds = RectF::new(
        bounds.left + m.left,
        bounds.top + m.top,
        (bounds.right - bounds.left - m.horizontal()).max(0.),
        (bounds.bottom - bounds.top - m.vertical()).max(0.),
    );
    node.rect = bounds;
    node.clip = node_clip(node, bounds, clip);
    node.divider = None;
    node.scroll = None;
    // Révélation : le contenu est disposé dans la hauteur naturelle, puis rogné à la
    // boîte animée (`node_clip` rogne toujours une révélation).
    let reveal = node.rules.as_ref().and_then(|r| r.reveal);
    if let Some(reveal) = reveal {
        node.rect.bottom = node.rect.top + reveal.natural.max(height(bounds));
    }
    let area = content_rect(node);
    node.rect = bounds;
    let size = SizeF {
        width: width(area),
        height: height(area),
    };
    let border = node.border();
    let positioned = node
        .rules
        .as_ref()
        .is_some_and(|r| r.position != Position::Static);
    let context = LayoutContext {
        viewport: context.viewport,
        parent: area,
        containing: if positioned {
            RectF::new(
                bounds.left + border.left,
                bounds.top + border.top,
                (width(bounds) - border.horizontal()).max(0.),
                (height(bounds) - border.vertical()).max(0.),
            )
        } else {
            context.containing
        },
    };
    // Limites relatives des enfants : pourcentages de la boîte de contenu.
    for child in &mut node.children {
        super::resolve_limits(child, size);
    }
    let mut out = super::position::take(node);
    let order = super::position::reorder(node);
    let result = (|| {
        match node.layout {
            Layout::Layers => super::layers::place(node, area, size, text, context)?,

            Layout::VirtualList => crate::ui::virtual_list::arrange(node, text)?,

            Layout::Scroll(axis) => super::scrolling::place(node, area, axis, text, context)?,

            Layout::Leaf => {
                node.intrinsic = node
                    .widget
                    .as_ref()
                    .map_or(Ok(SizeF::default()), |widget| widget.measure(size, text))?;
                if !node.children.is_empty() {
                    // Enfants en ligne (liens d'un texte enrichi) placés par le widget.
                    let boxes = match node.widget.as_mut() {
                        Some(widget) => widget.inline_boxes(area, text)?,
                        None => Vec::new(),
                    };
                    let clip = node.clip;
                    for (index, child) in node.children.iter_mut().enumerate() {
                        let rect = boxes
                            .get(index)
                            .copied()
                            .unwrap_or(RectF::new(area.left, area.top, 0., 0.));
                        arrange_in(child, rect, clip, text, context)?;
                    }
                }
            }

            Layout::Row | Layout::Column => super::linear::place(node, area, size, text, context)?,

            Layout::Grid { columns } => {
                let columns = super::grid::auto_columns(node, size.width, columns);
                super::grid::place(node, area, size, columns, text, context)?
            }

            Layout::Split(split) => super::split::place(node, area, size, split, text, context)?,
        }
        for (_, child) in &mut out {
            if super::position::hidden(child) {
                super::position::collapse(child, area.left, area.top);
            } else {
                super::position::placed(child, context, node.clip, text)?;
            }
        }
        Ok(())
    })();
    super::position::unorder(node, order);
    super::position::restore(node, out);
    if result.is_ok() {
        update_paint_order(node);
    }
    result
}
