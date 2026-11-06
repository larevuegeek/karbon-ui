//! Style calculé d'un élément, remis à son widget.
use super::{Properties, Typography};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
/// Result of style resolution for one element, handed to its widget.
pub struct ComputedStyle {
    /// Overflow is visible on at least one axis.
    pub overflow_visible: bool,
    /// Effective typography, including inherited values.
    pub text: Typography,
    /// Properties declared on the element itself after applying matching states.
    pub local: Properties,
    /// The element paints its own background (color, gradient or layers).
    pub element_background: bool,
    /// The element declares a border color or width.
    pub element_border: bool,
    /// The element declares padding.
    pub element_padding: bool,
}
impl ComputedStyle {
    /// Returns `true` when both styles produce the same layout metrics (colors and
    /// decorations ignored).
    pub fn metrics_eq(self, other: Self) -> bool {
        let text = |mut t: Typography| {
            t.color = None;
            t.decoration = None;
            t
        };
        text(self.text) == text(other.text)
            && self.local.layout == other.local.layout
            && text(self.local.text) == text(other.local.text)
            && self.local.padding == other.local.padding
            && self.local.border_width == other.local.border_width
            && self.local.border_edges == other.local.border_edges
            && self.element_padding == other.element_padding
    }
}
