//! Formats de texte DirectWrite par taille et police, et polices embarquées.
use super::*;

impl Renderer {
    pub(super) fn format(&mut self, size: f32, bold: bool) -> crate::Result<()> {
        self.font_format(size, Font::system(bold))
    }
    pub(in crate::platform::windows) fn load_fonts(
        &mut self,
        assets: &[FontAsset],
    ) -> crate::Result<()> {
        if assets.is_empty() {
            return Ok(());
        }
        let fonts = fonts::FontCollection::new(&self.text_factory, assets)?;
        self.formats.clear();
        self.measurements.clear();
        self.typography = Default::default();
        self.filters.clear();
        self.fonts = Some(fonts);
        Ok(())
    }
    pub(super) fn font_format(&mut self, size: f32, font: Font) -> crate::Result<()> {
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
        let key = (size.to_bits(), font);
        if self.formats.contains_key(&key) {
            return Ok(());
        }
        let resolved = fonts::resolve(&self.text_factory, self.fonts.as_ref(), font.family)?;
        let family = HSTRING::from(resolved);
        let locale = w!("fr-FR");
        if let Some(fonts) = &self.fonts
            && fonts.contains(resolved)?
        {
            let format = fonts.format(&family, size, font.weight, locale)?;
            self.formats.insert(key, format);
            return Ok(());
        }
        // SAFETY: chaînes terminées par NUL vivantes pendant l'appel ; collection système.
        let format = unsafe {
            self.text_factory.CreateTextFormat(
                &family,
                None,
                DWRITE_FONT_WEIGHT(font.weight as i32),
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                size,
                locale,
            )
        }
        .check("CreateTextFormat")?;
        self.formats.insert(key, format);
        Ok(())
    }
}
