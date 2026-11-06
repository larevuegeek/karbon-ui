//! Propriétés de texte héritables ([`Typography`]) et leurs valeurs.
use crate::ui::{font::Font, theme::Color};

/// Horizontal text alignment (CSS `text-align`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextAlign {
    /// Align to the start edge (default).
    #[default]
    Start,
    /// Center each line.
    Center,
    /// Align to the end edge.
    End,
    /// Stretch lines to fill the available width.
    Justify,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
/// Text decoration lines (CSS `text-decoration-line`); flags can be combined.
pub struct TextDecoration {
    /// Draw a line below the text.
    pub underline: bool,
    /// Draw a line through the text.
    pub line_through: bool,
    /// Draw a line above the text.
    pub overline: bool,
}
#[derive(Clone, Copy, Debug, PartialEq)]
/// Line height (CSS `line-height`).
pub enum LineHeight {
    /// The font's natural line height.
    Normal,
    /// Multiple of the font size; must be finite and positive.
    Factor(f32),
    /// Absolute height in pixels; must be finite and positive.
    Px(f32),
}
/// Letter spacing (CSS `letter-spacing`), added after every character.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LetterSpacing {
    /// Logical pixels (may be negative).
    Px(f32),
    /// Multiple of the font size of the element that declares it.
    Em(f32),
}
impl LetterSpacing {
    /// Resolves the spacing in pixels for a font of `size` pixels.
    pub fn resolve(self, size: f32) -> f32 {
        match self {
            Self::Px(n) => n,
            Self::Em(n) => n * size,
        }
    }
    /// Multiplies pixel values by `scale` (template scale factor).
    pub fn scaled(self, scale: f32) -> Self {
        match self {
            Self::Px(n) => Self::Px(n * scale),
            em => em,
        }
    }
}
/// Case transformation applied before layout (CSS `text-transform`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextTransform {
    /// Text as written (default).
    #[default]
    None,
    /// Every letter in capitals.
    Uppercase,
    /// Every letter in lower case.
    Lowercase,
    /// First letter of each word in capitals.
    Capitalize,
}
impl TextTransform {
    /// Applies the transformation to `value`; borrows it when nothing changes.
    pub fn apply(self, value: &str) -> std::borrow::Cow<'_, str> {
        match self {
            Self::None => value.into(),
            Self::Uppercase => value.to_uppercase().into(),
            Self::Lowercase => value.to_lowercase().into(),
            Self::Capitalize => {
                let mut out = String::with_capacity(value.len());
                let mut start = true;
                for c in value.chars() {
                    if start && c.is_alphanumeric() {
                        out.extend(c.to_uppercase());
                    } else {
                        out.push(c);
                    }
                    start = c.is_whitespace() || matches!(c, '-' | '(' | '"' | '\'' | '«');
                }
                out.into()
            }
        }
    }
}
/// Line breaking of text (CSS `white-space`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum WhiteSpace {
    /// Lines wrap at the box width (default).
    #[default]
    Normal,
    /// No automatic wrapping; only explicit line breaks.
    NoWrap,
    /// Spaces and line breaks of dynamic text are kept; no automatic wrapping.
    Pre,
    /// Spaces and line breaks of dynamic text are kept; lines wrap at the box width.
    PreWrap,
}
impl WhiteSpace {
    /// `true` when lines wrap at the box width.
    pub fn wraps(self) -> bool {
        matches!(self, Self::Normal | Self::PreWrap)
    }
}
/// What happens to text overflowing its line box (CSS `text-overflow`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextOverflow {
    /// Overflowing text is clipped (default).
    #[default]
    Clip,
    /// The last visible characters are replaced by `…`.
    Ellipsis,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
/// Inheritable text properties; `None` means "inherit from the parent".
pub struct Typography {
    /// Font size in pixels (finite and positive).
    pub size: Option<f32>,
    /// Font family name (non-empty, without NUL).
    pub family: Option<&'static str>,
    /// Font weight, from 1 to 999.
    pub weight: Option<u16>,
    /// Text color.
    pub color: Option<Color>,
    /// Horizontal alignment.
    pub align: Option<TextAlign>,
    /// Line height.
    pub line_height: Option<LineHeight>,
    /// Decoration lines.
    pub decoration: Option<TextDecoration>,
    /// Extra space after each character.
    pub letter_spacing: Option<LetterSpacing>,
    /// Italic style (CSS `font-style`).
    pub italic: Option<bool>,
    /// Case transformation.
    pub transform: Option<TextTransform>,
    /// Line breaking.
    pub white_space: Option<WhiteSpace>,
    /// Overflow marker of a clipped line. Propagated to descendant text, so that
    /// `.truncate` works on a container as well as on the text itself.
    pub overflow: Option<TextOverflow>,
    /// Maximum number of lines (`line-clamp`), `0` for none; the last visible line ends
    /// with `…` when text is cut. Propagated like [`overflow`](Self::overflow).
    pub line_clamp: Option<u16>,
    /// Use tabular (fixed-width) digits (CSS `font-variant-numeric: tabular-nums`).
    pub tabular_numbers: Option<bool>,
    /// Right-to-left base direction (CSS `direction: rtl`). Only the side of the
    /// ellipsis changes: a truncated single line keeps its end and starts with `…`.
    pub rtl: Option<bool>,
    /// Colour of the decoration lines (`text-decoration-color`); the text colour when
    /// unset.
    pub decoration_color: Option<Color>,
    /// Distance between the alphabetic baseline and the underline, in pixels
    /// (`text-underline-offset`); the font's position when unset.
    pub underline_offset: Option<f32>,
    /// Thickness of the decoration lines, in pixels (`text-decoration-thickness`).
    pub decoration_thickness: Option<f32>,
}
impl Typography {
    /// Layers `self` over `base`: each property set on `self` wins, the others
    /// come from `base`.
    pub fn over(self, base: Self) -> Self {
        Self {
            size: self.size.or(base.size),
            family: self.family.or(base.family),
            weight: self.weight.or(base.weight),
            color: self.color.or(base.color),
            align: self.align.or(base.align),
            line_height: self.line_height.or(base.line_height),
            // Text decoration is propagated separately from font inheritance.
            decoration: self.decoration.or(base.decoration),
            letter_spacing: self.letter_spacing.or(base.letter_spacing),
            italic: self.italic.or(base.italic),
            transform: self.transform.or(base.transform),
            white_space: self.white_space.or(base.white_space),
            overflow: self.overflow.or(base.overflow),
            line_clamp: self.line_clamp.or(base.line_clamp),
            tabular_numbers: self.tabular_numbers.or(base.tabular_numbers),
            rtl: self.rtl.or(base.rtl),
            decoration_color: self.decoration_color.or(base.decoration_color),
            underline_offset: self.underline_offset.or(base.underline_offset),
            decoration_thickness: self.decoration_thickness.or(base.decoration_thickness),
        }
    }
    /// Checks size, weight, family and line height against their documented
    /// ranges; returns an [`Error::InvalidInput`](crate::Error::InvalidInput) error otherwise.
    pub fn validate(self) -> crate::Result<()> {
        if self.size.is_some_and(|n| !n.is_finite() || n <= 0.)
            || self.weight.is_some_and(|n| !(1..=999).contains(&n))
            || self
                .family
                .is_some_and(|s| s.is_empty() || s.contains('\0'))
            || matches!(self.line_height,Some(LineHeight::Factor(n)|LineHeight::Px(n)) if !n.is_finite()||n<=0.)
            || matches!(self.letter_spacing,Some(LetterSpacing::Px(n)|LetterSpacing::Em(n)) if !n.is_finite())
        {
            return Err(crate::Error::invalid_input("invalid computed typography"));
        }
        Ok(())
    }
    /// Returns `base` with the family and weight overridden when set.
    pub fn font(self, base: Font) -> Font {
        Font::new(
            self.family.unwrap_or(base.family),
            self.weight.unwrap_or(base.weight),
        )
    }
    /// Resolves the line height in pixels for a font of `size` pixels;
    /// `None` for [`LineHeight::Normal`] or when unset.
    pub fn line(self, size: f32) -> Option<f32> {
        match self.line_height {
            Some(LineHeight::Factor(n)) => Some(n * size),
            Some(LineHeight::Px(n)) => Some(n),
            _ => None,
        }
    }
}
