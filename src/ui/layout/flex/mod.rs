//! Flex lines are planned once, then sized with iterative min/max freezing.
use super::*;
mod lines;
use lines::{axis, distribute, plan, sized};
pub(super) fn natural(
    node: &mut Element,
    size: SizeF,
    text: &mut dyn TextMeasurer,
) -> crate::Result<SizeF> {
    let row = matches!(node.layout, Layout::Row);
    let mut lines = plan(node, size, text)?;
    let gap = if row {
        node.style.horizontal_gap()
    } else {
        node.style.vertical_gap()
    };
    let cross_gap = if row {
        node.style.vertical_gap()
    } else {
        node.style.horizontal_gap()
    };
    let main = lines
        .iter()
        .map(|l| {
            l.items.iter().map(|i| i.main).sum::<f32>()
                + gap * l.items.len().saturating_sub(1) as f32
        })
        .fold(0., f32::max);
    // Text cross sizes depend on the resolved main allocation, not the basis.
    // In particular basis:0 must not wrap a whole paragraph into a zero-width box.
    for line in &mut lines {
        distribute(
            line,
            axis(size, row).0 - gap * line.items.len().saturating_sub(1) as f32,
        );
        line.cross = 0.;
        for item in &mut line.items {
            let desired = super::measure::measure_main(
                &mut node.children[item.index],
                sized(item.main, axis(size, row).1, row),
                row,
                text,
            )?;
            item.cross = axis(desired, row).1;
            line.cross = line.cross.max(item.cross);
        }
    }
    let cross = lines.iter().map(|l| l.cross).sum::<f32>()
        + cross_gap * lines.len().saturating_sub(1) as f32;
    Ok(sized(main, cross, row))
}
/// Distance approchée entre le haut de la boîte de marge et la première ligne de base :
/// texte d'une feuille (taille et hauteur de ligne calculées), sinon premier enfant
/// dans le flux ; sans texte, le bas de la boîte (comme CSS).
fn first_baseline(node: &Element) -> f32 {
    let edges = box_edges(node);
    let top = node.style.margin.top + edges.top;
    let typography = node.styles.as_ref().map(|s| s.computed.text);
    if matches!(node.layout, Layout::Leaf) {
        if let Some(widget) = &node.widget
            && matches!(
                widget.semantics().role,
                Role::Text | Role::Heading | Role::Link
            )
        {
            let size = typography.and_then(|t| t.size).unwrap_or(16.);
            let line = typography.and_then(|t| t.line(size)).unwrap_or(size * 1.25);
            return top + (line - size) / 2. + size * 0.8;
        }
        return node.style.margin.top
            + height(node.rect).max(node.intrinsic.height + edges.vertical());
    }
    node.children
        .iter()
        .find(|c| !super::position::hidden(c) && !super::position::out_of_flow(c))
        .map_or(
            node.style.margin.top + height(node.rect).max(edges.vertical()),
            |c| top + first_baseline(c),
        )
}
pub(super) fn place(
    node: &mut Element,
    area: RectF,
    size: SizeF,
    text: &mut dyn TextMeasurer,
    context: LayoutContext,
) -> crate::Result<()> {
    let clip = child_clip(node, area);
    let row = matches!(node.layout, Layout::Row);
    let (available, cross_available) = axis(size, row);
    let gap = if row {
        node.style.horizontal_gap()
    } else {
        node.style.vertical_gap()
    };
    let cross_gap = if row {
        node.style.vertical_gap()
    } else {
        node.style.horizontal_gap()
    };
    let mut lines = plan(node, size, text)?;
    let wrap = node
        .rules
        .as_ref()
        .is_some_and(|r| r.flex_wrap != FlexWrap::NoWrap);
    let reverse = node.rules.as_ref().is_some_and(|r| r.flex_reverse);
    let wrap_reverse = node
        .rules
        .as_ref()
        .is_some_and(|r| r.flex_wrap == FlexWrap::WrapReverse);
    if !wrap {
        for l in &mut lines {
            l.cross = cross_available;
        }
    }
    for line in &mut lines {
        let target = available - gap * line.items.len().saturating_sub(1) as f32;
        distribute(line, target);
        if wrap {
            line.cross = 0.;
            for item in &mut line.items {
                let child = &mut node.children[item.index];
                let desired = super::measure::measure_main(
                    child,
                    sized(item.main, cross_available, row),
                    row,
                    text,
                )?;
                item.cross = axis(desired, row).1;
                line.cross = line.cross.max(item.cross);
            }
        }
    }
    let mut cross_cursor = 0.;
    for line in lines {
        // align-items: baseline (rangée) : premières lignes de base alignées.
        let baseline_of = |child: &Element| {
            child
                .rules
                .as_ref()
                .and_then(|r| r.align_self)
                .unwrap_or(node.style.align)
                == Align::Baseline
                && row
        };
        let line_baseline = line
            .items
            .iter()
            .map(|i| &node.children[i.index])
            .filter(|c| baseline_of(c))
            .map(first_baseline)
            .fold(None, |a: Option<f32>, b| Some(a.map_or(b, |a| a.max(b))));
        let free = (available
            - line.items.iter().map(|i| i.main).sum::<f32>()
            - gap * line.items.len().saturating_sub(1) as f32)
            .max(0.);
        let auto_count = line
            .items
            .iter()
            .map(|i| node.children[i.index].auto_margins().count(row))
            .sum::<usize>();
        let share = if auto_count > 0 {
            free / auto_count as f32
        } else {
            0.
        };
        let extra = if auto_count > 0 { 0. } else { free };
        let (mut cursor, gap) = match node.style.justify {
            Justify::Start => (0., gap),
            Justify::Center => (extra / 2., gap),
            Justify::End => (extra, gap),
            Justify::SpaceBetween => (
                0.,
                gap + if line.items.len() > 1 {
                    extra / (line.items.len() - 1) as f32
                } else {
                    0.
                },
            ),
        };
        for item in line.items {
            let child = &mut node.children[item.index];
            let original = child.style.margin;
            let auto = child.auto_margins();
            let mut margin = auto.add(original, row, share);
            let main = item.main + auto.count(row) as f32 * share;
            let desired =
                super::measure::measure_main(child, sized(item.main, line.cross, row), row, text)?;
            let cross_length = if row {
                child.style.height
            } else {
                child.style.width
            };
            let align = child
                .rules
                .as_ref()
                .and_then(|r| r.align_self)
                .unwrap_or(node.style.align);
            let (min, max) = limits(child, !row);
            let cm = if row {
                margin.vertical()
            } else {
                margin.horizontal()
            };
            let cross = if matches!(align, Align::Stretch)
                && auto.count(!row) == 0
                && matches!(cross_length, Length::Auto | Length::Fill(_))
            {
                line.cross.clamp(min + cm, max + cm)
            } else {
                axis(desired, row).1
            };
            let cross_free = (line.cross - cross).max(0.);
            let (cross, cross_offset) = if auto.count(!row) > 0 {
                margin = auto.add(margin, !row, cross_free / auto.count(!row) as f32);
                (cross + cross_free, 0.)
            } else {
                (
                    cross,
                    match align {
                        Align::Center => cross_free / 2.,
                        Align::End => cross_free,
                        Align::Baseline if row => {
                            line_baseline.map_or(0., |b| (b - first_baseline(child)).max(0.))
                        }
                        _ => 0.,
                    },
                )
            };
            let line_start = if wrap_reverse {
                cross_available - cross_cursor - line.cross
            } else {
                cross_cursor
            };
            let cross_offset = if wrap_reverse {
                line.cross - cross - cross_offset
            } else {
                cross_offset
            };
            let main_pos = if reverse {
                available - cursor - main
            } else {
                cursor
            };
            let rect = if row {
                RectF::new(
                    area.left + main_pos,
                    area.top + line_start + cross_offset,
                    main,
                    cross,
                )
            } else {
                RectF::new(
                    area.left + line_start + cross_offset,
                    area.top + main_pos,
                    cross,
                    main,
                )
            };
            child.style.margin = margin;
            let result = arrange_in(child, rect, clip, text, context);
            child.style.margin = original;
            result?;
            cursor += main + gap;
        }
        cross_cursor += line.cross + cross_gap;
    }
    Ok(())
}
