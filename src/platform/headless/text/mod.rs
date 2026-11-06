//! Métriques de texte déterministes, indépendantes des polices installées.
mod lines;
mod measurer;
mod rich;
#[cfg(test)]
mod tests;

pub(super) use lines::formatted_lines;
pub(super) use rich::{align_lines, rich_lines};

/// Deterministic [`TextMeasurer`](crate::ui::element::TextMeasurer) with fixed metrics, independent of installed fonts and
/// of the operating system.
///
/// - Every character (Unicode scalar value, spaces included) advances by
///   [`ADVANCE`](Self::ADVANCE) × font size, whatever the family or weight.
/// - A line is [`LINE_HEIGHT`](Self::LINE_HEIGHT) × font size tall, or the explicit
///   line height of a [`TextFormat`](crate::ui::styling::text::TextFormat).
/// - `\n` starts a new line (a trailing `\r` is ignored). A line wider than the available
///   width wraps after the last space that fits; a word wider than the whole line is
///   broken between characters. The space at a wrap point is not counted.
/// - The measured width is that of the widest line; the height is the number of lines ×
///   the line height. Empty text measures zero wide and one line high.
/// - The available width is clamped to `1.0..=16384.0`, like the Windows backend.
///
/// For example, `"Hello"` at 20 px measures 60 × 25, and `"Hello world"` at 20 px
/// wrapped to 80 px gives two lines: 60 × 50.
///
/// ```
/// use karbon_ui::{platform::headless::HeadlessTextMeasurer, ui::element::TextMeasurer};
///
/// let mut text = HeadlessTextMeasurer::new();
/// let size = text.measure_text("Hello world", 20., false, 80.).unwrap();
/// assert_eq!((size.width, size.height), (60., 50.));
/// ```
#[derive(Debug, Default, Clone, Copy)]
pub struct HeadlessTextMeasurer;

impl HeadlessTextMeasurer {
    /// Horizontal advance of every character, as a fraction of the font size.
    pub const ADVANCE: f32 = 0.6;
    /// Default line height, as a fraction of the font size.
    pub const LINE_HEIGHT: f32 = 1.25;
    /// Create a measurer.
    pub const fn new() -> Self {
        Self
    }
}

/// Largeur disponible bornée comme DirectWrite (1 à 16 384 px), NaN compris.
fn clamp_width(width: f32) -> f32 {
    if width.is_nan() {
        16384.
    } else {
        width.clamp(1., 16384.)
    }
}

pub(super) fn line_height(size: f32, explicit: Option<f32>) -> f32 {
    explicit.unwrap_or(size * HeadlessTextMeasurer::LINE_HEIGHT)
}
