//! In-app drag and drop: stable identities, compatible groups and before/after drops.
use crate::application::Handler;
use crate::ui::element::Axis;

/// Identity of a draggable element, attached with
/// [`Element::draggable`](crate::ui::element::Element::draggable).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DragSource {
    /// Group shared with the [`DropTarget`]s that accept this source.
    pub group: u64,
    /// Application key of the dragged item, stable across rebuilds.
    pub key: u64,
}
impl DragSource {
    /// Source identified by `key` within `group`.
    pub fn new(group: u64, key: u64) -> Self {
        Self { group, key }
    }
}
impl From<(u64, u64)> for DragSource {
    /// `(group, key)`.
    fn from((group, key): (u64, u64)) -> Self {
        Self::new(group, key)
    }
}
/// Element that accepts drops of [`DragSource`]s from the same group, attached with
/// [`Element::drop_target`](crate::ui::element::Element::drop_target).
///
/// Dropping a source onto the target with the same key is ignored.
#[derive(Debug, Clone)]
pub struct DropTarget {
    /// Group of the sources this target accepts.
    pub group: u64,
    /// Application key of the target item.
    pub key: u64,
    pub(crate) on_drop: Handler<DropEvent>,
    /// Axis used to decide between [`DropPosition::Before`] and [`DropPosition::After`]
    /// (vertical by default).
    pub axis: Axis,
}
impl DropTarget {
    /// Accept sources of `group`; a validated drop is turned into a message by
    /// `on_drop`, for example `Msg::Reorder` with `Reorder(DropEvent)`.
    pub fn new<M: 'static>(
        group: u64,
        key: u64,
        on_drop: impl Fn(DropEvent) -> M + 'static,
    ) -> Self {
        Self {
            group,
            key,
            on_drop: Handler::new(on_drop),
            axis: Axis::Vertical,
        }
    }
    /// Split the target along `axis`: the first half drops before, the second half after.
    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }
}
/// Where the source lands relative to the target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropPosition {
    /// Above (vertical axis) or left of (horizontal axis) the target.
    Before,
    /// Below (vertical axis) or right of (horizontal axis) the target.
    After,
}
/// Completed drop, passed to the target's `on_drop` handler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DropEvent {
    /// Group shared by the source and the target.
    pub group: u64,
    /// Key of the dragged [`DragSource`].
    pub source: u64,
    /// Key of the [`DropTarget`] that received the drop.
    pub target: u64,
    /// Side of the target where the source was dropped.
    pub position: DropPosition,
}

mod interaction;
pub(super) use interaction::State;
