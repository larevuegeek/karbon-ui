use super::*;
pub(super) fn place(
    node: &mut Element,
    area: RectF,
    size: SizeF,
    text: &mut dyn TextMeasurer,
    context: LayoutContext,
) -> crate::Result<()> {
    for index in 0..node.children.len() {
        let (previous, remaining) = node.children.split_at_mut(index);
        let child = &mut remaining[0];
        let bounds = if index == 0 {
            area
        } else {
            let config = crate::ui::popover::config(child).map(|p| (p.anchor, p.placement));
            let popup = config.map(|c| c.0);
            let anchor = popup.and_then(|p| match p {
                crate::ui::popover::Anchor::Point(p) => Some(RectF::new(p.x, p.y, 0., 0.)),
                crate::ui::popover::Anchor::Element(id) => previous
                    .iter()
                    .rev()
                    .find_map(|n| n.find(id))
                    .map(|n| intersect(n.rect, n.clip))
                    .filter(|r| width(*r) > 0. && height(*r) > 0.),
            });
            let inset: f32 = if popup.is_some() { 8. } else { 16. };
            let viewport = RectF::new(
                area.left + inset.min(size.width / 2.),
                area.top + inset.min(size.height / 2.),
                (size.width - inset * 2.).max(0.),
                (size.height - inset * 2.).max(0.),
            );
            let available = SizeF {
                width: width(viewport),
                height: height(viewport),
            };
            let desired = measure(child, available, text)?;
            // `match-width` : au moins la largeur de l'ancre (hauteur pour un côté).
            let matched = config
                .filter(|c| c.1.match_anchor)
                .zip(anchor)
                .map(|(c, a)| match c.1.side {
                    crate::ui::popover::Side::Top | crate::ui::popover::Side::Bottom => {
                        (width(a), 0.)
                    }
                    _ => (0., height(a)),
                })
                .unwrap_or((0., 0.));
            let width = desired.width.max(matched.0).min(available.width);
            // Le texte peut prendre plus de lignes après réduction de la largeur.
            let old_width = child.style.width;
            let old_min = child.style.min_width;
            let border_width = (width - child.style.margin.horizontal()).max(0.);
            child.style.width = Length::Px(border_width);
            child.style.min_width = old_min.min(border_width);
            let wrapped = measure(child, available, text);
            child.style.width = old_width;
            child.style.min_width = old_min;
            let desired = wrapped?;
            let height = desired.height.max(matched.1).min(available.height);
            if let Some((_, placement)) = config {
                anchor.map_or(RectF::new(area.left, area.top, 0., 0.), |anchor| {
                    crate::ui::layers::AnchoredLayer::place_with(
                        anchor,
                        SizeF { width, height },
                        viewport,
                        placement,
                    )
                    .bounds
                })
            } else {
                let top = crate::ui::modal::config(child)
                    .is_some_and(|m| m.placement == crate::ui::modal::ModalPlacement::Top);
                RectF::new(
                    area.left + (size.width - width) / 2.,
                    if top {
                        area.top
                    } else {
                        area.top + (size.height - height) / 2.
                    },
                    width,
                    height,
                )
            }
        };
        arrange_in(child, bounds, node.clip, text, context)?;
    }

    Ok(())
}
