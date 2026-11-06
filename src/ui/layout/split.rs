use super::*;
pub(super) fn place(
    node: &mut Element,
    area: RectF,
    size: SizeF,
    split: Split,
    text: &mut dyn TextMeasurer,
    context: LayoutContext,
) -> crate::Result<()> {
    let clip = child_clip(node, area);
    let horizontal = split.axis == Axis::Horizontal;
    let available = (if horizontal { size.width } else { size.height } - DIVIDER_SIZE).max(0.);
    let first = split_extent(available, split.fraction, split.min_first, split.min_second);
    let second = (available - first).max(0.);
    let (a, bar, b) = if horizontal {
        (
            RectF::new(area.left, area.top, first, size.height),
            RectF::new(
                area.left + first,
                area.top,
                DIVIDER_SIZE.min(size.width),
                size.height,
            ),
            RectF::new(
                area.left + first + DIVIDER_SIZE,
                area.top,
                second,
                size.height,
            ),
        )
    } else {
        (
            RectF::new(area.left, area.top, size.width, first),
            RectF::new(
                area.left,
                area.top + first,
                size.width,
                DIVIDER_SIZE.min(size.height),
            ),
            RectF::new(
                area.left,
                area.top + first + DIVIDER_SIZE,
                size.width,
                second,
            ),
        )
    };
    node.divider = Some(bar);
    arrange_in(&mut node.children[0], a, clip, text, context)?;
    arrange_in(&mut node.children[1], b, clip, text, context)?;

    Ok(())
}
/// Size of the first pane of a split: `available * fraction`, clamped so both panes respect
/// their minimums. When the minimums do not fit, `available` is shared in proportion to them.
pub fn split_extent(available: f32, fraction: f32, min_first: f32, min_second: f32) -> f32 {
    if min_first + min_second > available {
        if min_first + min_second > 0. {
            available * min_first / (min_first + min_second)
        } else {
            0.
        }
    } else {
        (available * fraction).clamp(min_first, available - min_second)
    }
}
