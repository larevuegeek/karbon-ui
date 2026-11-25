//! Bornes de ce que peint un texte, pour le redessin partiel (KUI-055) : les mises en
//! page DirectWrite de l'image, préparées par [`Cache::prepare`].
use super::*;
use crate::platform::shared::damage::TextInk;

/// Les textes de l'image en cours, vus par l'inventaire des zones modifiées.
pub(crate) struct Ink<'a>(pub(crate) &'a Cache);

impl TextInk for Ink<'_> {
    fn ink(&mut self, command: &DrawCommand) -> crate::Result<Option<RectF>> {
        Ok(match command {
            // `DrawText` avec `D2D1_DRAW_TEXT_OPTIONS_CLIP` : rien hors de la boîte.
            DrawCommand::Text { rect, .. } | DrawCommand::FontText { rect, .. } => Some(*rect),
            DrawCommand::FormattedText {
                rect,
                text,
                format,
                wrap,
                ..
            } => {
                let key = Key::new(Arc::clone(text), *format, rect.right - rect.left, *wrap);
                let layout = self.0.entries.get(&key).map(|entry| &entry.layout);
                Some(bounds(layout, *rect, format.size, format.clip))
            }
            DrawCommand::RichText {
                rect,
                paragraph,
                wrap,
                ..
            } => {
                let key = RichKey::new(paragraph, rect.right - rect.left, *wrap);
                let layout = self.0.rich.get(&key).map(RichEntry::layout);
                let format = paragraph.format;
                Some(bounds(layout, *rect, format.size, format.clip))
            }
            _ => None,
        })
    }
}

/// La boîte du texte, ses lignes si elles en débordent (texte non découpé, centré,
/// italique), et une marge d'une demi-taille de police pour les jambages et les
/// décorations. Sans mise en page connue : toute la fenêtre (rien n'est oublié).
fn bounds(layout: Option<&IDWriteTextLayout>, rect: RectF, size: f32, clip: bool) -> RectF {
    let mut metrics = DWRITE_TEXT_METRICS::default();
    // SAFETY : mise en page du cache, vivante ; sortie locale écrite pendant l'appel.
    let Some(()) = layout.and_then(|layout| unsafe { layout.GetMetrics(&mut metrics) }.ok()) else {
        return RectF {
            left: -1e7,
            top: -1e7,
            right: 1e7,
            bottom: 1e7,
        };
    };
    let width = metrics.width.max(metrics.widthIncludingTrailingWhitespace);
    let reach = size * 0.5;
    let ink = RectF {
        left: rect.left.min(rect.left + metrics.left) - reach,
        top: rect.top.min(rect.top + metrics.top) - reach,
        right: rect.right.max(rect.left + metrics.left + width) + reach,
        bottom: rect.bottom.max(rect.top + metrics.top + metrics.height) + reach,
    };
    if !clip {
        return ink;
    }
    RectF {
        left: ink.left.max(rect.left - 1.),
        top: ink.top.max(rect.top - 1.),
        right: ink.right.min(rect.right + 1.),
        bottom: ink.bottom.min(rect.bottom + 1.),
    }
}
