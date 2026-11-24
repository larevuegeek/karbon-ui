//! Docked panels (KUI-023): a [`DockTree`] of groups of tabs split side by side or
//! stacked, shown by a [`DockArea`]. Drag a tab onto another group to dock it on one of
//! its sides or among its tabs; drag a divider to share the space. The layout is data of
//! the application, saved and read back as JSON ([`DockTree::to_json`],
//! [`DockTree::from_json`]). Mouse gestures use [`ui::drag`](crate::ui::drag).
mod json;
mod state;
#[cfg(test)]
mod tests;
mod tree;
mod view;

pub use state::{DockEvent, DockMsg, DockReply, DockState};
pub use tree::{DockNode, DockSide, DockTree};
pub use view::DockArea;
