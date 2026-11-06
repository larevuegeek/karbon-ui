//! Mesure du texte par DirectWrite (`TextMeasurer`), avec les mêmes formats et mises en
//! page que le dessin.
use super::*;

impl crate::ui::element::TextMeasurer for Renderer {
    fn measure_formatted(
        &mut self,
        value: &str,
        format: crate::ui::styling::text::TextFormat,
        width: f32,
    ) -> crate::Result<crate::ui::element::SizeF> {
        if !format.advanced() {
            return self.measure_font(value, format.size, format.font, width);
        }
        self.font_format(format.size, format.font)?;
        self.typography.measure(
            &self.text_factory,
            &self.formats[&(format.size.to_bits(), format.font)],
            value,
            format,
            width,
        )
    }
    fn measure_rich(
        &mut self,
        paragraph: &crate::ui::styling::text::RichParagraph,
        width: f32,
    ) -> crate::Result<crate::ui::element::SizeF> {
        paragraph.validate()?;
        let format = paragraph.format;
        self.font_format(format.size, format.font)?;
        self.typography.measure_rich(
            &self.text_factory,
            &self.formats[&(format.size.to_bits(), format.font)],
            self.fonts.as_ref(),
            paragraph,
            width.clamp(1., 16384.),
        )
    }
    fn rich_text_rects(
        &mut self,
        paragraph: &crate::ui::styling::text::RichParagraph,
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
        let format = paragraph.format;
        self.font_format(format.size, format.font)?;
        self.typography.rich_rects(
            &self.text_factory,
            &self.formats[&(format.size.to_bits(), format.font)],
            self.fonts.as_ref(),
            paragraph,
            width.clamp(1., 16384.),
            range,
        )
    }
    fn layout_line(
        &mut self,
        value: &str,
        size: f32,
    ) -> crate::Result<crate::ui::text_input::TextLine> {
        self.layout_font_line(value, size, Font::system(false))
    }
    fn layout_font_line(
        &mut self,
        value: &str,
        size: f32,
        font: Font,
    ) -> crate::Result<crate::ui::text_input::TextLine> {
        self.layout_formatted_line(
            value,
            crate::ui::styling::text::TextFormat::new(Default::default(), font, size),
        )
    }
    fn layout_formatted_line(
        &mut self,
        value: &str,
        format: crate::ui::styling::text::TextFormat,
    ) -> crate::Result<crate::ui::text_input::TextLine> {
        format.validate()?;
        self.font_format(format.size, format.font)?;
        self.typography.line(
            &self.text_factory,
            &self.formats[&(format.size.to_bits(), format.font)],
            value,
            format,
        )
    }
    fn layout_wrapped_text(
        &mut self,
        value: &str,
        format: crate::ui::styling::text::TextFormat,
        width: f32,
    ) -> crate::Result<Vec<crate::ui::text_input::WrappedLine>> {
        format.validate()?;
        self.font_format(format.size, format.font)?;
        let ranges = self.typography.wrap_ranges(
            &self.text_factory,
            &self.formats[&(format.size.to_bits(), format.font)],
            value,
            format,
            width.clamp(1., 16384.),
        )?;
        crate::ui::text_input::lines_from_ranges(self, value, format, ranges)
    }
    fn measure_text(
        &mut self,
        text: &str,
        size: f32,
        bold: bool,
        max_width: f32,
    ) -> crate::Result<crate::ui::element::SizeF> {
        self.measure_font(text, size, Font::system(bold), max_width)
    }
    fn measure_font(
        &mut self,
        text: &str,
        size: f32,
        font: Font,
        max_width: f32,
    ) -> crate::Result<crate::ui::element::SizeF> {
        let max_width = max_width.clamp(1., 16384.);
        let key = (text.to_owned(), size.to_bits(), font, max_width.to_bits());
        if let Some(value) = self.measurements.get(&key) {
            return Ok(*value);
        }
        self.font_format(size, font)?;
        let format = &self.formats[&(size.to_bits(), font)];
        // SAFETY: format DirectWrite valide, possédé par le cache.
        unsafe { format.SetWordWrapping(DWRITE_WORD_WRAPPING_WRAP) }
            .check("SetWordWrapping (measure)")?;
        let utf16: Vec<u16> = text.encode_utf16().collect();
        // SAFETY: texte UTF-16 et format vivants pendant l'appel.
        let layout = unsafe {
            self.text_factory
                .CreateTextLayout(&utf16, format, max_width, 16384.)
        }
        .check("CreateTextLayout")?;
        let mut metrics = DWRITE_TEXT_METRICS::default();
        // SAFETY: sortie valide pour la durée de l'appel.
        unsafe { layout.GetMetrics(&mut metrics) }.check("GetMetrics")?;
        let value = crate::ui::element::SizeF {
            width: metrics.widthIncludingTrailingWhitespace,
            height: metrics.height,
        };
        // Cache de mesures borné ; la vue conserve sa géométrie entre les événements.
        if self.measurements.len() >= 256 {
            self.measurements.clear();
        }
        self.measurements.insert(key, value);
        Ok(value)
    }
}
