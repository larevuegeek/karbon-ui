use super::*;
pub(super) fn place(
    node: &mut Element,
    area: RectF,
    size: SizeF,
    text: &mut dyn TextMeasurer,
    context: LayoutContext,
) -> crate::Result<()> {
    let clip = child_clip(node, area);
    if node
        .rules
        .as_ref()
        .is_some_and(|r| r.css_flex || r.flex_wrap != FlexWrap::NoWrap || r.flex_reverse)
        || node.children.iter().any(|c| {
            c.rules.as_ref().is_some_and(|r| {
                r.margin_auto != AutoMargins::default()
                    || r.flex_grow.is_some()
                    || r.flex_shrink.is_some()
                    || r.flex_basis.is_some()
                    || r.align_self.is_some()
            })
        })
    {
        return super::flex::place(node, area, size, text, context);
    }
    let row = matches!(node.layout, Layout::Row);
    let main_available = if row { size.width } else { size.height };
    let gap = if row {
        node.style.horizontal_gap()
    } else {
        node.style.vertical_gap()
    };
    let mut lengths = Vec::with_capacity(node.children.len());
    let mut weights = Vec::with_capacity(node.children.len());
    let mut maxima = Vec::with_capacity(node.children.len());
    for child in &mut node.children {
        let desired = measure(child, size, text)?;
        let length = if row {
            child.style.width
        } else {
            child.style.height
        };
        let (min, max) = limits(child, row);
        let weight = if let Length::Fill(weight) = length {
            weight
        } else {
            0.
        };
        let margin = if row {
            child.style.margin.horizontal()
        } else {
            child.style.margin.vertical()
        };
        lengths.push(if weight > 0. {
            min + margin
        } else {
            if row { desired.width } else { desired.height }
        });
        weights.push(weight);
        maxima.push(max + margin);
    }
    distribute(
        &mut lengths,
        &weights,
        &maxima,
        (main_available - gap * node.children.len().saturating_sub(1) as f32).max(0.),
    );
    let used = lengths.iter().sum::<f32>() + gap * node.children.len().saturating_sub(1) as f32;
    let extra = (main_available - used).max(0.);
    let (offset, gap) = match node.style.justify {
        Justify::Start => (0., gap),
        Justify::Center => (extra / 2., gap),
        Justify::End => (extra, gap),
        Justify::SpaceBetween => (
            0.,
            gap + if node.children.len() > 1 {
                extra / (node.children.len() - 1) as f32
            } else {
                0.
            },
        ),
    };
    let mut cursor = if row { area.left } else { area.top } + offset;
    for (child, main) in node.children.iter_mut().zip(lengths) {
        let available = if row {
            SizeF {
                width: main,
                height: size.height,
            }
        } else {
            SizeF {
                width: size.width,
                height: main,
            }
        };
        let desired = super::measure::measure_main(child, available, row, text)?;
        let cross_available = if row { size.height } else { size.width };
        let cross_length = if row {
            child.style.height
        } else {
            child.style.width
        };
        // Une boîte en ligne (`inline-block`) garde sa largeur naturelle dans un bloc.
        let inline = !row && child.rules.as_ref().is_some_and(|r| r.inline_level);
        let cross = if matches!(node.style.align, Align::Stretch)
            && matches!(cross_length, Length::Auto | Length::Fill(_))
            && !inline
        {
            let margin = if row {
                child.style.margin.vertical()
            } else {
                child.style.margin.horizontal()
            };
            let (min, max) = limits(child, !row);
            cross_available.clamp(min + margin, max + margin)
        } else {
            if row { desired.height } else { desired.width }
        };
        let cross_offset = match node.style.align {
            Align::Center => (cross_available - cross).max(0.) / 2.,
            Align::End => (cross_available - cross).max(0.),
            _ => 0.,
        };
        let rect = if row {
            RectF::new(cursor, area.top + cross_offset, main, cross)
        } else {
            RectF::new(area.left + cross_offset, cursor, cross, main)
        };
        arrange_in(child, rect, clip, text, context)?;
        cursor += main + gap;
    }

    Ok(())
}
fn distribute(lengths: &mut [f32], weights: &[f32], maxima: &[f32], available: f32) {
    for _ in 0..=lengths.len() {
        let free = available - lengths.iter().sum::<f32>();
        if free <= 0.001 {
            break;
        }
        let total: f32 = weights
            .iter()
            .enumerate()
            .filter(|(i, w)| **w > 0. && lengths[*i] < maxima[*i] - 0.001)
            .map(|(_, w)| *w)
            .sum();
        if total <= 0. {
            break;
        }
        for i in 0..lengths.len() {
            if weights[i] > 0. && lengths[i] < maxima[i] {
                lengths[i] = (lengths[i] + free * weights[i] / total).min(maxima[i]);
            }
        }
    }
}
