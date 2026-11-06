//! Paragraphes enrichis : placement des caractères par portion, coupure, ellipse et
//! alignement des lignes.
use super::{HeadlessTextMeasurer, clamp_width, line_height};
use crate::ui::styling::{TextAlign, text::RichParagraph};

/// Caractère placé d'un paragraphe enrichi (coordonnées relatives à la ligne).
pub(in crate::platform::headless) struct Glyph {
    pub start: usize,
    pub end: usize,
    pub x: f32,
    pub advance: f32,
    pub size: f32,
    pub space: bool,
}
/// Ligne d'un paragraphe enrichi : mêmes règles de coupure que
/// [`wrap`](super::lines::wrap), avec une avance propre à la taille de chaque caractère.
pub(in crate::platform::headless) struct RichLine {
    pub glyphs: Vec<Glyph>,
    pub top: f32,
    pub height: f32,
    pub width: f32,
    pub offset: f32,
}
pub(in crate::platform::headless) fn rich_lines(
    paragraph: &RichParagraph,
    width: f32,
    wrap_lines: bool,
) -> Vec<RichLine> {
    let spacing = paragraph.format.letter_spacing;
    let limit = if wrap_lines && paragraph.format.wraps() {
        clamp_width(width)
    } else {
        f32::INFINITY
    };
    let explicit = paragraph.format.line_height;
    let mut lines: Vec<Vec<Glyph>> = Vec::new();
    let text = &paragraph.text;
    let mut start = 0;
    for part in text.split('\n') {
        let end = start + part.len();
        let body = part.strip_suffix('\r').unwrap_or(part);
        let mut current: Vec<Glyph> = Vec::new();
        let mut x = 0.;
        let mut skipping = false;
        let mut wrapped = false;
        for (offset, c) in body.char_indices() {
            let byte = start + offset;
            let size = paragraph.size_at(byte);
            let advance = HeadlessTextMeasurer::ADVANCE * size + spacing;
            let space = c == ' ';
            if skipping && space {
                continue;
            }
            skipping = false;
            if x + advance > limit + 1e-3 * advance && !current.is_empty() {
                wrapped = true;
                if space {
                    // L'espace de coupure n'est pas compté.
                    lines.push(std::mem::take(&mut current));
                    x = 0.;
                    skipping = true;
                    continue;
                }
                let cut = current
                    .iter()
                    .rposition(|g| g.space)
                    .filter(|&i| current[..i].iter().any(|g| !g.space));
                if let Some(index) = cut {
                    let mut rest = current.split_off(index);
                    let mut line = std::mem::take(&mut current);
                    while line.last().is_some_and(|g| g.space) {
                        line.pop();
                    }
                    lines.push(line);
                    while rest.first().is_some_and(|g| g.space) {
                        rest.remove(0);
                    }
                    let shift = rest.first().map_or(0., |g| g.x);
                    for g in &mut rest {
                        g.x -= shift;
                    }
                    x = rest.last().map_or(0., |g| g.x + g.advance);
                    current = rest;
                } else {
                    lines.push(std::mem::take(&mut current));
                    x = 0.;
                }
            }
            current.push(Glyph {
                start: byte,
                end: byte + c.len_utf8(),
                x,
                advance,
                size,
                space,
            });
            x += advance;
        }
        if !current.is_empty() || !wrapped {
            lines.push(current);
        }
        start = end + 1;
    }
    // Ellipse et limite de lignes : derniers caractères remplacés par un glyphe `…`.
    let max = paragraph.format.max_lines as usize;
    let cut = max > 0 && lines.len() > max;
    if cut {
        lines.truncate(max);
    }
    if cut || paragraph.format.ellipsis || max > 0 {
        let limit = clamp_width(width);
        let count = lines.len();
        for (index, line) in lines.iter_mut().enumerate() {
            let overflows = line.last().is_some_and(|g| g.x + g.advance > limit + 1e-3);
            if !overflows && !(cut && index + 1 == count) {
                continue;
            }
            if paragraph.format.ellipsis_start && !cut && count == 1 {
                // Ellipse de début : glyphes retirés en tête, `…` placé à gauche.
                let size = line.first().map_or(paragraph.format.size, |g| g.size);
                let advance = HeadlessTextMeasurer::ADVANCE * size + spacing;
                let end = line.last().map_or(0., |g| g.x + g.advance);
                while line
                    .first()
                    .is_some_and(|g| g.space || end - g.x + advance > limit + 1e-3)
                {
                    line.remove(0);
                }
                let shift = line.first().map_or(end, |g| g.x) - advance;
                for g in line.iter_mut() {
                    g.x -= shift;
                }
                let at = line.first().map_or(0, |g| g.start);
                line.insert(
                    0,
                    Glyph {
                        start: at,
                        end: at,
                        x: 0.,
                        advance,
                        size,
                        space: false,
                    },
                );
                continue;
            }
            let size = line.last().map_or(paragraph.format.size, |g| g.size);
            let advance = HeadlessTextMeasurer::ADVANCE * size + spacing;
            while line
                .last()
                .is_some_and(|g| g.space || g.x + g.advance + advance > limit + 1e-3)
            {
                line.pop();
            }
            let (x, at) = line.last().map_or((0., 0), |g| (g.x + g.advance, g.end));
            line.push(Glyph {
                start: at,
                end: at,
                x,
                advance,
                size,
                space: false,
            });
        }
    }
    let mut top = 0.;
    lines
        .into_iter()
        .map(|glyphs| {
            let size = glyphs
                .iter()
                .map(|g| g.size)
                .fold(None, |a: Option<f32>, b| Some(a.map_or(b, |a| a.max(b))))
                .unwrap_or(paragraph.format.size);
            let height = line_height(size, explicit);
            let width = glyphs.last().map_or(0., |g| g.x + g.advance);
            let line = RichLine {
                glyphs,
                top,
                height,
                width,
                offset: 0.,
            };
            top += height;
            line
        })
        .collect()
}
/// Décalage horizontal des lignes selon l'alignement du paragraphe.
pub(in crate::platform::headless) fn align_lines(
    lines: &mut [RichLine],
    width: f32,
    align: TextAlign,
) {
    for line in lines {
        line.offset = match align {
            TextAlign::Center => ((width - line.width) / 2.).max(0.),
            TextAlign::End => (width - line.width).max(0.),
            TextAlign::Start | TextAlign::Justify => 0.,
        };
    }
}
