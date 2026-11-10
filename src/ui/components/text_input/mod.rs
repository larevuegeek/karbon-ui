//! Text field, on one line or several (`multiline`): application-owned value,
//! editing state kept per element ID, and native text geometry.
mod accessibility;
mod drawing;
mod editor;
mod events;
mod geometry;
mod menu;
mod multiline;
mod multiline_paint;
mod pointer;
mod rows;
mod state;
mod style;
mod widget;
mod wrap;
pub use editor::{multi_line, single_line};
pub use geometry::{TextCaret, TextCluster, TextLine};
pub use menu::{DEFAULT_MENU_LABELS, EditMenuLabels};
pub(crate) use menu::{EditMenu, MenuAnchor, MenuResponse};
pub use style::InputStyle;
pub use widget::TextInput;
pub use wrap::WrappedLine;
// Mise en page d'une ligne seule : servie aux mesureurs natifs (DirectWrite, Core Text,
// Pango).
#[cfg_attr(
    not(any(windows, target_os = "macos", target_os = "linux")),
    allow(unused_imports)
)]
pub(crate) use wrap::line_format;
pub(crate) use wrap::{lines_from_ranges, wrap_text};
mod model;
pub use model::*;
