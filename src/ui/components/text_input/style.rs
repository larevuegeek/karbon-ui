use super::*;
/// Appearance of a [`TextInput`](super::TextInput); colours set to `None` keep
/// the theme colour.
#[derive(Clone, Copy)]
pub struct InputStyle {
    /// Font size in logical units, 1 to 128 (15 by default).
    pub font_size: f32,
    /// Inner padding on every side in logical units, non-negative (12 by default).
    pub padding: f32,
    /// Corner radius in logical units, non-negative (8 by default).
    pub radius: f32,
    /// Background colour.
    pub background: Option<Color>,
    /// Text colour.
    pub foreground: Option<Color>,
    /// Border colour when not focused (theme border by default).
    pub border: Option<Color>,
    /// Border colour when focused (theme accent by default).
    pub focus_border: Option<Color>,
}
impl Default for InputStyle {
    fn default() -> Self {
        Self {
            font_size: 15.,
            padding: 12.,
            radius: 8.,
            background: None,
            foreground: None,
            border: None,
            focus_border: None,
        }
    }
}
