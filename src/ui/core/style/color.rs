use super::*;
/// Semantic colour role, resolved against the active [`Theme`] at paint time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeColor {
    /// Window background.
    Background,
    /// Background of cards and panels.
    Surface,
    /// Title bar background.
    TitleBar,
    /// Main text.
    Text,
    /// Secondary text.
    Muted,
    /// Accent colour.
    Accent,
    /// Borders and separators.
    Border,
    /// Hover highlight.
    Hover,
    /// Pressed highlight.
    Pressed,
    /// Destructive actions and errors.
    Danger,
    /// Text drawn on the accent colour.
    OnAccent,
}
impl ThemeColor {
    /// Colour of this role in `theme`.
    pub fn resolve(self, theme: Theme) -> Color {
        match self {
            Self::Background => theme.background,
            Self::Surface => theme.surface,
            Self::TitleBar => theme.title_bar,
            Self::Text => theme.text,
            Self::Muted => theme.muted,
            Self::Accent => theme.accent,
            Self::Border => theme.border,
            Self::Hover => theme.hover,
            Self::Pressed => theme.pressed,
            Self::Danger => theme.danger,
            Self::OnAccent => theme.on_accent,
        }
    }
    /// This role with the alpha channel replaced by `alpha`.
    pub fn with_alpha(self, alpha: u8) -> StyleColor {
        StyleColor::ThemeAlpha(self, alpha)
    }
}
/// Colour used by a style recipe: fixed, or taken from the theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleColor {
    /// Fixed colour, independent of the theme.
    Fixed(Color),
    /// Colour of a theme role.
    Theme(ThemeColor),
    /// Colour of a theme role with a replaced alpha channel.
    ThemeAlpha(ThemeColor, u8),
}
impl StyleColor {
    /// Concrete colour for `theme`.
    pub fn resolve(self, theme: Theme) -> Color {
        match self {
            Self::Fixed(color) => color,
            Self::Theme(role) => role.resolve(theme),
            Self::ThemeAlpha(role, alpha) => Color {
                alpha,
                ..role.resolve(theme)
            },
        }
    }
}
impl From<Color> for StyleColor {
    fn from(value: Color) -> Self {
        Self::Fixed(value)
    }
}
impl From<ThemeColor> for StyleColor {
    fn from(value: ThemeColor) -> Self {
        Self::Theme(value)
    }
}
