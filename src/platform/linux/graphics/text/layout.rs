//! Mise en page d'un paragraphe par Pango : coupure des lignes et justification par
//! Pango ; placement, alignement, interlignage, ellipses et limite de lignes ici.
use super::{
    Fonts,
    attributes::{self, Segment},
};
use crate::{
    platform::linux::ffi::pango::{
        self, ELLIPSIZE_END, LayoutLine, Metrics, SCALE, WRAP_WORD_CHAR, index_to_x, line_extents,
    },
    ui::{
        element::SizeF,
        styling::{TextAlign, text::RichParagraph},
    },
};

/// Ligne placée dans la mise en page (origine en haut à gauche de la boîte).
pub(in crate::platform::linux) struct Line {
    /// Ligne Pango, possédée par l'une des mises en page de `Layout::owners`.
    pub raw: *mut LayoutLine,
    /// Octet du paragraphe où commence le texte de la mise en page de la ligne.
    pub shift: usize,
    /// Décalage horizontal de l'alignement.
    pub x: f32,
    pub top: f32,
    /// Ligne de base, depuis le haut de la mise en page.
    pub baseline: f32,
    pub height: f32,
    /// Largeur typographique, espaces de fin compris.
    pub width: f32,
    /// Largeur des espaces de fin.
    pub trailing: f32,
    /// Intervalle d'octets du paragraphe affiché par la ligne.
    pub start: usize,
    pub end: usize,
}
impl Line {
    /// Abscisse (depuis l'origine de la ligne) de l'octet `byte` du paragraphe.
    pub fn offset(&self, byte: usize) -> f32 {
        (index_to_x(self.raw, byte.saturating_sub(self.shift)) as f64 / SCALE) as f32
    }
}
pub(in crate::platform::linux) struct Layout {
    /// Paragraphe réellement mis en page (après une ellipse de début).
    pub paragraph: RichParagraph,
    pub lines: Vec<Line>,
    pub size: SizeF,
    pub segments: Vec<Segment>,
    /// Métriques de la police de base (interligne, décorations).
    pub metrics: Metrics,
    /// Mise en page du paragraphe entier, puis celles des lignes coupées par une ellipse.
    /// Gardées pour les pointeurs `Line::raw`, jamais modifiées après la construction.
    owners: Vec<pango::Layout>,
    /// Glyphes placés, extraits une fois pour le renderer Vulkan (`glyphs.rs`).
    pub glyphs: std::cell::OnceCell<std::rc::Rc<[super::glyphs::GlyphRun]>>,
}
impl Layout {
    /// Mise en page Pango du paragraphe entier (attributs des caractères).
    pub fn main(&self) -> &pango::Layout {
        &self.owners[0]
    }
}
/// Ligne brute avant placement : ligne Pango, décalage, intervalle, fin de paragraphe.
struct Raw {
    line: *mut LayoutLine,
    shift: usize,
    start: usize,
    end: usize,
    last: bool,
}
fn newline(c: char) -> bool {
    matches!(
        c,
        '\n' | '\u{0B}' | '\u{0C}' | '\r' | '\u{85}' | '\u{2028}' | '\u{2029}'
    )
}
/// Mise en page Pango des octets `range` de `paragraph`.
fn typeset(
    fonts: &mut Fonts,
    paragraph: &RichParagraph,
    segments: &[Segment],
    range: std::ops::Range<usize>,
    text: &str,
) -> crate::Result<pango::Layout> {
    let mut layout = fonts.map().layout()?;
    layout.set_text(text)?;
    layout.set_font_description(&attributes::font(
        fonts,
        paragraph,
        crate::ui::styling::text::SpanStyle::new(),
    )?);
    layout.set_attributes(&attributes::list(fonts, paragraph, segments, range)?);
    Ok(layout)
}
/// Ligne unique des octets `range`, coupée par `…` pour tenir dans `width` ; les retours
/// à la ligne y deviennent des espaces (Core Text les ignore dans une ligne isolée).
fn ellipsized(
    fonts: &mut Fonts,
    paragraph: &RichParagraph,
    segments: &[Segment],
    range: std::ops::Range<usize>,
    width: f32,
) -> crate::Result<pango::Layout> {
    let text = paragraph.text[range.clone()]
        .chars()
        .map(|c| {
            if newline(c) {
                " ".repeat(c.len_utf8())
            } else {
                c.to_string()
            }
        })
        .collect::<String>();
    let mut layout = typeset(fonts, paragraph, segments, range, &text)?;
    layout.set_width(Some(width.max(0.)));
    layout.set_ellipsize(ELLIPSIZE_END);
    Ok(layout)
}
/// Largeur typographique, ascendante et descendante d'une ligne Pango. Pango retire de
/// la largeur la demi-espace de lettres du début et de la fin de ligne, mais coupe les
/// lignes en la comptant : `spacing` la rend à toute ligne non vide (largeur de
/// DirectWrite, qui ajoute l'espacement après chaque caractère, dernier compris).
fn bounds(raw: &Raw, spacing: f32) -> (f32, f32, f32) {
    let r = line_extents(raw.line);
    let px = |v: i32| (f64::from(v) / SCALE) as f32;
    let extra = if raw.end > raw.start { spacing } else { 0. };
    (px(r.width) + extra, px(-r.y), px(r.height + r.y))
}
pub(super) fn build(
    fonts: &mut Fonts,
    paragraph: RichParagraph,
    width: f32,
    wrap: bool,
) -> crate::Result<Layout> {
    let format = paragraph.format;
    let text = std::sync::Arc::clone(&paragraph.text);
    let segments = attributes::segments(&paragraph);
    let base = attributes::font(
        fonts,
        &paragraph,
        crate::ui::styling::text::SpanStyle::new(),
    )?;
    let metrics = fonts.map().metrics(&base)?;
    let mut main = typeset(fonts, &paragraph, &segments, 0..text.len(), &text)?;
    if wrap {
        main.set_width(Some(width.max(1.)));
        main.set_wrap(WRAP_WORD_CHAR);
        main.set_justify(format.align == TextAlign::Justify);
    }
    let count = main.line_count();
    let mut raw = (0..count)
        .map(|i| {
            let line = main.line(i);
            // SAFETY : ligne valide tant que `main` n'est pas modifiée.
            let (start, length) = unsafe { ((*line).start_index, (*line).length) };
            let start = start.max(0) as usize;
            Raw {
                line,
                shift: 0,
                start,
                end: start + length.max(0) as usize,
                // SAFETY : idem ; bit 0 : la ligne commence un paragraphe.
                last: i + 1 == count || unsafe { (*main.line(i + 1)).flags } & 1 != 0,
            }
        })
        .collect::<Vec<_>>();
    let mut owners = vec![main];
    // `line-clamp` : la dernière ligne gardée reprend la suite du texte, coupée par `…`.
    if format.max_lines > 0 && raw.len() > format.max_lines as usize {
        raw.truncate(format.max_lines as usize);
        let last = raw.last_mut().expect("max_lines > 0");
        let mut end = text.len();
        while let Some(c) = text[last.start..end]
            .chars()
            .next_back()
            .filter(|c| newline(*c))
        {
            end -= c.len_utf8();
        }
        let rest = ellipsized(fonts, &paragraph, &segments, last.start..end, width)?;
        last.line = rest.line(0);
        last.shift = last.start;
        last.end = end;
        last.last = true;
        owners.push(rest);
    }
    let spacing = format.letter_spacing;
    if format.ellipsis {
        for line in &mut raw {
            let (line_width, ..) = bounds(line, spacing);
            if line_width - trailing(line, &text) > width + 0.01 {
                let cut = ellipsized(fonts, &paragraph, &segments, line.start..line.end, width)?;
                line.line = cut.line(0);
                line.shift = line.start;
                owners.push(cut);
            }
        }
    }
    let leading = (metrics.height - metrics.ascent - metrics.descent).max(0.);
    let placement = |ascent: f32, descent: f32| match format.line_height {
        Some(line) => (
            line,
            ((line - format.size) * 0.5 + format.size * 0.8).clamp(0., line),
        ),
        None => (ascent + descent + leading, ascent),
    };
    let mut lines = Vec::with_capacity(raw.len());
    let mut top = 0f32;
    for r in raw {
        let (line_width, mut ascent, mut descent) = bounds(&r, spacing);
        if ascent + descent <= 0. {
            (ascent, descent) = (metrics.ascent, metrics.descent);
        }
        let (height, baseline) = placement(ascent, descent);
        let trailing = trailing(&r, &text);
        let content = line_width - trailing;
        let x = match format.align {
            TextAlign::Start | TextAlign::Justify => 0.,
            TextAlign::Center => (width - content) * 0.5,
            TextAlign::End => width - content,
        };
        lines.push(Line {
            raw: r.line,
            shift: r.shift,
            x,
            top,
            baseline: top + baseline,
            height,
            width: line_width,
            trailing,
            start: r.start,
            end: r.end,
        });
        top += height;
    }
    let size = SizeF {
        width: lines.iter().map(|l| l.width).fold(0., f32::max),
        height: lines.iter().map(|l| l.height).sum(),
    };
    Ok(Layout {
        paragraph,
        lines,
        size,
        segments,
        metrics,
        owners,
        glyphs: std::cell::OnceCell::new(),
    })
}
/// Largeur des espaces de fin d'une ligne (blancs avant sa fin).
fn trailing(line: &Raw, text: &str) -> f32 {
    let shown = &text[line.start..line.end.min(text.len())];
    let kept = shown.trim_end_matches(char::is_whitespace).len();
    if kept == shown.len() {
        return 0.;
    }
    let x = |byte: usize| index_to_x(line.line, byte.saturating_sub(line.shift)) as f64 / SCALE;
    (x(line.end) - x(line.start + kept)).abs() as f32
}
