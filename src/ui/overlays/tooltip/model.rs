use super::*;
pub(super) use crate::ui::{
    element::*,
    layers::AnchoredLayer,
    layout,
    scene::{DrawCommand, PointF, RectF, Scene},
    theme::{Color, Theme},
};
pub(super) use std::{
    sync::Arc,
    time::{Duration, Instant},
};
/// Snapshot of the visible tooltip, for accessibility and tests.
#[derive(Clone)]
pub struct TooltipInfo {
    /// Element the tooltip describes.
    pub owner: ElementId,
    /// Tooltip text.
    pub text: Arc<str>,
    /// Tooltip bounds in logical window coordinates.
    pub bounds: RectF,
}
#[derive(Clone)]
pub(super) struct Target {
    pub(super) id: ElementId,
    pub(super) tooltip: Tooltip,
    pub(super) anchor: RectF,
    pub(super) viewport: RectF,
}
pub(super) fn same(a: RectF, b: RectF) -> bool {
    a.left == b.left && a.top == b.top && a.right == b.right && a.bottom == b.bottom
}
#[derive(Default)]
pub(crate) struct Tooltips {
    pub(super) target: Option<Target>,
    pub(super) due: Option<Instant>,
    pub(super) visible: bool,
    pub(super) bounds: Option<RectF>,
    pub(super) suppressed: Option<ElementId>,
}
