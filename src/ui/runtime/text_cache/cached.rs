pub(super) use crate::ui::{
    core::hash::FastHasher,
    element::{SizeF, TextMeasurer},
    font::Font,
    scene::RectF,
    styling::{
        TextAlign,
        text::{RichParagraph, TextFormat, TextSpan},
    },
    text_input::{TextLine, WrappedLine},
};
pub(super) use std::{
    collections::HashMap,
    hash::{BuildHasherDefault, Hash, Hasher},
    sync::Arc,
};

/// Marge au-dessus de la ligne la plus large pour reprendre une mesure à une largeur
/// plus petite.
pub(super) const MARGIN: f32 = 0.5;
/// Mesures différentes gardées par texte (largeurs).
pub(super) const PER_TEXT: usize = 8;
/// Au-delà, la mémoire est vidée.
pub(super) const CAPACITY: usize = 8192;

/// Format d'une mesure, comparé bit à bit.
#[derive(Clone, PartialEq, Eq, Hash)]
pub(super) struct Format {
    pub(super) font: Font,
    pub(super) numbers: [u32; 7],
    pub(super) flags: [u8; 9],
    pub(super) spans: Box<[Span]>,
}
pub(super) type Span = (
    u32,
    u32,
    u16,
    bool,
    u8,
    Option<u32>,
    u32,
    Option<&'static str>,
);
pub(super) fn color(c: crate::ui::theme::Color) -> u32 {
    u32::from_le_bytes([c.red, c.green, c.blue, c.alpha])
}
pub(super) fn bits(n: Option<f32>) -> u32 {
    n.map_or(u32::MAX, f32::to_bits)
}
impl Format {
    pub(super) fn of(f: &TextFormat, spans: &[TextSpan]) -> Self {
        let d = |d: crate::ui::styling::TextDecoration| {
            u8::from(d.underline) | u8::from(d.line_through) << 1 | u8::from(d.overline) << 2
        };
        Self {
            font: f.font,
            numbers: [
                f.size.to_bits(),
                bits(f.line_height),
                f.decoration_style.color.map_or(0, color),
                bits(f.decoration_style.offset),
                bits(f.decoration_style.thickness),
                f.letter_spacing.to_bits(),
                u32::from(f.max_lines),
            ],
            flags: [
                u8::from(f.clip),
                f.align as u8,
                d(f.decoration),
                u8::from(f.decoration_style.color.is_some()),
                u8::from(f.italic),
                u8::from(f.tabular_numbers),
                f.transform as u8,
                f.white_space as u8,
                u8::from(f.ellipsis) | u8::from(f.ellipsis_start) << 1,
            ],
            spans: spans
                .iter()
                .map(|s| {
                    (
                        s.range.start as u32,
                        s.range.end as u32,
                        s.style.weight.unwrap_or(0),
                        s.style.italic,
                        d(s.style.decoration),
                        s.style.color.map(color),
                        bits(s.style.size),
                        s.style.family,
                    )
                })
                .collect(),
        }
    }
    /// Police et taille seules (`measure_font`, `measure_text`).
    pub(super) fn plain(font: Font, size: f32, kind: u8) -> Self {
        Self {
            font,
            numbers: [size.to_bits(), 0, 0, 0, 0, 0, 0],
            flags: [kind, 0, 0, 0, 0, 0, 0, 0, 0],
            spans: Box::new([]),
        }
    }
}
/// Méthode du mesureur qui a produit la mesure.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Kind {
    Formatted,
    Rich,
    Font,
    Text,
}
pub(super) struct Entry {
    pub(super) kind: Kind,
    pub(super) text: Arc<str>,
    pub(super) format: Format,
    pub(super) width: f32,
    pub(super) size: SizeF,
}
impl Entry {
    /// La mesure vaut pour `width` (voir le module).
    pub(super) fn serves(&self, width: f32, range: bool) -> bool {
        self.width.to_bits() == width.to_bits()
            || (range && width >= 1. && width <= self.width && self.size.width + MARGIN <= width)
    }
}
/// Mesures gardées, par texte et format.
#[derive(Default)]
pub(crate) struct TextCache {
    /// Échelle d'affichage des mesures gardées : le backend peut caler ses métriques
    /// sur les pixels de l'écran.
    pub(super) scale: u32,
    pub(super) entries: HashMap<u64, Vec<Entry>, BuildHasherDefault<FastHasher>>,
    pub(super) count: usize,
    /// `false` : chaque mesure est demandée au backend (diagnostic).
    pub(crate) disabled: bool,
}
impl TextCache {
    /// Oublie les mesures si l'échelle d'affichage change.
    pub(crate) fn begin(&mut self, scale: f32) {
        if self.scale != scale.to_bits() {
            self.scale = scale.to_bits();
            self.entries.clear();
            self.count = 0;
        }
    }
    pub(super) fn measure(
        &mut self,
        kind: Kind,
        text: &str,
        format: impl FnOnce() -> Format,
        width: f32,
        range: bool,
        measure: impl FnOnce() -> crate::Result<SizeF>,
    ) -> crate::Result<SizeF> {
        if self.disabled {
            return measure();
        }
        let format = format();
        let mut hasher = FastHasher::default();
        (kind, text, &format).hash(&mut hasher);
        let hash = hasher.finish();
        if let Some(entry) = self.entries.get(&hash).and_then(|list| {
            list.iter().find(|e| {
                e.kind == kind && e.serves(width, range) && *e.text == *text && e.format == format
            })
        }) {
            return Ok(entry.size);
        }
        let size = measure()?;
        if self.count >= CAPACITY {
            self.entries.clear();
            self.count = 0;
        }
        let list = self.entries.entry(hash).or_default();
        if list.len() >= PER_TEXT {
            list.remove(0);
            self.count -= 1;
        }
        list.push(Entry {
            kind,
            text: text.into(),
            format,
            width,
            size,
        });
        self.count += 1;
        Ok(size)
    }
}
/// Une mesure de ce format vaut pour une plage de largeurs (voir le module).
pub(super) fn reusable(format: &TextFormat) -> bool {
    matches!(format.align, TextAlign::Start | TextAlign::End) && format.max_lines == 0
}
/// Mesureur du backend précédé de la mémoire : seules les mesures de taille passent
/// par elle, tout le reste (lignes, carets, rectangles des liens) va au backend.
pub(crate) struct Cached<'a> {
    pub(crate) inner: &'a mut dyn TextMeasurer,
    pub(crate) cache: &'a mut TextCache,
}
impl TextMeasurer for Cached<'_> {
    fn measure_formatted(
        &mut self,
        value: &str,
        format: TextFormat,
        width: f32,
    ) -> crate::Result<SizeF> {
        let inner = &mut *self.inner;
        self.cache.measure(
            Kind::Formatted,
            value,
            || Format::of(&format, &[]),
            width,
            reusable(&format),
            || inner.measure_formatted(value, format, width),
        )
    }
    fn measure_rich(&mut self, paragraph: &RichParagraph, width: f32) -> crate::Result<SizeF> {
        let inner = &mut *self.inner;
        self.cache.measure(
            Kind::Rich,
            &paragraph.text,
            || Format::of(&paragraph.format, &paragraph.spans),
            width,
            reusable(&paragraph.format),
            || inner.measure_rich(paragraph, width),
        )
    }
    fn measure_font(
        &mut self,
        text: &str,
        size: f32,
        font: Font,
        max_width: f32,
    ) -> crate::Result<SizeF> {
        let inner = &mut *self.inner;
        self.cache.measure(
            Kind::Font,
            text,
            || Format::plain(font, size, 0),
            max_width,
            true,
            || inner.measure_font(text, size, font, max_width),
        )
    }
    fn measure_text(
        &mut self,
        text: &str,
        size: f32,
        bold: bool,
        max_width: f32,
    ) -> crate::Result<SizeF> {
        let inner = &mut *self.inner;
        self.cache.measure(
            Kind::Text,
            text,
            || Format::plain(Font::system(bold), size, 1),
            max_width,
            true,
            || inner.measure_text(text, size, bold, max_width),
        )
    }
    fn rich_text_rects(
        &mut self,
        paragraph: &RichParagraph,
        width: f32,
        range: std::ops::Range<usize>,
    ) -> crate::Result<Vec<RectF>> {
        self.inner.rich_text_rects(paragraph, width, range)
    }
    fn layout_line(&mut self, value: &str, size: f32) -> crate::Result<TextLine> {
        self.inner.layout_line(value, size)
    }
    fn layout_font_line(&mut self, value: &str, size: f32, font: Font) -> crate::Result<TextLine> {
        self.inner.layout_font_line(value, size, font)
    }
    fn layout_formatted_line(
        &mut self,
        value: &str,
        format: TextFormat,
    ) -> crate::Result<TextLine> {
        self.inner.layout_formatted_line(value, format)
    }
    fn layout_wrapped_text(
        &mut self,
        value: &str,
        format: TextFormat,
        width: f32,
    ) -> crate::Result<Vec<WrappedLine>> {
        self.inner.layout_wrapped_text(value, format, width)
    }
}
