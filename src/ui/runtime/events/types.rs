use super::super::*;
/// Keyboard keys the runtime handles for navigation and editing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    /// Tab (focus navigation).
    Tab,
    /// Enter / Return.
    Enter,
    /// Space bar.
    Space,
    /// Left arrow.
    Left,
    /// Right arrow.
    Right,
    /// Up arrow.
    Up,
    /// Down arrow.
    Down,
    /// Home.
    Home,
    /// End.
    End,
    /// Page Up.
    PageUp,
    /// Page Down.
    PageDown,
    /// Escape.
    Escape,
    /// Backspace.
    Backspace,
    /// Delete.
    Delete,
}
/// Platform-independent input event sent by a backend to
/// [`UiRuntime::event`](crate::ui::runtime::UiRuntime::event). Positions are in logical units
/// relative to the window's client area.
#[derive(Clone)]
pub enum UiEvent {
    /// Text typed into the focused input (also used to paste clipboard contents).
    Text(std::sync::Arc<str>),
    /// Editing command (copy, paste, undo, word navigation…) for the focused input.
    Edit(EditCommand),
    /// An IME composition started.
    CompositionStart,
    /// The IME composition text changed.
    CompositionUpdate {
        /// Current composition string.
        value: std::sync::Arc<str>,
        /// Caret position inside `value`, in UTF-16 code units.
        cursor_utf16: usize,
    },
    /// The IME committed the given text.
    CompositionCommit(std::sync::Arc<str>),
    /// The IME composition ended.
    CompositionEnd,
    /// The window gained (`true`) or lost (`false`) keyboard focus.
    FocusChanged(bool),
    /// Caret blink tick for the focused text input.
    CaretTick,
    /// One-shot runtime wake-up; the backend passes its monotonic clock.
    Timer(Instant),
    /// Pointer moved.
    PointerMove(PointF),
    /// Primary button pressed.
    PointerDown(PointF),
    /// Primary button pressed with Shift held (extends a text selection).
    PointerDownExtended(PointF),
    /// Primary button released.
    PointerUp(PointF),
    /// Pointer left the window.
    PointerLeave,
    /// Context menu request; `None` = keyboard: anchor near the focused control.
    ContextMenu(Option<PointF>),
    /// Deltas in logical units; positive = right / down, fractions preserved.
    Scroll {
        /// Pointer position, used to find the scroll container.
        position: PointF,
        /// Horizontal delta.
        delta_x: f32,
        /// Vertical delta.
        delta_y: f32,
    },
    /// Key pressed.
    KeyDown {
        /// Pressed key.
        key: Key,
        /// Shift held.
        shift: bool,
        /// Auto-repeat from a held key.
        repeat: bool,
    },
    /// Key released.
    KeyUp(Key),
    /// Abort the current pointer gesture (capture lost, window deactivated…).
    Cancel,
}
/// Mouse cursor shape requested by the runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cursor {
    /// Default arrow.
    Arrow,
    /// Pointing hand, over clickable elements.
    Hand,
    /// Horizontal resize, over a vertical split divider.
    ResizeHorizontal,
    /// Vertical resize, over a horizontal split divider.
    ResizeVertical,
    /// Text I-beam, over text inputs.
    Text,
    /// Forbidden action (`cursor: not-allowed`).
    NotAllowed,
    /// Busy (`cursor: wait`).
    Wait,
    /// Busy in the background, still interactive (`cursor: progress`).
    Progress,
    /// Help available (`cursor: help`).
    Help,
    /// Move or drag (`cursor: move`, `grab`, `grabbing`).
    Move,
    /// Precise selection (`cursor: crosshair`).
    Crosshair,
    /// Diagonal resize from the top-left corner (`cursor: nwse-resize`).
    ResizeDiagonal,
    /// Diagonal resize from the top-right corner (`cursor: nesw-resize`).
    ResizeAntiDiagonal,
}
/// What the backend must do after [`UiRuntime::event`](crate::ui::runtime::UiRuntime::event).
#[derive(Default)]
pub struct EventResult {
    /// The UI consumed the event; the backend must not apply its default handling.
    pub handled: bool,
    /// A new frame must be prepared and painted.
    pub repaint: bool,
    /// Capture the pointer so the release is received even outside the window.
    pub capture: bool,
    /// Release a pointer capture taken earlier.
    pub release: bool,
    /// Clipboard read or write requested by a text input.
    pub clipboard: Option<ClipboardRequest>,
}
