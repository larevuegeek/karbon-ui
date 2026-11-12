//! Paragraphes enrichis (`RichText`) : portions de styles différents, ellipse de début,
//! rectangles de sélection et couleurs par portion.
use super::*;

/// Attributs de mise en page d'une portion : bornes, graisse, italique, taille, famille,
/// soulignement et barré.
type SpanKey = (u32, u32, u16, bool, u32, Option<&'static str>, bool, bool);
/// Clé d'un paragraphe enrichi : seuls les attributs de mise en page comptent ;
/// les couleurs sont appliquées au dessin.
#[derive(Clone, Hash, PartialEq, Eq)]
pub(super) struct RichKey {
    base: Key,
    spans: Vec<SpanKey>,
}
impl RichKey {
    pub(super) fn new(paragraph: &RichParagraph, width: f32, wrap: bool) -> Self {
        Self {
            base: Key::new(Arc::clone(&paragraph.text), paragraph.format, width, wrap),
            spans: paragraph
                .spans
                .iter()
                .map(|s| {
                    (
                        s.range.start as u32,
                        s.range.end as u32,
                        s.style.weight.unwrap_or(0),
                        s.style.italic,
                        s.style.size.map_or(0, f32::to_bits),
                        s.style.family,
                        s.style.decoration.underline,
                        s.style.decoration.line_through,
                    )
                })
                .collect(),
        }
    }
}
#[derive(Clone)]
pub(super) struct RichEntry {
    layout: IDWriteTextLayout,
    size: SizeF,
    /// Position UTF-16 de chaque octet UTF-8 (frontières de caractères).
    utf16: Vec<u32>,
    /// Paragraphe réellement mis en page après une ellipse de début.
    cut: Option<RichParagraph>,
}
fn utf16_map(text: &str) -> Vec<u32> {
    let mut map = vec![0u32; text.len() + 1];
    let mut unit = 0u32;
    for (byte, c) in text.char_indices() {
        map[byte] = unit;
        unit += c.len_utf16() as u32;
    }
    map[text.len()] = unit;
    map
}
impl Cache {
    pub(super) fn ensure_rich(
        &mut self,
        factory: &IDWriteFactory,
        format_native: &IDWriteTextFormat,
        fonts: Option<&fonts::FontCollection>,
        paragraph: &RichParagraph,
        key: RichKey,
    ) -> crate::Result<&RichEntry> {
        if !self.rich.contains_key(&key)
            && key.base.ellipsis_start
            && let Some(at) = self.rich_start_cut(factory, format_native, fonts, paragraph, &key)?
        {
            let mut cut = paragraph.cut_start(at);
            cut.format.ellipsis_start = false;
            let width = f32::from_bits(key.base.width);
            let inner = RichKey::new(&cut, width, key.base.wrap);
            let mut entry = self
                .ensure_rich(factory, format_native, fonts, &cut, inner)?
                .clone();
            entry.cut = Some(cut);
            self.rich.insert(key.clone(), entry);
        }
        if !self.rich.contains_key(&key) {
            let value = paragraph.text.encode_utf16().collect::<Vec<_>>();
            let base = &key.base;
            // SAFETY: texte UTF-16 et format vivants pendant l'appel.
            let layout = unsafe {
                factory.CreateTextLayout(
                    &value,
                    format_native,
                    f32::from_bits(base.width),
                    1_000_000.,
                )
            }
            .check("CreateTextLayout rich")?;
            let align = base.alignment();
            // SAFETY: mise en page fraîchement créée, possédée localement.
            unsafe { layout.SetTextAlignment(align) }.check("SetTextAlignment rich")?;
            // SAFETY: idem.
            unsafe { layout.SetWordWrapping(wrapping(base.wrap)) }.check("SetWordWrapping rich")?;
            if base.line != 0 {
                let line = f32::from_bits(base.line);
                let size = f32::from_bits(base.size);
                let baseline = ((line - size) * 0.5 + size * 0.8).clamp(0., line);
                // SAFETY: idem.
                unsafe {
                    layout.SetLineSpacing(DWRITE_LINE_SPACING_METHOD_UNIFORM, line, baseline)
                }
                .check("SetLineSpacing rich")?;
            }
            let utf16 = utf16_map(&paragraph.text);
            let whole = DWRITE_TEXT_RANGE {
                startPosition: 0,
                length: value.len() as u32,
            };
            let clamped = features(factory, &layout, base, value.len() as u32)?;
            if base.decoration.underline {
                // SAFETY: idem.
                unsafe { layout.SetUnderline(true, whole) }.check("SetUnderline rich")?;
            }
            if base.decoration.line_through {
                // SAFETY: idem.
                unsafe { layout.SetStrikethrough(true, whole) }.check("SetStrikethrough rich")?;
            }
            for span in &paragraph.spans {
                let start = utf16[span.range.start];
                let range = DWRITE_TEXT_RANGE {
                    startPosition: start,
                    length: utf16[span.range.end] - start,
                };
                let s = span.style;
                // SAFETY (bloc) : mise en page possédée localement ; chaînes vivantes
                // pendant chaque appel.
                unsafe {
                    if let Some(weight) = s.weight {
                        layout
                            .SetFontWeight(DWRITE_FONT_WEIGHT(weight as i32), range)
                            .check("SetFontWeight")?;
                    }
                    if s.italic {
                        layout
                            .SetFontStyle(DWRITE_FONT_STYLE_ITALIC, range)
                            .check("SetFontStyle")?;
                    }
                    if s.decoration.underline {
                        layout.SetUnderline(true, range).check("SetUnderline")?;
                    }
                    if s.decoration.line_through {
                        layout
                            .SetStrikethrough(true, range)
                            .check("SetStrikethrough")?;
                    }
                    if let Some(size) = s.size {
                        layout.SetFontSize(size, range).check("SetFontSize")?;
                    }
                    if let Some(family) = s.family {
                        let family = fonts::resolve(factory, fonts, family)?;
                        layout
                            .SetFontFamilyName(&HSTRING::from(family), range)
                            .check("SetFontFamilyName")?;
                        if let Some(fonts) = fonts
                            && fonts.contains(family)?
                        {
                            layout
                                .SetFontCollection(fonts.collection.as_ref(), range)
                                .check("SetFontCollection")?;
                        }
                    }
                }
            }
            let mut metrics = DWRITE_TEXT_METRICS::default();
            // SAFETY: sortie valide pour la durée de l'appel.
            unsafe { layout.GetMetrics(&mut metrics) }.check("GetMetrics rich")?;
            self.rich.insert(
                key.clone(),
                RichEntry {
                    layout,
                    size: SizeF {
                        width: metrics.widthIncludingTrailingWhitespace,
                        height: clamped.unwrap_or(metrics.height),
                    },
                    utf16,
                    cut: None,
                },
            );
        }
        Ok(&self.rich[&key])
    }
    /// Ellipse de début d'un paragraphe enrichi : largeurs mesurées avec ses portions
    /// (familles, tailles, graisses), par dichotomie sur les frontières de caractères.
    fn rich_start_cut(
        &mut self,
        factory: &IDWriteFactory,
        format_native: &IDWriteTextFormat,
        fonts: Option<&fonts::FontCollection>,
        paragraph: &RichParagraph,
        key: &RichKey,
    ) -> crate::Result<Option<usize>> {
        let width = f32::from_bits(key.base.width);
        let natural = |cache: &mut Self, candidate: &RichParagraph| -> crate::Result<f32> {
            let mut candidate = candidate.clone();
            candidate.format.ellipsis_start = false;
            candidate.format.ellipsis = false;
            let probe = RichKey::new(&candidate, 1_000_000., false);
            Ok(cache
                .ensure_rich(factory, format_native, fonts, &candidate, probe)?
                .size
                .width)
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
    pub fn measure_rich(
        &mut self,
        factory: &IDWriteFactory,
        native: &IDWriteTextFormat,
        fonts: Option<&fonts::FontCollection>,
        paragraph: &RichParagraph,
        width: f32,
    ) -> crate::Result<SizeF> {
        if self.rich.len() >= 256 {
            self.rich.clear();
        }
        let key = RichKey::new(paragraph, width, true);
        Ok(self
            .ensure_rich(factory, native, fonts, paragraph, key)?
            .size)
    }
    pub fn rich_rects(
        &mut self,
        factory: &IDWriteFactory,
        native: &IDWriteTextFormat,
        fonts: Option<&fonts::FontCollection>,
        paragraph: &RichParagraph,
        width: f32,
        range: std::ops::Range<usize>,
    ) -> crate::Result<Vec<RectF>> {
        if self.rich.len() >= 256 {
            self.rich.clear();
        }
        let key = RichKey::new(paragraph, width, true);
        let entry = self.ensure_rich(factory, native, fonts, paragraph, key)?;
        // Après une ellipse de début, la mise en page porte le paragraphe coupé : les
        // octets du paragraphe d'origine sont décalés derrière le signe « … ».
        let limit = entry.utf16.len() - 1;
        let shift = entry.cut.as_ref().map_or(0, |cut| {
            paragraph.text.len() as isize - cut.text.len() as isize
        });
        let local = |byte: usize| ((byte as isize - shift).max(0) as usize).min(limit);
        let start = entry.utf16[local(range.start)];
        let length = entry.utf16[local(range.end)].saturating_sub(start);
        if length == 0 {
            return Ok(Vec::new());
        }
        let mut hits = vec![DWRITE_HIT_TEST_METRICS::default(); length as usize + 1];
        let mut count = 0;
        // SAFETY: tampon de sortie et compteur valides pour la durée de l'appel.
        unsafe {
            entry
                .layout
                .HitTestTextRange(start, length, 0., 0., Some(&mut hits), &mut count)
        }
        .check("HitTestTextRange rich")?;
        Ok(hits
            .into_iter()
            .take(count as usize)
            .map(|h| RectF::new(h.left, h.top, h.width, h.height))
            .collect())
    }
    /// Paragraphe enrichi : couleurs par portion via des pinceaux d'effet de dessin.
    pub fn draw_rich(
        &self,
        target: &ID2D1RenderTarget,
        brush: &ID2D1Brush,
        rect: RectF,
        paragraph: &RichParagraph,
        wrap: bool,
    ) -> crate::Result<()> {
        let key = RichKey::new(paragraph, rect.right - rect.left, wrap);
        let Some(entry) = self.rich.get(&key) else {
            return Ok(());
        };
        let layout = &entry.layout;
        for span in &entry.cut.as_ref().unwrap_or(paragraph).spans {
            let start = entry.utf16[span.range.start];
            let range = DWRITE_TEXT_RANGE {
                startPosition: start,
                length: entry.utf16[span.range.end] - start,
            };
            let effect: Option<windows::core::IUnknown> = match span.style.color {
                Some(color) => {
                    // SAFETY: cible de rendu vivante ; couleur lue pendant l'appel.
                    let brush = unsafe { target.CreateSolidColorBrush(&color_f(color), None) }
                        .check("CreateSolidColorBrush rich")?;
                    Some(brush.cast().check("QueryInterface IUnknown")?)
                }
                None => None,
            };
            // SAFETY: mise en page du cache ; effet facultatif vivant pendant l'appel.
            unsafe { layout.SetDrawingEffect(effect.as_ref(), range) }.check("SetDrawingEffect")?;
        }
        // SAFETY: passe de dessin ouverte ; mise en page du cache et pinceau vivants.
        unsafe {
            if paragraph.format.clip {
                target.PushAxisAlignedClip(&rect_f(rect), D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
            }
            target.DrawTextLayout(
                Vector2 {
                    X: rect.left,
                    Y: rect.top,
                },
                layout,
                brush,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
            );
            if paragraph.format.clip {
                target.PopAxisAlignedClip();
            }
        }
        Ok(())
    }
}
