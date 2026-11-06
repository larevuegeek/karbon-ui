use super::*;
use crate::ui::{
    element::{SizeF, TextMeasurer},
    font::Font,
    scene::{DrawCommand, RectF, Scene},
    theme::Color,
};
use std::sync::Arc;
/// Colour, position and thickness of decoration lines (`text-decoration-color`,
/// `text-underline-offset`, `text-decoration-thickness`); `None` keeps the text colour
/// and the font's metrics.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DecorationStyle {
    /// Line colour.
    pub color: Option<Color>,
    /// Distance from the alphabetic baseline to the top of the underline, in pixels.
    pub offset: Option<f32>,
    /// Line thickness in pixels.
    pub thickness: Option<f32>,
}
impl DecorationStyle {
    /// `true` when the lines must be drawn by the renderer itself instead of the
    /// font's own decorations.
    pub fn custom(self) -> bool {
        self != Self::default()
    }
}
/// Resolved text format passed to the renderer and the text measurer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextFormat {
    /// Clip glyphs to the layout rectangle (default `true`).
    pub clip: bool,
    /// Font family and weight.
    pub font: Font,
    /// Font size in device-independent pixels.
    pub size: f32,
    /// Horizontal alignment.
    pub align: TextAlign,
    /// Line height in pixels; `None` uses the font's natural line height.
    pub line_height: Option<f32>,
    /// Underline, strike-through and overline flags.
    pub decoration: TextDecoration,
    /// Colour, underline offset and thickness of the decoration lines.
    pub decoration_style: DecorationStyle,
    /// Extra space after each character, in pixels (may be negative).
    pub letter_spacing: f32,
    /// Italic style.
    pub italic: bool,
    /// Tabular (fixed-width) digits.
    pub tabular_numbers: bool,
    /// Case transformation, applied by [`measure`] and [`draw`] before layout.
    pub transform: TextTransform,
    /// Line breaking; [`WhiteSpace::NoWrap`] and [`WhiteSpace::Pre`] keep one line per
    /// explicit line break.
    pub white_space: WhiteSpace,
    /// End clipped lines with `…`.
    pub ellipsis: bool,
    /// With [`ellipsis`](Self::ellipsis) on a single line, cut the start of the text
    /// instead of its end (`…e HD\FLAC\`), as CSS `direction: rtl`.
    pub ellipsis_start: bool,
    /// Maximum number of lines, `0` for none; the last kept line ends with `…` when
    /// text is cut.
    pub max_lines: u16,
}
impl TextFormat {
    /// Builds a format from computed `text`, falling back to `font` and `size`
    /// for properties it leaves unset.
    pub fn new(text: Typography, font: Font, size: f32) -> Self {
        let size = text.size.unwrap_or(size);
        Self {
            clip: true,
            font: text.font(font),
            size,
            align: text.align.unwrap_or_default(),
            line_height: text.line(size),
            decoration: text.decoration.unwrap_or_default(),
            decoration_style: DecorationStyle {
                color: text.decoration_color,
                offset: text.underline_offset,
                thickness: text.decoration_thickness,
            },
            letter_spacing: text.letter_spacing.map_or(0., |s| s.resolve(size)),
            italic: text.italic.unwrap_or(false),
            tabular_numbers: text.tabular_numbers.unwrap_or(false),
            transform: text.transform.unwrap_or_default(),
            white_space: text.white_space.unwrap_or_default(),
            ellipsis: text.overflow == Some(TextOverflow::Ellipsis),
            ellipsis_start: text.overflow == Some(TextOverflow::Ellipsis)
                && text.rtl == Some(true)
                && !text.white_space.unwrap_or_default().wraps(),
            max_lines: text.line_clamp.unwrap_or(0),
        }
    }
    /// Returns `true` when the format needs the formatted-text path (no clipping,
    /// non-start alignment, explicit line height, decoration, spacing, italics,
    /// tabular digits, no wrapping, ellipsis or line clamp).
    pub fn advanced(self) -> bool {
        !self.clip
            || self.align != TextAlign::Start
            || self.line_height.is_some()
            || self.decoration != TextDecoration::default()
            || self.letter_spacing != 0.
            || self.italic
            || self.tabular_numbers
            || !self.white_space.wraps()
            || self.ellipsis
            || self.max_lines > 0
    }
    /// `true` when lines wrap at the box width.
    pub fn wraps(self) -> bool {
        self.white_space.wraps()
    }
    /// Returns the format with clipping set to `value`.
    pub fn clip(mut self, value: bool) -> Self {
        self.clip = value;
        self
    }
    /// Checks that size, family, weight and line height are valid.
    ///
    /// Returns an [`Error::InvalidInput`](crate::Error::InvalidInput) error otherwise.
    pub fn validate(self) -> crate::Result<()> {
        Typography {
            size: Some(self.size),
            family: Some(self.font.family),
            weight: Some(self.font.weight),
            line_height: self.line_height.map(LineHeight::Px),
            letter_spacing: Some(LetterSpacing::Px(self.letter_spacing)),
            ..Default::default()
        }
        .validate()
    }
}
/// Validates `format` and measures `value` laid out within `width` pixels.
///
/// # Errors
///
/// [`Error::InvalidInput`](crate::Error::InvalidInput) for an invalid `format`, and any
/// error returned by `measurer`.
///
/// The [`TextTransform`] of `format` is applied first. Without wrapping
/// (`white-space: nowrap | pre`) the natural width is returned, or at most `width`
/// with an ellipsis.
pub fn measure(
    measurer: &mut dyn TextMeasurer,
    value: &str,
    format: TextFormat,
    width: f32,
) -> crate::Result<SizeF> {
    format.validate()?;
    let value = format.transform.apply(value);
    measurer.measure_formatted(&value, format, width)
}
/// Records a text draw command for `value` in `rect`, wrapping lines when `wrap`
/// is `true` and the format wraps. Picks the simple or formatted text command as
/// needed, after applying the format's [`TextTransform`].
pub fn draw(
    scene: &mut Scene,
    rect: RectF,
    value: impl Into<Arc<str>>,
    format: TextFormat,
    color: Color,
    wrap: bool,
) {
    let mut value: Arc<str> = value.into();
    if format.transform != TextTransform::None
        && let std::borrow::Cow::Owned(text) = format.transform.apply(&value)
    {
        value = text.into();
    }
    let wrap = wrap && format.wraps();
    if format.advanced() {
        scene.commands.push(DrawCommand::FormattedText {
            rect,
            text: value,
            format,
            color,
            wrap,
        });
    } else {
        scene.commands.push(DrawCommand::FontText {
            rect,
            text: value,
            size: format.size,
            font: format.font,
            color,
            wrap,
        });
    }
}
/// Stores `value` into a widget's cached computed style.
///
/// The default style is stored as `None`. Returns `true` when the change
/// affects text metrics or layout (and so requires a new layout pass).
pub fn update(target: &mut Option<Box<ComputedStyle>>, value: &ComputedStyle) -> bool {
    if target.is_none() && *value == ComputedStyle::default() {
        return false;
    }
    if target.as_deref() == Some(value) {
        return false;
    }
    let layout = target.as_ref().is_none_or(|old| !old.metrics_eq(*value));
    if *value == ComputedStyle::default() {
        *target = None;
    } else {
        *target = Some(Box::new(*value));
    }
    layout
}

/// Style overrides of one range of a [`RichParagraph`]. Unset fields inherit the
/// paragraph's base [`TextFormat`] and colour.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SpanStyle {
    /// Font weight, from 1 to 999.
    pub weight: Option<u16>,
    /// Draw the range in italics.
    pub italic: bool,
    /// Decoration lines added to the range (combined with the base decoration).
    pub decoration: TextDecoration,
    /// Text colour.
    pub color: Option<Color>,
    /// Font size in pixels.
    pub size: Option<f32>,
    /// Font family name.
    pub family: Option<&'static str>,
}
impl SpanStyle {
    /// Style without overrides.
    pub const fn new() -> Self {
        Self {
            weight: None,
            italic: false,
            decoration: TextDecoration {
                underline: false,
                line_through: false,
                overline: false,
            },
            color: None,
            size: None,
            family: None,
        }
    }
    /// Bold (weight 700).
    pub const fn bold(mut self) -> Self {
        self.weight = Some(700);
        self
    }
    /// Sets the font weight (1 to 999).
    pub const fn weight(mut self, weight: u16) -> Self {
        self.weight = Some(weight);
        self
    }
    /// Italic.
    pub const fn italic(mut self) -> Self {
        self.italic = true;
        self
    }
    /// Underlined.
    pub const fn underline(mut self) -> Self {
        self.decoration.underline = true;
        self
    }
    /// Struck through.
    pub const fn strike(mut self) -> Self {
        self.decoration.line_through = true;
        self
    }
    /// Sets the colour.
    pub const fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
    /// Sets the font size in pixels.
    pub const fn size(mut self, size: f32) -> Self {
        self.size = Some(size);
        self
    }
    /// Sets the font family.
    pub const fn family(mut self, family: &'static str) -> Self {
        self.family = Some(family);
        self
    }
    /// Layers `self` over an enclosing style: values set on `self` win, flags combine.
    pub fn over(self, parent: Self) -> Self {
        Self {
            weight: self.weight.or(parent.weight),
            italic: self.italic || parent.italic,
            decoration: TextDecoration {
                underline: self.decoration.underline || parent.decoration.underline,
                line_through: self.decoration.line_through || parent.decoration.line_through,
                overline: self.decoration.overline || parent.decoration.overline,
            },
            color: self.color.or(parent.color),
            size: self.size.or(parent.size),
            family: self.family.or(parent.family),
        }
    }
    /// Span style carrying the text properties of `typography` (size, family, weight,
    /// colour and decoration); alignment and line height belong to the paragraph.
    pub fn from_typography(typography: Typography) -> Self {
        Self {
            weight: typography.weight,
            italic: typography.italic.unwrap_or(false),
            decoration: typography.decoration.unwrap_or_default(),
            color: typography.color,
            size: typography.size,
            family: typography.family,
        }
    }
    /// `true` when the style changes nothing.
    pub fn is_plain(self) -> bool {
        self == Self::new()
    }
    fn validate(self) -> crate::Result<()> {
        Typography {
            size: self.size,
            family: self.family,
            weight: self.weight,
            ..Default::default()
        }
        .validate()
    }
}
/// Styled byte range of a [`RichParagraph`].
#[derive(Clone, Debug, PartialEq)]
pub struct TextSpan {
    /// Byte range in [`RichParagraph::text`], on character boundaries.
    pub range: std::ops::Range<usize>,
    /// Overrides applied to the range.
    pub style: SpanStyle,
}
/// Paragraph whose ranges carry their own style, laid out and drawn as one block by
/// the backend (wrapping, alignment and line height come from `format`).
///
/// Built by [`RichText`](crate::ui::components::RichText); a custom widget can draw one
/// with [`DrawCommand::RichText`] and measure it with
/// [`TextMeasurer::measure_rich`].
#[derive(Clone, Debug, PartialEq)]
pub struct RichParagraph {
    /// Whole text.
    pub text: Arc<str>,
    /// Base format: font, size, alignment, line height and decoration.
    pub format: TextFormat,
    /// Styled ranges, sorted and non-overlapping; text outside them uses the base format.
    pub spans: Vec<TextSpan>,
}
/// Points de coupe possibles d'une ellipse de début : nombre de caractères retirés en
/// tête (au moins un) pour que `…` suivi du reste tienne selon `fits`. Recherche
/// dichotomique : `fits` est appelé O(log n) fois.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn cut_start(
    text: &str,
    mut fits: impl FnMut(&str) -> crate::Result<bool>,
) -> crate::Result<usize> {
    let bounds = text
        .char_indices()
        .map(|(i, _)| i)
        .chain([text.len()])
        .collect::<Vec<_>>();
    let (mut low, mut high) = (1usize.min(bounds.len() - 1), bounds.len() - 1);
    while low < high {
        let middle = (low + high) / 2;
        if fits(&format!("\u{2026}{}", &text[bounds[middle]..]))? {
            high = middle;
        } else {
            low = middle + 1;
        }
    }
    Ok(bounds[low])
}
impl RichParagraph {
    /// Paragraphe dont les `at` premiers octets sont remplacés par `…` (portions décalées,
    /// le signe prend le style du premier caractère gardé).
    #[cfg_attr(
        not(any(windows, target_os = "macos", target_os = "linux")),
        allow(dead_code)
    )]
    pub(crate) fn cut_start(&self, at: usize) -> Self {
        const SIGN: &str = "\u{2026}";
        let shift = |i: usize| i.saturating_sub(at) + SIGN.len();
        let first = self.style_at(at);
        let mut spans = vec![TextSpan {
            range: 0..SIGN.len(),
            style: first,
        }];
        spans.extend(
            self.spans
                .iter()
                .filter(|s| s.range.end > at)
                .map(|s| TextSpan {
                    range: shift(s.range.start.max(at))..shift(s.range.end),
                    style: s.style,
                }),
        );
        Self {
            text: format!("{SIGN}{}", &self.text[at..]).into(),
            format: self.format,
            spans,
        }
    }
    /// Checks the base format, the span styles, and that spans are sorted,
    /// non-overlapping and on character boundaries.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidInput`](crate::Error::InvalidInput) when one of these rules is broken.
    pub fn validate(&self) -> crate::Result<()> {
        self.format.validate()?;
        let mut end = 0;
        for span in &self.spans {
            if span.range.start < end
                || span.range.start > span.range.end
                || span.range.end > self.text.len()
                || !self.text.is_char_boundary(span.range.start)
                || !self.text.is_char_boundary(span.range.end)
            {
                return Err(crate::Error::invalid_input("invalid rich text span range"));
            }
            end = span.range.end;
            span.style.validate()?;
        }
        Ok(())
    }
    /// Effective style of byte `index`: the style of the span containing it, or the
    /// plain style.
    pub fn style_at(&self, index: usize) -> SpanStyle {
        self.spans
            .iter()
            .find(|s| s.range.contains(&index))
            .map_or(SpanStyle::new(), |s| s.style)
    }
    /// Font size of byte `index`.
    pub fn size_at(&self, index: usize) -> f32 {
        self.style_at(index).size.unwrap_or(self.format.size)
    }
}
