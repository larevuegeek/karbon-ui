use super::*;
/// Gouttière réservée même quand le contenu tient : `overflow: scroll` ou
/// `scrollbar-gutter: stable`. Sinon, comme CSS (`overflow: auto`), elle n'apparaît
/// qu'avec le débordement.
pub(super) fn gutter_always(node: &Element, axis: Axis) -> bool {
    node.rules.as_ref().is_some_and(|r| {
        r.scrollbar.gutter_stable
            || match axis {
                Axis::Horizontal => r.overflow_x == Overflow::Scroll,
                Axis::Vertical => r.overflow_y == Overflow::Scroll,
            }
    })
}
/// Épaisseur de la barre d'une zone qui défile (aucune avec `scrollbar-width: none`).
pub(super) fn bar_thickness(node: &Element) -> f32 {
    node.rules
        .as_ref()
        .map_or(Default::default(), |r| r.scrollbar)
        .thickness()
}
/// Indique au contenu d'une zone qui défile si sa longueur le long du défilement est
/// définie : c'est le cas quand celle de la zone l'est (`h-full`, `h-64`, `flex-1` d'un
/// parent déjà résolu…). En CSS, la zone est elle-même le conteneur flex : une base en
/// pourcentage (`flex-1`) s'y résout contre la zone, et un élément `min-h-0` rétrécit
/// au lieu de pousser la zone en débordement. Une zone de longueur automatique laisse
/// la base valoir la taille du contenu.
pub(super) fn mark_content(node: &mut Element, axis: Axis) {
    let length = match axis {
        Axis::Horizontal => node.style.width,
        Axis::Vertical => node.style.height,
    };
    let definite = !matches!(length, Length::Auto | Length::MaxContent);
    let Some(content) = node.children.first_mut() else {
        return;
    };
    // Seulement le long de l'axe principal du contenu (colonne qui défile verticalement,
    // rangée qui défile horizontalement).
    let main = match axis {
        Axis::Horizontal => matches!(content.layout, Layout::Row),
        Axis::Vertical => matches!(content.layout, Layout::Column),
    };
    let definite = definite && main;
    if definite || content.rules.is_some() {
        content.layout_rules().definite_main = definite;
    }
}
pub(super) fn place(
    node: &mut Element,
    area: RectF,
    axis: Axis,
    text: &mut dyn TextMeasurer,
    context: LayoutContext,
) -> crate::Result<()> {
    mark_content(node, axis);
    let always = gutter_always(node, axis);
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
    // La gouttière n'est réservée que si le contenu déborde, comme les barres classiques
    // d'un navigateur, sauf avec `overflow: scroll` ou `scrollbar-gutter: stable`.
    let mut thickness = bar_thickness(node);
    let mut viewport = crate::ui::scroll::viewport(area, axis, thickness);
    let mut available = size(viewport);
    // Un contenu masqué ou positionné est hors flux (`position::take`) : rien à défiler.
    let mut content = |available: SizeF| match node.children.first_mut() {
        Some(content) => measure(content, available, text),
        None => Ok(SizeF::default()),
    };
    let desired = if thickness > 0. && !always {
        let full = crate::ui::scroll::viewport(area, axis, 0.);
        let natural = content(size(full))?;
        if overflows(natural, size(full)) {
            content(available)?
        } else {
            thickness = 0.;
            viewport = full;
            available = size(full);
            natural
        }
    } else {
        content(available)?
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
    let clip = intersect(viewport, node.clip);
    // Les descendants positionnés défilent avec le contenu : leur bloc conteneur est
    // décalé du défilement, comme `scroll::set_offset` les déplace ensuite.
    let (dx, dy) = match axis {
        Axis::Horizontal => (-metrics.offset, 0.),
        Axis::Vertical => (0., -metrics.offset),
    };
    let c = context.containing;
    let context = LayoutContext {
        containing: RectF::new(c.left + dx, c.top + dy, width(c), height(c)),
        ..context
    };
    if let Some(content) = node.children.first_mut() {
        arrange_in(content, bounds, clip, text, context)?;
    }

    Ok(())
}
