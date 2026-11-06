use super::*;
/// Reusable presentation recipe for [`Text`](crate::ui::components::Text).
#[derive(Clone, Copy)]
pub struct TextStyle {
    /// Font size, in pixels.
    pub font_size: f32,
    /// Whether the text is bold.
    pub bold: bool,
    /// Text colour; `None` uses the theme text colour.
    pub color: Option<StyleColor>,
}
impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_size: 15.,
            bold: false,
            color: None,
        }
    }
}
