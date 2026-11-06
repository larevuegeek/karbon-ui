//! Attributs Pango d'un paragraphe : une police par portion, l'espacement des lettres,
//! les chiffres tabulaires, et une séparation des runs aux bornes des portions colorées
//! (la couleur est choisie au dessin, run par run).
use super::Fonts;
use crate::{
    platform::linux::ffi::pango::{AttrList, FontDescription},
    ui::styling::text::{RichParagraph, SpanStyle},
};
use std::ops::Range;

/// Portion de style homogène, en octets du paragraphe.
pub(in crate::platform::linux) struct Segment {
    pub start: usize,
    pub end: usize,
    pub style: SpanStyle,
}
/// Portions du paragraphe (les intervalles hors `spans` prennent le style de base).
pub(super) fn segments(paragraph: &RichParagraph) -> Vec<Segment> {
    let segment = |range: Range<usize>, style| Segment {
        start: range.start,
        end: range.end,
        style,
    };
    let mut result = Vec::new();
    let mut at = 0;
    for span in &paragraph.spans {
        if span.range.start > at {
            result.push(segment(at..span.range.start, SpanStyle::new()));
        }
        if span.range.end > span.range.start {
            result.push(segment(span.range.clone(), span.style));
        }
        at = span.range.end;
    }
    if at < paragraph.text.len() || result.is_empty() {
        result.push(segment(at..paragraph.text.len(), SpanStyle::new()));
    }
    result
}
/// Police d'une portion : famille, graisse, taille et italique de base surchargés.
pub(super) fn font(
    fonts: &mut Fonts,
    paragraph: &RichParagraph,
    style: SpanStyle,
) -> crate::Result<FontDescription> {
    let f = paragraph.format;
    fonts.font(
        style.family.unwrap_or(f.font.family),
        style.weight.unwrap_or(f.font.weight),
        f.italic || style.italic,
        style.size.unwrap_or(f.size),
    )
}
/// Attributs des octets `range` de `paragraph`, décalés au début de `range` (texte d'une
/// mise en page qui ne contient que cette partie).
pub(super) fn list(
    fonts: &mut Fonts,
    paragraph: &RichParagraph,
    segments: &[Segment],
    range: Range<usize>,
) -> crate::Result<AttrList> {
    let list = AttrList::new()?;
    let length = range.end - range.start;
    for segment in segments {
        let (start, end) = (segment.start.max(range.start), segment.end.min(range.end));
        if end <= start {
            continue;
        }
        let local = start - range.start..end - range.start;
        list.font(&font(fonts, paragraph, segment.style)?, local.clone())?;
        if segment.style.color.is_some() {
            list.split(local)?;
        }
    }
    let format = paragraph.format;
    if format.letter_spacing != 0. {
        list.letter_spacing(format.letter_spacing, 0..length)?;
    }
    if format.tabular_numbers {
        list.features(c"tnum", 0..length)?;
    }
    Ok(list)
}
