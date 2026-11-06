//! DirectWrite layouts shared by measurement and rendering, including font,
//! alignment, line spacing and decorations.
use super::*;
use crate::ui::{
    element::SizeF,
    styling::{
        TextAlign, TextDecoration,
        text::{RichParagraph, TextFormat},
    },
};
mod attributes;
mod line;
mod rich;
mod styled;
use attributes::{features, start_cut};
use rich::{RichEntry, RichKey};

#[derive(Clone, Hash, PartialEq, Eq)]
struct Key {
    value: Arc<str>,
    font: Font,
    size: u32,
    width: u32,
    line: u32,
    align: TextAlign,
    decoration: TextDecoration,
    wrap: bool,
    spacing: u32,
    italic: bool,
    tabular: bool,
    ellipsis: bool,
    ellipsis_start: bool,
    max_lines: u16,
    /// Soulignement dessiné par le moteur (couleur, décalage ou épaisseur propres).
    custom_underline: bool,
}
impl Key {
    fn new(value: Arc<str>, format: TextFormat, width: f32, wrap: bool) -> Self {
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
    fn alignment(&self) -> DWRITE_TEXT_ALIGNMENT {
        match self.align {
            TextAlign::Start => DWRITE_TEXT_ALIGNMENT_LEADING,
            TextAlign::End => DWRITE_TEXT_ALIGNMENT_TRAILING,
            TextAlign::Center => DWRITE_TEXT_ALIGNMENT_CENTER,
            TextAlign::Justify => DWRITE_TEXT_ALIGNMENT_JUSTIFIED,
        }
    }
}
#[derive(Clone)]
struct Entry {
    layout: IDWriteTextLayout,
    size: SizeF,
    overlines: Vec<RectF>,
    /// Soulignements dessinés par le moteur : boîte de chaque morceau de ligne et ligne
    /// de base dans la mise en page.
    underlines: Vec<(RectF, f32)>,
}
#[derive(Default)]
pub(super) struct Cache {
    entries: HashMap<Key, Entry>,
    rich: HashMap<RichKey, RichEntry>,
    /// Tailles mesurées pour le layout. Le dessin ne garde que les mises en page de
    /// l'image courante ; sans ce cache, chaque texte mesuré à une autre largeur que
    /// celle du dessin recréait sa mise en page DirectWrite à chaque image.
    sizes: HashMap<Key, SizeF>,
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
