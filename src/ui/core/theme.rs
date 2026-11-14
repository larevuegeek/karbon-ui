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
    /// Colour given like CSS `oklch(L C h / alpha)`: lightness and alpha in `0..=1`,
    /// chroma usually in `0..=0.4`, hue in degrees. Out-of-gamut channels are clamped.
    ///
    /// For a colour that follows a value of the view, prefer a stylesheet rule reading a
    /// custom property (`oklch(0.7 0.15 var(--h))` with `style:--h={…}`).
    ///
    /// ```
    /// use karbon_ui::ui::theme::Color;
    ///
    /// assert_eq!(Color::oklch(1., 0., 0., 1.), Color::hex(0xffffff));
    /// ```
    pub fn oklch(lightness: f32, chroma: f32, hue: f32, alpha: f32) -> Self {
        use crate::ui::styling::color::{oklab_to_srgb, oklch_to_oklab, to_color};
        let [r, g, b] = oklab_to_srgb(oklch_to_oklab([lightness, chroma, hue]));
        to_color([r, g, b, alpha])
    }
    /// OKLab coordinates `[L, a, b]` of the colour (alpha ignored), the space in which
    /// `oklch()` and `color-mix(in oklab, …)` work: lightness in `0..=1`, chroma
    /// `a.hypot(b)`, hue `b.atan2(a)` in radians.
    ///
    /// ```
    /// use karbon_ui::ui::theme::Color;
    ///
    /// let [l, a, b] = Color::hex(0xffffff).to_oklab();
    /// assert!((l - 1.).abs() < 1e-3 && a.abs() < 1e-3 && b.abs() < 1e-3);
    /// ```
    pub fn to_oklab(self) -> [f32; 3] {
        let channel = |v: u8| f32::from(v) / 255.;
        crate::ui::styling::color::srgb_to_oklab([
            channel(self.red),
            channel(self.green),
            channel(self.blue),
        ])
    }
    /// Parses a CSS hexadecimal colour: `#rgb`, `#rgba`, `#rrggbb` or `#rrggbbaa` (the
    /// `#` is optional). Returns `None` for any other text.
    ///
    /// ```
    /// use karbon_ui::ui::theme::Color;
    ///
    /// assert_eq!(Color::parse("#22c55e"), Some(Color::hex(0x22c55e)));
    /// assert_eq!(Color::parse("#fff8"), Some(Color::rgba(255, 255, 255, 0x88)));
    /// assert_eq!(Color::parse("green"), None);
    /// ```
    pub fn parse(text: &str) -> Option<Self> {
        let digits = text.trim().strip_prefix('#').unwrap_or(text.trim());
        if !digits.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        let value = u32::from_str_radix(digits, 16).ok()?;
        let short = |n: u32| ((n & 0xf) * 0x11) as u8;
        Some(match digits.len() {
            3 => Self::rgb(short(value >> 8), short(value >> 4), short(value)),
            4 => Self::rgba(
                short(value >> 12),
                short(value >> 8),
                short(value >> 4),
                short(value),
            ),
            6 => Self::hex(value),
            8 => Self::rgba(
                (value >> 24) as u8,
                (value >> 16) as u8,
                (value >> 8) as u8,
                value as u8,
            ),
            _ => return None,
        })
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
