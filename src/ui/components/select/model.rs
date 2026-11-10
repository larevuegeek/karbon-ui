use super::*;
pub(super) use crate::application::{Handler, Message};
pub(super) use crate::ui::{
    element::*,
    layers::AnchoredLayer,
    layout,
    runtime::{Cursor, Key, UiEvent},
    scene::{DrawCommand, PointF, RectF, Scene},
    theme::{Color, Theme},
};
pub(super) use std::{
    collections::HashSet,
    sync::Arc,
    time::{Duration, Instant},
};

pub(super) const ROW_HEIGHT: f32 = 32.;
/// Accessibility snapshot of one [`Select`] option.
#[derive(Clone)]
pub struct SelectOptionInfo {
    /// Application key of the option.
    pub id: u64,
    /// Displayed label.
    pub label: Arc<str>,
    /// Whether both the select and the option accept input.
    pub enabled: bool,
    /// Whether this is the currently selected option.
    pub selected: bool,
    /// Whether this option is the active (keyboard-highlighted) row of the open list.
    pub focused: bool,
    /// Row bounds in logical window coordinates; zero while the list is closed.
    pub bounds: RectF,
    /// Visible part of the row inside the list; zero while the list is closed.
    pub clip: RectF,
}
/// Accessibility snapshot of a [`Select`] and its list.
#[derive(Clone)]
pub struct SelectInfo {
    /// Whether the option list is open.
    pub expanded: bool,
    /// Bounds of the open list in logical window coordinates.
    pub popup: Option<RectF>,
    /// All options while open; only the selected option while closed.
    pub options: Vec<SelectOptionInfo>,
}
pub(super) struct OpenSelect {
    pub(super) row_height: f32,
    pub(super) id: ElementId,
    pub(super) options: Arc<[SelectOption]>,
    pub(super) selected: Option<u64>,
    pub(super) anchor: RectF,
    pub(super) clip: RectF,
    pub(super) viewport: RectF,
    pub(super) layer: AnchoredLayer,
    pub(super) active: Option<u64>,
    pub(super) hover: Option<u64>,
    pub(super) pressed: Option<u64>,
    pub(super) first: usize,
    pub(super) drag_scroll: Option<f32>,
    pub(super) query: String,
    pub(super) typed: Option<Instant>,
    pub(super) visible_items: usize,
}
pub(super) fn control(root: &Element, id: ElementId) -> Option<(&Select, RectF, RectF)> {
    let n = root.find(id)?;
    Some((
        n.widget.as_ref()?.select()?,
        layout::content_rect(n),
        n.clip,
    ))
}
pub(crate) fn same_rect(a: RectF, b: RectF) -> bool {
    a.left == b.left && a.top == b.top && a.right == b.right && a.bottom == b.bottom
}
#[derive(Default)]
pub(crate) struct SelectResponse {
    pub handled: bool,
    pub repaint: bool,
    pub capture: bool,
    pub release: bool,
    pub focus: Option<ElementId>,
    pub change: Option<Message>,
}
#[derive(Default)]
pub(crate) struct Selects {
    pub(super) open: Option<OpenSelect>,
    pub(super) captured: bool,
    pub(super) dismiss_capture: bool,
    pub(super) pointer: Option<PointF>,
}
