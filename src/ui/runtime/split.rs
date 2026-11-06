use super::*;
impl UiRuntime {
    pub(super) fn resize_split(&mut self, id: ElementId, fraction: f32) {
        if let Some(node) = self.root.as_mut().and_then(|r| r.find_mut(id))
            && let Layout::Split(mut split) = node.layout
        {
            let bounds = layout::content_rect(node);
            let available = (if split.axis == Axis::Horizontal {
                layout::width(bounds)
            } else {
                layout::height(bounds)
            } - layout::DIVIDER_SIZE)
                .max(0.);
            let extent = layout::split_extent(
                available,
                fraction.clamp(0., 1.),
                split.min_first,
                split.min_second,
            );
            let fraction = if available > 0. {
                extent / available
            } else {
                0.
            };
            if (fraction - split.fraction).abs() > 0.00001 {
                split.fraction = fraction;
                node.layout = Layout::Split(split);
                self.components.splits.fractions.insert(id, fraction);
                self.invalidation.relayout = true;
                self.invalidation.repaint = true;
            }
        }
    }
}
