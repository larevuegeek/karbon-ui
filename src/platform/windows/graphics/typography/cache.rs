pub(super) use super::super::*;
pub(super) use super::attributes::{features, start_cut};
pub(super) use super::rich::{RichEntry, RichKey};
pub(super) use crate::ui::{
    element::SizeF,
    styling::{
        TextAlign, TextDecoration,
        text::{RichParagraph, TextFormat},
    },
};

#[derive(Clone, Hash, PartialEq, Eq)]
pub(super) struct Key {
    pub(super) value: Arc<str>,
    pub(super) font: Font,
    pub(super) size: u32,
    pub(super) width: u32,
    pub(super) line: u32,
    pub(super) align: TextAlign,
    pub(super) decoration: TextDecoration,
    pub(super) wrap: bool,
    pub(super) spacing: u32,
    pub(super) italic: bool,
    pub(super) tabular: bool,
    pub(super) ellipsis: bool,
    pub(super) ellipsis_start: bool,
    pub(super) max_lines: u16,
    /// Soulignement dessiné par le moteur (couleur, décalage ou épaisseur propres).
    pub(super) custom_underline: bool,
}
impl Key {
    pub(super) fn new(value: Arc<str>, format: TextFormat, width: f32, wrap: bool) -> Self {
        Self {
            value,
            font: format.font,
            size: format.size.to_bits(),
            width: width.max(0.001).to_bits(),
            line: format.line_height.map(f32::to_bits).unwrap_or(0),
            align: format.align,
            decoration: format.decoration,
            wrap: wrap && format.wraps(),
            spacing: format.letter_spacing.to_bits(),
            italic: format.italic,
            tabular: format.tabular_numbers,
            ellipsis: format.ellipsis,
            ellipsis_start: format.ellipsis_start && !(wrap && format.wraps()),
            max_lines: format.max_lines,
            custom_underline: format.decoration.underline && format.decoration_style.custom(),
        }
    }
    /// Alignement DirectWrite du texte (`text-align`).
    pub(super) fn alignment(&self) -> DWRITE_TEXT_ALIGNMENT {
        match self.align {
            TextAlign::Start => DWRITE_TEXT_ALIGNMENT_LEADING,
            TextAlign::End => DWRITE_TEXT_ALIGNMENT_TRAILING,
            TextAlign::Center => DWRITE_TEXT_ALIGNMENT_CENTER,
            TextAlign::Justify => DWRITE_TEXT_ALIGNMENT_JUSTIFIED,
        }
    }
}
#[derive(Clone)]
pub(super) struct Entry {
    pub(super) layout: IDWriteTextLayout,
    pub(super) size: SizeF,
    pub(super) overlines: Vec<RectF>,
    /// Soulignements dessinés par le moteur : boîte de chaque morceau de ligne et ligne
    /// de base dans la mise en page.
    pub(super) underlines: Vec<(RectF, f32)>,
}
#[derive(Default)]
pub(crate) struct Cache {
    pub(super) entries: HashMap<Key, Entry>,
    pub(super) rich: HashMap<RichKey, RichEntry>,
    /// Tailles mesurées pour le layout. Le dessin ne garde que les mises en page de
    /// l'image courante ; sans ce cache, chaque texte mesuré à une autre largeur que
    /// celle du dessin recréait sa mise en page DirectWrite à chaque image.
    pub(super) sizes: HashMap<Key, SizeF>,
}
impl Cache {
    pub fn measure(
        &mut self,
        factory: &IDWriteFactory,
        native: &IDWriteTextFormat,
        value: &str,
        format: TextFormat,
        width: f32,
    ) -> crate::Result<SizeF> {
        let key = Key::new(Arc::from(value), format, width, true);
        if let Some(size) = self.sizes.get(&key) {
            return Ok(*size);
        }
        if self.entries.len() >= 512 {
            self.entries.clear();
        }
        if self.sizes.len() >= 8192 {
            self.sizes.clear();
        }
        let size = self.ensure(factory, native, key.clone())?.size;
        self.sizes.insert(key, size);
        Ok(size)
    }
    pub fn prepare(
        &mut self,
        factory: &IDWriteFactory,
        formats: &HashMap<(u32, Font), IDWriteTextFormat>,
        fonts: Option<&fonts::FontCollection>,
        scene: &Scene,
    ) -> crate::Result<()> {
        let mut used = std::collections::HashSet::new();
        let mut used_rich = std::collections::HashSet::new();
        for command in &scene.commands {
            if let DrawCommand::RichText {
                rect,
                paragraph,
                wrap,
                ..
            } = command
            {
                let key = RichKey::new(paragraph, rect.right - rect.left, *wrap);
                used_rich.insert(key.clone());
                self.ensure_rich(
                    factory,
                    &formats[&(paragraph.format.size.to_bits(), paragraph.format.font)],
                    fonts,
                    paragraph,
                    key,
                )?;
            }
            if let DrawCommand::FormattedText {
                rect,
                text,
                format,
                wrap,
                ..
            } = command
            {
                let key = Key::new(Arc::clone(text), *format, rect.right - rect.left, *wrap);
                used.insert(key.clone());
                self.ensure(
                    factory,
                    &formats[&(format.size.to_bits(), format.font)],
                    key,
                )?;
            }
        }
        self.entries.retain(|key, _| used.contains(key));
        self.rich.retain(|key, _| used_rich.contains(key));
        Ok(())
    }
}
