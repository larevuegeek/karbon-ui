//! Implémentation de `TextMeasurer` : mesures, rectangles de sélection et lignes à
//! carets aux métriques fixes.
use super::{
    HeadlessTextMeasurer, align_lines, clamp_width, formatted_lines, line_height,
    lines::{max_chars, wrap},
    rich_lines,
};
use crate::ui::{
    element::{SizeF, TextMeasurer},
    font::Font,
    scene::RectF,
    styling::text::{RichParagraph, TextFormat},
    text_input::{TextCaret, TextCluster, TextLine, WrappedLine},
};

fn check(size: f32, font: Font) -> crate::Result<()> {
    if !(1..=999).contains(&font.weight)
        || font.family.contains('\0')
        || font.family.is_empty()
        || !size.is_finite()
        || size <= 0.
    {
        return Err(crate::Error::invalid_input(
            "invalid font family, weight or size",
        ));
    }
    Ok(())
}

fn measure(text: &str, size: f32, explicit: Option<f32>, width: f32) -> SizeF {
    let lines = wrap(text, max_chars(width, size));
    let widest = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    SizeF {
        width: widest as f32 * HeadlessTextMeasurer::ADVANCE * size,
        height: lines.len().max(1) as f32 * line_height(size, explicit),
    }
}

fn line(value: &str, size: f32, explicit: Option<f32>) -> TextLine {
    let advance = HeadlessTextMeasurer::ADVANCE * size;
    let height = line_height(size, explicit);
    let mut carets = vec![TextCaret { byte: 0, x: 0. }];
    let mut clusters = Vec::new();
    for (index, (byte, c)) in value.char_indices().enumerate() {
        let x = index as f32 * advance;
        let end = byte + c.len_utf8();
        carets.push(TextCaret {
            byte: end,
            x: x + advance,
        });
        clusters.push(TextCluster {
            start: byte,
            end,
            rect: RectF::new(x, 0., advance, height),
        });
    }
    TextLine {
        width: clusters.len() as f32 * advance,
        carets,
        clusters,
        height,
    }
}

impl TextMeasurer for HeadlessTextMeasurer {
    fn measure_rich(&mut self, paragraph: &RichParagraph, width: f32) -> crate::Result<SizeF> {
        paragraph.validate()?;
        let lines = rich_lines(paragraph, width, true);
        Ok(SizeF {
            width: lines.iter().map(|l| l.width).fold(0., f32::max),
            height: lines.iter().map(|l| l.height).sum(),
        })
    }
    fn rich_text_rects(
        &mut self,
        paragraph: &RichParagraph,
        width: f32,
        range: std::ops::Range<usize>,
    ) -> crate::Result<Vec<RectF>> {
        paragraph.validate()?;
        let mut lines = rich_lines(paragraph, width, true);
        align_lines(&mut lines, clamp_width(width), paragraph.format.align);
        Ok(lines
            .iter()
            .filter_map(|line| {
                let inside = line
                    .glyphs
                    .iter()
                    .filter(|g| g.start < range.end && g.end > range.start);
                let (left, right) = inside.fold(None, |a: Option<(f32, f32)>, g| {
                    Some(a.map_or((g.x, g.x + g.advance), |(l, r)| {
                        (l.min(g.x), r.max(g.x + g.advance))
                    }))
                })?;
                Some(RectF::new(
                    line.offset + left,
                    line.top,
                    right - left,
                    line.height,
                ))
            })
            .collect())
    }
    fn measure_formatted(
        &mut self,
        value: &str,
        format: TextFormat,
        width: f32,
    ) -> crate::Result<SizeF> {
        check(format.size, format.font)?;
        let lines = formatted_lines(value, &format, width, true);
        let advance = HeadlessTextMeasurer::ADVANCE * format.size + format.letter_spacing;
        let widest = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
        Ok(SizeF {
            width: (widest as f32 * advance).max(0.),
            height: lines.len().max(1) as f32 * line_height(format.size, format.line_height),
        })
    }
    fn layout_line(&mut self, value: &str, size: f32) -> crate::Result<TextLine> {
        self.layout_font_line(value, size, Font::system(false))
    }
    fn layout_font_line(&mut self, value: &str, size: f32, font: Font) -> crate::Result<TextLine> {
        check(size, font)?;
        Ok(line(value, size, None))
    }
    fn layout_formatted_line(
        &mut self,
        value: &str,
        format: TextFormat,
    ) -> crate::Result<TextLine> {
        check(format.size, format.font)?;
        let mut result = line(value, format.size, format.line_height);
        if format.letter_spacing != 0. {
            let advance = HeadlessTextMeasurer::ADVANCE * format.size + format.letter_spacing;
            for (index, cluster) in result.clusters.iter_mut().enumerate() {
                cluster.rect = RectF::new(index as f32 * advance, 0., advance, result.height);
            }
            for (index, caret) in result.carets.iter_mut().enumerate() {
                caret.x = index as f32 * advance;
            }
            result.width = result.clusters.len() as f32 * advance;
        }
        Ok(result)
    }
    fn layout_wrapped_text(
        &mut self,
        value: &str,
        format: TextFormat,
        width: f32,
    ) -> crate::Result<Vec<WrappedLine>> {
        check(format.size, format.font)?;
        // Même coupure que la mesure : après le dernier espace qui tient, mot trop long
        // coupé entre deux caractères ; les espaces de coupure restent en fin de ligne.
        let advance = HeadlessTextMeasurer::ADVANCE * format.size + format.letter_spacing;
        let fit = if advance > 0. {
            (clamp_width(width) / advance + 1e-3).floor() as usize
        } else {
            usize::MAX
        };
        let base = value.as_ptr() as usize;
        let ranges = wrap(value, fit)
            .into_iter()
            .map(|line| {
                let start = line.as_ptr() as usize - base;
                start..start + line.len()
            })
            .collect();
        crate::ui::text_input::lines_from_ranges(self, value, format, ranges)
    }
    fn measure_font(
        &mut self,
        text: &str,
        size: f32,
        font: Font,
        max_width: f32,
    ) -> crate::Result<SizeF> {
        check(size, font)?;
        Ok(measure(text, size, None, max_width))
    }
    fn measure_text(
        &mut self,
        text: &str,
        size: f32,
        bold: bool,
        max_width: f32,
    ) -> crate::Result<SizeF> {
        self.measure_font(text, size, Font::system(bold), max_width)
    }
}
