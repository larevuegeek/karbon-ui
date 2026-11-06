//! Delayed textual help, independent of control layout and hit-testing.
use crate::ui::{
    element::*,
    layers::AnchoredLayer,
    layout,
    scene::{DrawCommand, PointF, RectF, Scene},
    theme::{Color, Theme},
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

mod config;
mod drawing;
mod state;
pub use config::{Tooltip, TooltipStyle};
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
struct Target {
    id: ElementId,
    tooltip: Tooltip,
    anchor: RectF,
    viewport: RectF,
}
fn same(a: RectF, b: RectF) -> bool {
    a.left == b.left && a.top == b.top && a.right == b.right && a.bottom == b.bottom
}
#[derive(Default)]
pub(crate) struct Tooltips {
    target: Option<Target>,
    due: Option<Instant>,
    visible: bool,
    bounds: Option<RectF>,
    suppressed: Option<ElementId>,
}
