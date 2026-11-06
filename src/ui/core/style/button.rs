use super::*;
/// Background and foreground colours of a button in one state.
#[derive(Clone, Copy)]
pub struct ButtonColors {
    /// Fill colour.
    pub background: StyleColor,
    /// Label colour.
    pub foreground: StyleColor,
}
impl ButtonColors {
    /// Colours from anything convertible to [`StyleColor`] (a [`Color`] or a [`ThemeColor`]).
    pub fn new(background: impl Into<StyleColor>, foreground: impl Into<StyleColor>) -> Self {
        Self {
            background: background.into(),
            foreground: foreground.into(),
        }
    }
}
/// Reusable presentation recipe for [`Button`](crate::ui::components::Button).
///
/// `None` colours fall back to the colours of the button variant.
#[derive(Clone, Copy)]
pub struct ButtonStyle {
    /// Colours at rest.
    pub normal: Option<ButtonColors>,
    /// Colours while hovered.
    pub hovered: Option<ButtonColors>,
    /// Colours while pressed.
    pub pressed: Option<ButtonColors>,
    /// Colours while disabled.
    pub disabled: Option<ButtonColors>,
    /// Colour of the focus ring.
    pub focus: Option<StyleColor>,
    /// Border colour.
    pub border: Option<StyleColor>,
    /// Border width, in pixels.
    pub border_width: f32,
    /// Corner radius, in pixels.
    pub radius: f32,
    /// Horizontal padding, in pixels.
    pub padding_x: f32,
    /// Vertical padding, in pixels.
    pub padding_y: f32,
    /// Minimum height, in pixels.
    pub min_height: f32,
    /// Label font size, in pixels.
    pub font_size: f32,
    /// Whether the label is bold.
    pub bold: bool,
}
impl ButtonStyle {
    /// Default recipe: 8 px radius, 16×10 px padding, 40 px minimum height, 14 px bold label.
    pub const DEFAULT: Self = Self {
        normal: None,
        hovered: None,
        pressed: None,
        disabled: None,
        focus: None,
        border: None,
        border_width: 1.,
        radius: 8.,
        padding_x: 16.,
        padding_y: 10.,
        min_height: 40.,
        font_size: 14.,
        bold: true,
    };
    pub(crate) fn validate(&self) -> crate::Result<()> {
        font_size(self.font_size)?;
        dimensions(&[
            self.border_width,
            self.radius,
            self.padding_x,
            self.padding_y,
            self.min_height,
        ])
    }
}
impl Default for ButtonStyle {
    fn default() -> Self {
        Self::DEFAULT
    }
}
