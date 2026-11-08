use super::*;
pub(super) fn place(
    node: &mut Element,
    area: RectF,
    axis: Axis,
    text: &mut dyn TextMeasurer,
    context: LayoutContext,
) -> crate::Result<()> {
    let bar = node
        .rules
        .as_ref()
        .map_or(Default::default(), |r| r.scrollbar);
    let overflows = |desired: SizeF, available: SizeF| match axis {
        Axis::Horizontal => desired.width > available.width + 0.01,
        Axis::Vertical => desired.height > available.height + 0.01,
    };
    // Comme CSS : la zone visible est la boîte de padding (la barre se loge entre la
    // bordure et le padding) ; le padding défile avec le contenu, il n'en rogne rien.
    let pad = node.style.padding;
    let area = RectF::new(
        area.left - pad.left,
        area.top - pad.top,
        width(area) + pad.horizontal(),
        height(area) + pad.vertical(),
    );
    let size = |viewport: RectF| SizeF {
        width: (width(viewport) - pad.horizontal()).max(0.),
        height: (height(viewport) - pad.vertical()).max(0.),
    };
    // Barre fine : la gouttière n'est réservée que si le contenu déborde (comme les
    // barres classiques d'un navigateur), sauf avec `scrollbar-gutter: stable`.
    let mut thickness = bar.thickness();
    let mut viewport = crate::ui::scroll::viewport(area, axis, thickness);
    let mut available = size(viewport);
    let desired = if bar.width == ScrollbarWidth::Thin && !bar.gutter_stable {
        let full = crate::ui::scroll::viewport(area, axis, 0.);
        let content = measure(&mut node.children[0], size(full), text)?;
        if overflows(content, size(full)) {
            measure(&mut node.children[0], available, text)?
        } else {
            thickness = 0.;
            viewport = full;
            available = size(full);
            content
        }
    } else {
        measure(&mut node.children[0], available, text)?
    };
    let extent = match axis {
        Axis::Horizontal => desired.width.max(available.width) + pad.horizontal(),
        Axis::Vertical => desired.height.max(available.height) + pad.vertical(),
    };
    let metrics = crate::ui::scroll::metrics(area, axis, extent, node.scroll_offset, thickness);
    node.scroll_offset = metrics.offset;
    node.scroll = Some(metrics);
    let bounds = match axis {
        Axis::Horizontal => RectF::new(
            viewport.left + pad.left - metrics.offset,
            viewport.top + pad.top,
            extent - pad.horizontal(),
            available.height,
        ),
        Axis::Vertical => RectF::new(
            viewport.left + pad.left,
            viewport.top + pad.top - metrics.offset,
            available.width,
            extent - pad.vertical(),
        ),
    };
    arrange_in(
        &mut node.children[0],
        bounds,
        intersect(viewport, node.clip),
        text,
        context,
    )?;

    Ok(())
}
