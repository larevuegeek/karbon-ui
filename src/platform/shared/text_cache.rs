//! Cache des mises en page de texte des backends Core Text (macOS) et Pango (Linux) : une
//! mise en page par texte, format, portions, largeur et retour à la ligne, tailles
//! mesurées, et ellipse de début (le texte est coupé au début jusqu'à tenir).
use crate::ui::{
    element::SizeF,
    font::Font,
    styling::text::{RichParagraph, SpanStyle, TextFormat},
};
use std::{collections::HashMap, rc::Rc, sync::Arc};

/// Pile de texte d'un backend : met en page un paragraphe.
pub(in crate::platform) trait Typesetter {
    type Layout;
    /// Mise en page de `paragraph` (déjà coupé au début s'il le faut) à `width`.
    fn typeset(
        &mut self,
        paragraph: RichParagraph,
        width: f32,
        wrap: bool,
    ) -> crate::Result<Self::Layout>;
    /// Taille de la mise en page.
    fn size(layout: &Self::Layout) -> SizeF;
    /// Abscisse de la ligne la plus à gauche (décalage d'alignement), au plus 0 : bornes
    /// de ce que peint un texte centré ou aligné à droite qui déborde de sa boîte.
    fn left(layout: &Self::Layout) -> f32;
}
/// Attributs d'une portion qui changent la mise en page ou le dessin.
type SpanKey = (
    u32,
    u32,
    u16,
    bool,
    u32,
    Option<&'static str>,
    u8,
    Option<u32>,
);
/// Clé d'une mise en page : texte, format, portions, largeur et retour à la ligne.
#[derive(Clone, PartialEq, Eq, Hash)]
struct Key {
    text: Arc<str>,
    font: Font,
    numbers: [u32; 6],
    flags: [u8; 6],
    max_lines: u16,
    spans: Vec<SpanKey>,
    width: u32,
    wrap: bool,
}
fn decoration_bits(d: crate::ui::styling::TextDecoration) -> u8 {
    u8::from(d.underline) | u8::from(d.line_through) << 1 | u8::from(d.overline) << 2
}
fn color_bits(c: crate::ui::theme::Color) -> u32 {
    u32::from_le_bytes([c.red, c.green, c.blue, c.alpha])
}
impl Key {
    fn new(paragraph: &RichParagraph, width: f32, wrap: bool) -> Self {
        let f = paragraph.format;
        let style = f.decoration_style;
        let span = |s: &crate::ui::styling::text::TextSpan| -> SpanKey {
            let SpanStyle {
                weight,
                italic,
                decoration,
                color,
                size,
                family,
            } = s.style;
            (
                s.range.start as u32,
                s.range.end as u32,
                weight.unwrap_or(0),
                italic,
                size.map_or(0, f32::to_bits),
                family,
                decoration_bits(decoration),
                color.map(color_bits),
            )
        };
        let wrap = wrap && f.wraps();
        Self {
            text: Arc::clone(&paragraph.text),
            font: f.font,
            numbers: [
                f.size.to_bits(),
                f.line_height.map_or(0, f32::to_bits),
                f.letter_spacing.to_bits(),
                style.offset.map_or(u32::MAX, f32::to_bits),
                style.thickness.map_or(u32::MAX, f32::to_bits),
                style.color.map_or(0, color_bits),
            ],
            flags: [
                u8::from(f.clip) | u8::from(f.italic) << 1 | u8::from(f.tabular_numbers) << 2,
                u8::from(f.ellipsis) | u8::from(f.ellipsis_start && !wrap) << 1,
                decoration_bits(f.decoration),
                f.align as u8,
                u8::from(style.color.is_some()),
                0,
            ],
            max_lines: f.max_lines,
            spans: paragraph.spans.iter().map(span).collect(),
            width: width.max(0.001).to_bits(),
            wrap,
        }
    }
}
/// Paragraphe sans portion d'un texte simple ou formaté.
pub(in crate::platform) fn plain(text: &Arc<str>, format: TextFormat) -> RichParagraph {
    RichParagraph {
        text: Arc::clone(text),
        format,
        spans: Vec::new(),
    }
}
/// Format d'un texte simple (`Text`, `FontText`, mesures sans format).
pub(in crate::platform) fn simple_format(font: Font, size: f32) -> TextFormat {
    TextFormat::new(Default::default(), font, size)
}
/// Mises en page retenues tant qu'elles servent, et tailles mesurées.
pub(in crate::platform) struct TextCache<T: Typesetter> {
    layouts: HashMap<Key, Rc<T::Layout>>,
    /// Tailles du layout ; les mises en page mesurées à d'autres largeurs que celles du
    /// dessin ne sont pas toutes gardées.
    sizes: HashMap<Key, SizeF>,
}
impl<T: Typesetter> Default for TextCache<T> {
    fn default() -> Self {
        Self {
            layouts: HashMap::new(),
            sizes: HashMap::new(),
        }
    }
}
impl<T: Typesetter> TextCache<T> {
    /// Mise en page de `paragraph` à `width` ; l'ellipse de début coupe d'abord le texte.
    pub fn layout(
        &mut self,
        fonts: &mut T,
        paragraph: &RichParagraph,
        width: f32,
        wrap: bool,
    ) -> crate::Result<Rc<T::Layout>> {
        let key = Key::new(paragraph, width, wrap);
        if let Some(layout) = self.layouts.get(&key) {
            return Ok(Rc::clone(layout));
        }
        paragraph.validate()?;
        let wrap = key.wrap;
        let mut laid = paragraph.clone();
        if paragraph.format.ellipsis_start && !wrap {
            laid.format.ellipsis_start = false;
            if let Some(at) = self.start_cut(fonts, &laid, width)? {
                laid = laid.cut_start(at);
            }
        }
        let layout = Rc::new(fonts.typeset(laid, width, wrap)?);
        if self.layouts.len() >= 512 {
            self.layouts.clear();
        }
        self.layouts.insert(key, Rc::clone(&layout));
        Ok(layout)
    }
    /// Taille mesurée de `paragraph` à `width` (avec retour à la ligne).
    pub fn size(
        &mut self,
        fonts: &mut T,
        paragraph: &RichParagraph,
        width: f32,
    ) -> crate::Result<SizeF> {
        let key = Key::new(paragraph, width, true);
        if let Some(size) = self.sizes.get(&key) {
            return Ok(*size);
        }
        let size = T::size(&*self.layout(fonts, paragraph, width, true)?);
        if self.sizes.len() >= 8192 {
            self.sizes.clear();
        }
        self.sizes.insert(key, size);
        Ok(size)
    }
    /// Ellipse de début : octet où commence la partie gardée, si le texte déborde.
    fn start_cut(
        &mut self,
        fonts: &mut T,
        paragraph: &RichParagraph,
        width: f32,
    ) -> crate::Result<Option<usize>> {
        let mut natural = |cache: &mut Self, candidate: &RichParagraph| -> crate::Result<f32> {
            let mut probe = candidate.clone();
            probe.format.ellipsis = false;
            probe.format.ellipsis_start = false;
            probe.format.max_lines = 0;
            Ok(T::size(&*cache.layout(fonts, &probe, 1_000_000., false)?).width)
        };
        if paragraph.text.is_empty() || natural(self, paragraph)? <= width {
            return Ok(None);
        }
        let bounds = paragraph
            .text
            .char_indices()
            .map(|(i, _)| i)
            .chain([paragraph.text.len()])
            .collect::<Vec<_>>();
        let (mut low, mut high) = (1usize.min(bounds.len() - 1), bounds.len() - 1);
        while low < high {
            let middle = (low + high) / 2;
            if natural(self, &paragraph.cut_start(bounds[middle]))? <= width {
                high = middle;
            } else {
                low = middle + 1;
            }
        }
        Ok(Some(bounds[low]))
    }
}
