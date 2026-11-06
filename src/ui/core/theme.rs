// Couleurs RGB indépendantes du format attendu par le moteur de rendu.
/// RGBA colour with 8 bits per channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    /// Red channel.
    pub red: u8,
    /// Green channel.
    pub green: u8,
    /// Blue channel.
    pub blue: u8,
    /// Alpha channel, 255 being opaque.
    pub alpha: u8,
}
impl Color {
    /// Opaque colour, same RGB order as a CSS `#rrggbb` colour.
    pub const fn hex(rgb: u32) -> Self {
        Self::rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
    }
    /// Opaque colour from its channels.
    pub const fn rgb(red: u8, green: u8, blue: u8) -> Self {
        Self::rgba(red, green, blue, 255)
    }
    /// Colour from its channels, including alpha.
    pub const fn rgba(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }
}

/// Shared semantic palette, resolved by components and their style recipes.
#[derive(Clone, Copy, PartialEq)]
pub struct Theme {
    /// Window background.
    pub background: Color,
    /// Background of cards and panels.
    pub surface: Color,
    /// Title bar background.
    pub title_bar: Color,
    /// Main text.
    pub text: Color,
    /// Secondary text.
    pub muted: Color,
    /// Accent colour.
    pub accent: Color,
    /// Text drawn on the accent colour.
    pub on_accent: Color,
    /// Borders and separators.
    pub border: Color,
    /// Hover highlight.
    pub hover: Color,
    /// Pressed highlight.
    pub pressed: Color,
    /// Destructive actions and errors.
    pub danger: Color,
}
impl Theme {
    /// Built-in dark palette.
    pub const DARK: Self = Self {
        background: Color::rgb(10, 15, 12),
        surface: Color::rgb(17, 25, 21),
        title_bar: Color::rgb(19, 29, 23),
        text: Color::rgb(241, 245, 249),
        muted: Color::rgb(148, 163, 160),
        accent: Color::rgb(30, 211, 116),
        on_accent: Color::rgb(255, 255, 255),
        border: Color::rgb(43, 56, 49),
        hover: Color::rgb(38, 53, 44),
        pressed: Color::rgb(60, 75, 66),
        danger: Color::rgb(200, 40, 50),
    };
    /// Built-in light palette.
    pub const LIGHT: Self = Self {
        background: Color::rgb(243, 247, 244),
        surface: Color::rgb(255, 255, 255),
        title_bar: Color::rgb(231, 239, 234),
        text: Color::rgb(19, 34, 25),
        muted: Color::rgb(79, 100, 87),
        accent: Color::rgb(0, 125, 65),
        on_accent: Color::rgb(255, 255, 255),
        border: Color::rgb(198, 215, 204),
        hover: Color::rgb(207, 225, 214),
        pressed: Color::rgb(180, 207, 190),
        danger: Color::rgb(200, 40, 50),
    };
}
