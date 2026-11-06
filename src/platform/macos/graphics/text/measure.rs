//! Mesure du texte par Core Text (`TextMeasurer`), avec les mêmes mises en page que le
//! dessin.
use super::super::Renderer;
use super::{plain, simple_format};
use crate::ui::{
    element::{SizeF, TextMeasurer},
    font::Font,
    scene::RectF,
    styling::text::{RichParagraph, TextFormat},
    text_input::TextLine,
};
use std::sync::Arc;

impl TextMeasurer for Renderer {
    fn measure_formatted(
        &mut self,
        value: &str,
        format: TextFormat,
        width: f32,
    ) -> crate::Result<SizeF> {
        if !format.advanced() {
            return self.measure_font(value, format.size, format.font, width);
        }
        format.validate()?;
        self.text
            .size(&mut self.fonts, &plain(&Arc::from(value), format), width)
    }
    fn measure_rich(&mut self, paragraph: &RichParagraph, width: f32) -> crate::Result<SizeF> {
        self.text
            .size(&mut self.fonts, paragraph, width.clamp(1., 16384.))
    }
    fn rich_text_rects(
        &mut self,
        paragraph: &RichParagraph,
        width: f32,
        range: std::ops::Range<usize>,
    ) -> crate::Result<Vec<RectF>> {
        paragraph.validate()?;
        if range.start > range.end
            || range.end > paragraph.text.len()
            || !paragraph.text.is_char_boundary(range.start)
            || !paragraph.text.is_char_boundary(range.end)
        {
            return Err(crate::Error::invalid_input("invalid rich text range"));
        }
        let layout = self
            .text
            .layout(&mut self.fonts, paragraph, width.clamp(1., 16384.), true)?;
        // Après une ellipse de début, les octets sont décalés derrière le signe `…`.
        let shift = paragraph.text.len() as isize - layout.paragraph.text.len() as isize;
        let local = |byte: usize| {
            let b = (byte as isize - shift).max(0) as usize;
            layout.utf16[b.min(layout.paragraph.text.len())]
        };
        let (start, end) = (local(range.start), local(range.end));
        let mut rects = Vec::new();
        for line in &layout.lines {
            let (a, b) = (start.max(line.start), end.min(line.end));
            if a >= b {
                continue;
            }
            let (x0, x1) = (line.offset(a), line.offset(b));
            rects.push(RectF::new(
                line.x + x0.min(x1),
                line.top,
                (x1 - x0).abs(),
                line.height,
            ));
        }
        Ok(rects)
    }
    fn layout_line(&mut self, value: &str, size: f32) -> crate::Result<TextLine> {
        self.layout_font_line(value, size, Font::system(false))
    }
    fn layout_font_line(&mut self, value: &str, size: f32, font: Font) -> crate::Result<TextLine> {
        self.layout_formatted_line(value, simple_format(font, size))
    }
    fn layout_formatted_line(
        &mut self,
        value: &str,
        format: TextFormat,
    ) -> crate::Result<TextLine> {
        format.validate()?;
        self.text.line(&mut self.fonts, value, format)
    }
    fn layout_wrapped_text(
        &mut self,
        value: &str,
        format: TextFormat,
        width: f32,
    ) -> crate::Result<Vec<crate::ui::text_input::WrappedLine>> {
        format.validate()?;
        let ranges =
            self.text
                .wrap_ranges(&mut self.fonts, value, format, width.clamp(1., 16384.))?;
        crate::ui::text_input::lines_from_ranges(self, value, format, ranges)
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
    fn measure_font(
        &mut self,
        text: &str,
        size: f32,
        font: Font,
        max_width: f32,
    ) -> crate::Result<SizeF> {
        let max_width = max_width.clamp(1., 16384.);
        self.text.size(
            &mut self.fonts,
            &plain(&Arc::from(text), simple_format(font, size)),
            max_width,
        )
    }
}
