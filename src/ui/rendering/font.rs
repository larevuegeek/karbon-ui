//! Fonts supplied by the application; nothing is installed system-wide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Font selection: a family name and a CSS-style weight.
pub struct Font {
    /// Family name, either a system family or one supplied as a [`FontAsset`].
    pub family: &'static str,
    /// Weight from 100 to 900 (400 = regular, 700 = bold).
    pub weight: u16,
}
impl Font {
    /// Font with the given family and weight.
    pub const fn new(family: &'static str, weight: u16) -> Self {
        Self { family, weight }
    }
    /// Default system font (Segoe UI), regular or bold.
    pub const fn system(bold: bool) -> Self {
        Self::new("Segoe UI", if bold { 700 } else { 400 })
    }
}
/// Fixed-width family used for code (`<code>`, `<pre>` in templates): Consolas on Windows,
/// the generic `monospace` family elsewhere.
pub const MONOSPACE: &str = if cfg!(windows) {
    "Consolas"
} else {
    "monospace"
};
impl From<&'static str> for Font {
    fn from(family: &'static str) -> Self {
        Self::new(family, 400)
    }
}
/// Font file embedded in the application and loaded privately by the backend.
///
/// Returned by [`Application::fonts`](crate::application::Application::fonts).
pub struct FontAsset {
    /// Family name under which the font is referenced by [`Font::family`].
    pub family: &'static str,
    /// Raw font file bytes (TTF, OTF, WOFF or WOFF2); must be non-empty and at most 16 MiB.
    pub data: &'static [u8],
}
