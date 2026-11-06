//! Texte : chaque glyphe est une boîte aux métriques de `HeadlessTextMeasurer`.
use super::{Canvas, paint::premultiplied, shape::intersect};
use crate::platform::headless::text::{
    HeadlessTextMeasurer, align_lines, formatted_lines, line_height, rich_lines,
};
use crate::ui::{
    scene::RectF,
    styling::{TextAlign, text::TextFormat},
    theme::Color,
};

impl Canvas<'_> {
    /// Boîtes de glyphes : mêmes coupures et métriques que `HeadlessTextMeasurer`.
    pub(super) fn text(
        &mut self,
        rect: RectF,
        text: &str,
        format: &TextFormat,
        color: Color,
        wrap_lines: bool,
    ) {
        let (size, align) = (format.size, format.align);
        if !(size.is_finite() && size > 0.) {
            return;
        }
        let color = self.text_fills.last().copied().unwrap_or(color);
        let advance = HeadlessTextMeasurer::ADVANCE * size + format.letter_spacing;
        let line = line_height(size, format.line_height);
        let width = rect.right - rect.left;
        let lines = formatted_lines(text, format, width, wrap_lines);
        let source = premultiplied(color, self.opacity);
        if source[3] == 0 {
            return;
        }
        let mut boxes = Vec::new();
        for (row, content) in lines.iter().enumerate() {
            let used = content.chars().count() as f32 * advance;
            let start = rect.left
                + match align {
                    TextAlign::Center => ((width - used) / 2.).max(0.),
                    TextAlign::End => (width - used).max(0.),
                    TextAlign::Start | TextAlign::Justify => 0.,
                };
            let top = rect.top + row as f32 * line;
            for (column, c) in content.chars().enumerate() {
                if !c.is_whitespace() {
                    let left = start + column as f32 * advance;
                    boxes.push(RectF::new(
                        left + advance * 0.1,
                        top + line * 0.2,
                        advance * 0.8,
                        line * 0.6,
                    ));
                }
            }
        }
        // Le texte est rogné à sa boîte de mise en page.
        for glyph in boxes {
            let glyph = intersect(glyph, rect);
            if glyph.left < glyph.right && glyph.top < glyph.bottom {
                self.cover(glyph, |p| {
                    (p.x >= glyph.left
                        && p.x < glyph.right
                        && p.y >= glyph.top
                        && p.y < glyph.bottom)
                        .then_some(source)
                });
            }
        }
    }
    /// Paragraphe enrichi : boîtes de glyphes colorées par portion, avec soulignement et
    /// barré en traits de 1 px.
    pub(super) fn rich_text(
        &mut self,
        rect: RectF,
        paragraph: &crate::ui::styling::text::RichParagraph,
        color: Color,
        wrap_lines: bool,
    ) {
        let width = rect.right - rect.left;
        let fill = self.text_fills.last().copied();
        let mut lines = rich_lines(paragraph, width, wrap_lines);
        align_lines(&mut lines, width, paragraph.format.align);
        let base = paragraph.format.decoration;
        let mut boxes = Vec::new();
        for line in &lines {
            for g in &line.glyphs {
                let style = paragraph.style_at(g.start);
                let source = premultiplied(fill.or(style.color).unwrap_or(color), self.opacity);
                let left = rect.left + line.offset + g.x;
                let top = rect.top + line.top;
                if !g.space {
                    boxes.push((
                        RectF::new(
                            left + g.advance * 0.1,
                            top + line.height * 0.2,
                            g.advance * 0.8,
                            line.height * 0.6,
                        ),
                        source,
                    ));
                }
                if style.decoration.underline || base.underline {
                    boxes.push((
                        RectF::new(left, top + line.height * 0.85, g.advance, 1.),
                        source,
                    ));
                }
                if style.decoration.line_through || base.line_through {
                    boxes.push((
                        RectF::new(left, top + line.height * 0.5, g.advance, 1.),
                        source,
                    ));
                }
            }
        }
        for (glyph, source) in boxes {
            if source[3] == 0 {
                continue;
            }
            let glyph = intersect(glyph, rect);
            if glyph.left < glyph.right && glyph.top < glyph.bottom {
                self.cover(glyph, |p| {
                    (p.x >= glyph.left
                        && p.x < glyph.right
                        && p.y >= glyph.top
                        && p.y < glyph.bottom)
                        .then_some(source)
                });
            }
        }
    }
}
