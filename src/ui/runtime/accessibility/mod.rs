use super::*;
/// Snapshot of one node of the accessibility tree, as exposed to screen readers.
///
/// Produced by [`UiRuntime::accessibility`](crate::ui::runtime::UiRuntime::accessibility);
/// bounds are in logical units relative to the window's client area.
#[derive(Clone)]
pub struct AccessibleElement {
    /// Element identity.
    pub id: ElementId,
    /// Closest accessible ancestor, if any.
    pub parent: Option<ElementId>,
    /// Semantic role.
    pub role: Role,
    /// Accessible name (label).
    pub name: String,
    /// The element accepts interaction.
    pub enabled: bool,
    /// The element has keyboard focus.
    pub focused: bool,
    /// Full bounds, including any running animation.
    pub bounds: RectF,
    /// Bounds clipped by scroll containers and overflow.
    pub clipped_bounds: RectF,
    /// The element is in the focus order.
    pub focusable: bool,
    /// Fully clipped away (scrolled out of view).
    pub offscreen: bool,
    /// Supports [`AccessibilityAction::Invoke`].
    pub invokable: bool,
    /// Current text value of a text input.
    pub value: Option<String>,
    /// Checked state of a checkbox, switch or radio button.
    pub choice: Option<ChoiceState>,
    /// Radio group containing this radio button.
    pub selection_container: Option<ElementId>,
    /// Text and selection details of a text input.
    pub text: Option<AccessibleText>,
    /// Value range of a slider.
    pub range: Option<RangeInfo>,
    /// State of a select (expanded, options, selection).
    pub select: Option<SelectInfo>,
    /// The value cannot be changed by the user.
    pub read_only: bool,
    /// Help text: input or select placeholder, then tooltip text; empty if none.
    pub help: String,
    /// Declared expanded state of a popup trigger (`aria-expanded`).
    pub expanded: Option<bool>,
}
/// Request from an assistive technology, applied with
/// [`UiRuntime::accessibility_action`](crate::ui::runtime::UiRuntime::accessibility_action).
pub enum AccessibilityAction {
    /// Move keyboard focus to the element.
    Focus(ElementId),
    /// Activate a button-like element.
    Invoke(ElementId),
    /// Toggle a checkbox or switch.
    Toggle(ElementId),
    /// Select a radio button.
    SelectChoice(ElementId),
    /// Open or close a select's popup.
    ExpandSelect {
        /// Select element.
        id: ElementId,
        /// `true` to open, `false` to close.
        expanded: bool,
    },
    /// Choose a select option, as a click would.
    ChooseOption {
        /// Select element.
        id: ElementId,
        /// Option identifier.
        option: u64,
    },
    /// Highlight a select option without choosing it.
    HighlightOption {
        /// Select element.
        id: ElementId,
        /// Option identifier.
        option: u64,
    },
    /// Set a slider value; must lie within the slider's range.
    SetNumber {
        /// Slider element.
        id: ElementId,
        /// New value.
        value: f64,
    },
    /// Replace the text of a text input.
    SetValue {
        /// Text input element.
        id: ElementId,
        /// New text.
        value: String,
    },
    /// Select text in a text input.
    SelectText {
        /// Text input element.
        id: ElementId,
        /// Byte range to select.
        range: std::ops::Range<usize>,
        /// Text the range refers to; the request is rejected if the input no longer shows it.
        expected: std::sync::Arc<str>,
    },
    /// Scroll a text input so the given byte offset is visible.
    RevealText {
        /// Text input element.
        id: ElementId,
        /// Byte offset in the text.
        byte: usize,
    },
}
/// Reason an [`AccessibilityAction`] was rejected.
#[derive(Debug, PartialEq)]
pub enum AccessibilityError {
    /// The element (or its state) no longer exists or is not in the active layer.
    Unavailable,
    /// The element or option is disabled.
    Disabled,
    /// The value is read-only.
    ReadOnly,
    /// The element does not support this action.
    Unsupported,
    /// The value or option is out of range or unknown.
    InvalidValue,
}

mod actions;
mod snapshot;
