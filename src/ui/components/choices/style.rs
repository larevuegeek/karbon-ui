use super::*;
#[derive(Clone, Default)]
/// Colour overrides shared by [`Checkbox`](super::Checkbox),
/// [`Switch`](super::Switch) and [`RadioButton`](super::RadioButton); `None`
/// keeps the theme colour.
pub struct ChoiceStyle {
    /// Fill of the checked indicator (theme accent by default).
    pub accent: Option<Color>,
    /// Label colour (theme text by default).
    pub foreground: Option<Color>,
    /// Indicator outline (theme muted colour by default).
    pub border: Option<Color>,
    /// Unchecked indicator fill (theme title bar colour by default).
    pub track: Option<Color>,
    /// Focus ring colour (theme accent by default).
    pub focus: Option<Color>,
}
