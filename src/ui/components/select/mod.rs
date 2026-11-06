//! Non-editable choice with a list shown above the content; the application
//! value is identified by key.
use crate::application::{Handler, Message};
use crate::ui::{
    element::*,
    layers::AnchoredLayer,
    layout,
    runtime::{Cursor, Key, UiEvent},
    scene::{DrawCommand, PointF, RectF, Scene},
    theme::{Color, Theme},
};
use std::{
    collections::HashSet,
    sync::Arc,
    time::{Duration, Instant},
};

const ROW_HEIGHT: f32 = 32.;
mod drawing;
mod events;
mod navigation;
mod option;
mod popup;
mod state;
mod style;
mod widget;
pub use option::SelectOption;
pub use style::SelectStyle;
pub use widget::Select;
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
struct OpenSelect {
    row_height: f32,
    id: ElementId,
    options: Arc<[SelectOption]>,
    selected: Option<u64>,
    anchor: RectF,
    clip: RectF,
    viewport: RectF,
    layer: AnchoredLayer,
    active: Option<u64>,
    hover: Option<u64>,
    pressed: Option<u64>,
    first: usize,
    drag_scroll: Option<f32>,
    query: String,
    typed: Option<Instant>,
    visible_items: usize,
}
fn control(root: &Element, id: ElementId) -> Option<(&Select, RectF, RectF)> {
    let n = root.find(id)?;
    Some((
        n.widget.as_ref()?.select()?,
        layout::content_rect(n),
        n.clip,
    ))
}
pub(super) fn same_rect(a: RectF, b: RectF) -> bool {
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
    open: Option<OpenSelect>,
    captured: bool,
    dismiss_capture: bool,
    pointer: Option<PointF>,
}
