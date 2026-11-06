//! Text field, on one line or several (`multiline`): application-owned value,
//! editing state kept per element ID, and native text geometry.
use crate::application::{Action, Handler, Message};
use crate::ui::{
    element::*,
    layout,
    scene::{DrawCommand, PointF, RectF, Scene},
    theme::{Color, Theme},
};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    ops::Range,
    sync::Arc,
};

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
use editor::trim_history;
pub use editor::{multi_line, single_line};
pub use geometry::{TextCaret, TextCluster, TextLine};
pub use menu::{DEFAULT_MENU_LABELS, EditMenuLabels};
pub(crate) use menu::{EditMenu, MenuAnchor, MenuResponse};
use rows::Rows;
pub use style::InputStyle;
pub use widget::TextInput;
use widget::paint_input_container;
pub use wrap::WrappedLine;
// Mise en page d'une ligne seule : servie aux mesureurs natifs (DirectWrite, Core Text,
// Pango).
#[cfg_attr(
    not(any(windows, target_os = "macos", target_os = "linux")),
    allow(unused_imports)
)]
pub(crate) use wrap::line_format;
pub(crate) use wrap::{lines_from_ranges, wrap_text};
/// Editing command sent by the backend to the focused [`TextInput`] (usually
/// from a keyboard shortcut or context menu).
#[derive(Clone, Copy)]
pub enum EditCommand {
    /// Select the whole text.
    SelectAll,
    /// Copy the selection to the clipboard.
    Copy,
    /// Copy the selection, then delete it (copy only when read-only).
    Cut,
    /// Request the clipboard text to insert it at the selection.
    Paste,
    /// Delete the selected text.
    DeleteSelection,
    /// Restore the previous edit.
    Undo,
    /// Reapply the last undone edit.
    Redo,
    /// Move the caret to the previous word boundary.
    WordLeft {
        /// Extend the selection instead of collapsing it.
        extend: bool,
    },
    /// Move the caret to the next word boundary.
    WordRight {
        /// Extend the selection instead of collapsing it.
        extend: bool,
    },
}
/// Clipboard operation the backend must perform for a text input.
#[derive(Clone)]
pub enum ClipboardRequest {
    /// Put `text` on the clipboard.
    Write {
        /// Selected text to copy.
        text: String,
        /// Whether the request comes from a cut (the selection is then removed).
        cut: bool,
    },
    /// Read the clipboard and deliver its text back as a paste.
    Read,
}
#[derive(Default)]
pub(crate) struct InputResponse {
    pub handled: bool,
    pub repaint: bool,
    pub change: Option<Message>,
    pub submit: Option<Message>,
    /// `on_key` du champ, émis avant l'effet de la touche.
    pub key: Option<Message>,
    pub clipboard: Option<ClipboardRequest>,
}
#[derive(Clone)]
struct Snapshot {
    text: String,
    anchor: usize,
    cursor: usize,
}
struct Composition {
    range: Range<usize>,
    value: Arc<str>,
    caret: usize,
}
/// Editing state of a text input, for backends (IME) and tests. Geometry is in
/// logical window coordinates; indices are UTF-8 byte offsets.
pub struct InputSnapshot {
    /// Current text, including edits not yet echoed back by the application.
    pub value: String,
    /// Selected byte range (empty at the caret).
    pub selection: Range<usize>,
    /// Whether an IME composition is in progress.
    pub composing: bool,
    /// Whether the field is read-only.
    pub read_only: bool,
    /// Caret rectangle.
    pub caret: RectF,
    /// Visible text area.
    pub bounds: RectF,
}
/// Caret and area of a text input, used to position IME windows. Geometry is in
/// logical window coordinates.
pub struct InputGeometry {
    /// Whether an IME composition is in progress.
    pub composing: bool,
    /// Whether the field is read-only.
    pub read_only: bool,
    /// Caret rectangle.
    pub caret: RectF,
    /// Visible text area.
    pub bounds: RectF,
}
/// Displayed text and geometry in logical window coordinates.
/// Indices are UTF-8 byte offsets, at boundaries measured by the backend.
#[derive(Clone)]
pub struct AccessibleText {
    /// Displayed text, including any IME composition.
    pub value: Arc<str>,
    /// Selected byte range (the composition caret while composing).
    pub selection: Range<usize>,
    /// Caret stops of the displayed text.
    pub carets: Vec<TextCaret>,
    /// Grapheme clusters with their rectangles.
    pub clusters: Vec<TextCluster>,
    /// Visible area of the text.
    pub clip: RectF,
    /// Whether the field accepts several lines (`<textarea>`).
    pub multiline: bool,
    /// Visual lines of the displayed text, in order (a single one for a single-line
    /// field). In a multiline field, a byte at an automatic line break is both the end
    /// of one line and the start of the next, so it appears twice in `carets`.
    pub lines: Vec<AccessibleLine>,
}
/// Visual line of an [`AccessibleText`], in logical window coordinates.
#[derive(Clone)]
pub struct AccessibleLine {
    /// Displayed bytes of the line, without its ending line feed.
    pub range: Range<usize>,
    /// Line rectangle (scrolled, not clipped).
    pub rect: RectF,
}
struct Editor {
    text: String,
    anchor: usize,
    cursor: usize,
    line: Option<TextLine>,
    display: Arc<str>,
    composition: Option<Composition>,
    scroll: f32,
    text_offset: f32,
    rect: RectF,
    clip: RectF,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    font: Option<crate::ui::styling::text::TextFormat>,
    reveal: Option<usize>,
    /// Champ multiligne : texte assaini en gardant les retours à la ligne.
    multiline: bool,
    /// Lignes visuelles d'un champ multiligne.
    rows: Option<Rows>,
    /// Défilement vertical d'un champ multiligne.
    scroll_y: f32,
    /// Le caret a bougé : la prochaine synchronisation le rend visible.
    follow: bool,
    /// Caret à une coupure automatique affiché en fin de ligne plutôt qu'au début de
    /// la suivante (Fin, clic après la fin d'une ligne).
    upstream: bool,
    /// Abscisse visée par Haut/Bas successifs (colonne visuelle).
    goal: Option<f32>,
}
/// Dernier appui dans un champ : les appuis rapprochés (`MULTI_CLICK`, à
/// `MULTI_CLICK_DISTANCE` près) comptent double puis triple.
#[derive(Clone, Copy)]
struct Click {
    id: ElementId,
    position: PointF,
    at: std::time::Instant,
    /// 1 : caret ; 2 : mot ; 3 : ligne.
    count: u8,
}
/// Délai entre deux appuis d'un même clic multiple.
const MULTI_CLICK: std::time::Duration = std::time::Duration::from_millis(500);
/// Déplacement toléré entre deux appuis d'un même clic multiple (unités logiques).
const MULTI_CLICK_DISTANCE: f32 = 4.;
pub(crate) struct TextInputs {
    states: HashMap<ElementId, Editor>,
    focused: Option<ElementId>,
    pub dragging: Option<ElementId>,
    active: bool,
    caret_visible: bool,
    revision: u64,
    click: Option<Click>,
}
impl Default for TextInputs {
    fn default() -> Self {
        Self {
            states: HashMap::new(),
            focused: None,
            dragging: None,
            active: true,
            caret_visible: true,
            revision: 0,
            click: None,
        }
    }
}
