//! Mise en page d'un paragraphe par `CTTypesetter` : coupure des lignes, alignement,
//! interlignage, ellipses et limite de lignes.
use super::super::super::ffi::{byte_map, range, utf16_map};
use super::Fonts;
use super::attributed::{self, Segment};
use crate::ui::{
    element::SizeF,
    styling::{
        TextAlign,
        text::{RichParagraph, SpanStyle},
    },
};
use objc2_core_foundation::{CFAttributedString, CFIndex, CFRetained, CGFloat};
use objc2_core_text::{CTFont, CTLine, CTLineTruncationType, CTTypesetter};

/// Ligne placée dans la mise en page (origine en haut à gauche de la boîte).
pub(in crate::platform::macos) struct Line {
    pub line: CFRetained<CTLine>,
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
    /// Intervalle UTF-16 affiché par la ligne.
    pub start: usize,
    pub end: usize,
}
impl Line {
    /// Abscisse (depuis l'origine de la ligne) de l'index UTF-16 `index`.
    pub fn offset(&self, index: usize) -> f32 {
        // SAFETY : ligne valide ; décalage secondaire non demandé.
        (unsafe {
            self.line
                .offset_for_string_index(index as CFIndex, std::ptr::null_mut())
        }) as f32
    }
}
pub(in crate::platform::macos) struct Layout {
    /// Paragraphe réellement mis en page (après une ellipse de début).
    pub paragraph: RichParagraph,
    pub lines: Vec<Line>,
    pub size: SizeF,
    pub segments: Vec<Segment>,
    /// Police de base (métriques des décorations).
    pub base: CFRetained<CTFont>,
    /// Index UTF-16 de chaque octet, et octet de chaque index UTF-16.
    pub utf16: Vec<usize>,
    pub bytes: Vec<usize>,
    /// Glyphes placés, extraits une fois pour le renderer Metal (`glyphs.rs`).
    pub glyphs: std::cell::OnceCell<std::rc::Rc<[super::glyphs::GlyphRun]>>,
}
fn newline(unit: u16) -> bool {
    matches!(unit, 0x0A | 0x0B | 0x0C | 0x0D | 0x85 | 0x2028 | 0x2029)
}
/// Ligne brute avant placement : ligne Core Text, intervalle, fin de paragraphe.
struct Raw {
    line: CFRetained<CTLine>,
    start: usize,
    end: usize,
    last: bool,
}
fn empty_line() -> crate::Result<CFRetained<CTLine>> {
    // SAFETY : chaîne vide sans attribut.
    let string = unsafe {
        CFAttributedString::new(None, Some(&super::super::super::ffi::cf_string("")), None)
    };
    let string = super::super::super::ffi::required(string, "CFAttributedStringCreate")?;
    // SAFETY : chaîne attribuée valide.
    Ok(unsafe { CTLine::with_attributed_string(&string) })
}
/// Largeur typographique, ascendante, descendante et interligne d'une ligne.
fn bounds(line: &CTLine) -> (f32, f32, f32, f32) {
    let (mut ascent, mut descent, mut leading): (CGFloat, CGFloat, CGFloat) = (0., 0., 0.);
    // SAFETY : sorties locales valides en écriture.
    let width = unsafe { line.typographic_bounds(&mut ascent, &mut descent, &mut leading) };
    (width as f32, ascent as f32, descent as f32, leading as f32)
}
pub(super) fn build(
    fonts: &mut Fonts,
    paragraph: RichParagraph,
    width: f32,
    wrap: bool,
) -> crate::Result<Layout> {
    let format = paragraph.format;
    let utf16 = utf16_map(&paragraph.text);
    let bytes = byte_map(&paragraph.text);
    let units = paragraph.text.encode_utf16().collect::<Vec<_>>();
    let base = attributed::font(fonts, &paragraph, SpanStyle::new())?;
    let mut raw = Vec::new();
    let mut segments = Vec::new();
    if !units.is_empty() {
        let (string, parts) = attributed::build(fonts, &paragraph, &utf16)?;
        segments = parts;
        // SAFETY : chaîne attribuée valide, possédée pendant la mise en page.
        let typesetter = unsafe { CTTypesetter::with_attributed_string(&string) };
        let breaking = if wrap { f64::from(width.max(1.)) } else { 1e7 };
        let line_of = |start: usize, end: usize| -> crate::Result<CFRetained<CTLine>> {
            if end <= start {
                return empty_line();
            }
            // SAFETY : intervalle non vide dans la chaîne du typesetter.
            Ok(unsafe { typesetter.line(range(start, end - start)) })
        };
        let mut start = 0;
        while start < units.len() {
            // SAFETY : index de départ dans la chaîne.
            let count = unsafe { typesetter.suggest_line_break(start as CFIndex, breaking) };
            let end = (start + count.max(1) as usize).min(units.len());
            let mut visible = end;
            while visible > start && newline(units[visible - 1]) {
                visible -= 1;
            }
            raw.push(Raw {
                line: line_of(start, visible)?,
                start,
                end: visible,
                last: visible < end || end == units.len(),
            });
            start = end;
        }
        // Retour à la ligne final : DirectWrite compte une ligne vide de plus.
        if units.last().is_some_and(|u| newline(*u)) {
            raw.push(Raw {
                line: empty_line()?,
                start: units.len(),
                end: units.len(),
                last: true,
            });
        }
        let token = |segments: &[Segment]| {
            attributed::ellipsis(
                segments.last().map_or(&base, |s| &s.font),
                format.letter_spacing,
            )
        };
        // `line-clamp` : la dernière ligne gardée reprend la suite du texte, coupée par `…`.
        if format.max_lines > 0 && raw.len() > format.max_lines as usize {
            raw.truncate(format.max_lines as usize);
            let last = raw.last_mut().expect("max_lines > 0");
            let mut end = units.len();
            while end > last.start && newline(units[end - 1]) {
                end -= 1;
            }
            let rest = line_of(last.start, end)?;
            // SAFETY : ligne et jeton valides.
            last.line = unsafe {
                rest.truncated_line(
                    f64::from(width),
                    CTLineTruncationType::End,
                    Some(&*token(&segments)?),
                )
            }
            .unwrap_or(rest);
            last.end = end;
            last.last = true;
        }
        if format.ellipsis {
            let sign = token(&segments)?;
            for line in &mut raw {
                if bounds(&line.line).0 - trailing(&line.line) > width + 0.01 {
                    // SAFETY : ligne et jeton valides.
                    if let Some(cut) = unsafe {
                        line.line.truncated_line(
                            f64::from(width),
                            CTLineTruncationType::End,
                            Some(&sign),
                        )
                    } {
                        line.line = cut;
                    }
                }
            }
        }
        if format.align == TextAlign::Justify {
            for line in raw.iter_mut().filter(|l| !l.last) {
                // SAFETY : ligne valide ; facteur 1 (justification complète).
                if let Some(justified) = unsafe { line.line.justified_line(1., f64::from(width)) } {
                    line.line = justified;
                }
            }
        }
    }
    // SAFETY : police valide.
    let natural = unsafe {
        (
            base.ascent() as f32,
            base.descent() as f32,
            base.leading() as f32,
        )
    };
    let placement = |ascent: f32, descent: f32, leading: f32| match format.line_height {
        Some(line) => (
            line,
            ((line - format.size) * 0.5 + format.size * 0.8).clamp(0., line),
        ),
        None => (ascent + descent + leading, ascent),
    };
    let mut lines = Vec::with_capacity(raw.len().max(1));
    let mut top = 0f32;
    if raw.is_empty() {
        let (height, baseline) = placement(natural.0, natural.1, natural.2);
        lines.push(Line {
            line: empty_line()?,
            x: 0.,
            top,
            baseline,
            height,
            width: 0.,
            trailing: 0.,
            start: 0,
            end: 0,
        });
    }
    for r in raw {
        let (line_width, mut ascent, mut descent, mut leading) = bounds(&r.line);
        if ascent + descent <= 0. {
            (ascent, descent, leading) = natural;
        }
        let (height, baseline) = placement(ascent, descent, leading);
        let trailing = trailing(&r.line);
        let content = line_width - trailing;
        let x = match format.align {
            TextAlign::Start | TextAlign::Justify => 0.,
            TextAlign::Center => (width - content) * 0.5,
            TextAlign::End => width - content,
        };
        lines.push(Line {
            line: r.line,
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
        base,
        utf16,
        bytes,
        glyphs: std::cell::OnceCell::new(),
    })
}
fn trailing(line: &CTLine) -> f32 {
    // SAFETY : ligne valide.
    (unsafe { line.trailing_whitespace_width() }) as f32
}
