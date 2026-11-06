//! Coupure d'un texte éditable en lignes visuelles : type public [`WrappedLine`],
//! implémentation par défaut de `TextMeasurer::layout_wrapped_text` et assemblage des
//! lignes à partir des coupures natives d'un backend.
//!
//! Chaque ligne est ensuite mise en page seule (`layout_formatted_line`) : le champ
//! dessine chaque ligne avec la même mise en page, donc carets, sélection et glyphes
//! coïncident quel que soit le backend.
use super::*;
use crate::ui::styling::text::TextFormat;

/// One visual line of an editable text, as returned by
/// [`TextMeasurer::layout_wrapped_text`](crate::ui::element::TextMeasurer::layout_wrapped_text).
pub struct WrappedLine {
    /// UTF-8 bytes of the text shown on the line, without the line feed that ends it.
    /// Spaces at an automatic break stay at the end of the line, so the next line
    /// starts where this one ends; after a line feed it starts one byte later.
    pub range: Range<usize>,
    /// Caret stops and clusters of the line. Byte offsets are relative to the whole
    /// text; positions are relative to the line's own top-left corner.
    pub line: TextLine,
}

/// Format d'une ligne mise en page seule : début de ligne, ni coupure ni ellipse.
pub(crate) fn line_format(mut format: TextFormat) -> TextFormat {
    format.align = crate::ui::styling::TextAlign::Start;
    format.white_space = crate::ui::styling::WhiteSpace::Normal;
    format.transform = Default::default();
    format.ellipsis = false;
    format.ellipsis_start = false;
    format.max_lines = 0;
    format
}

/// Implémentation par défaut : chaque paragraphe (séparé par `\n`) est mis en page sur
/// une ligne, puis coupé glouton après les espaces (mot trop long coupé entre clusters).
pub(crate) fn wrap_text<T: TextMeasurer + ?Sized>(
    text: &mut T,
    value: &str,
    format: TextFormat,
    width: f32,
) -> crate::Result<Vec<WrappedLine>> {
    let single = line_format(format);
    let mut ranges = Vec::new();
    let mut start = 0;
    for paragraph in value.split('\n') {
        let line = text.layout_formatted_line(paragraph, single)?;
        for range in greedy(paragraph, &line, width) {
            ranges.push(start + range.start..start + range.end);
        }
        start += paragraph.len() + 1;
    }
    lines_from_ranges(text, value, format, ranges)
}

/// Coupure gloutonne d'un paragraphe d'après la géométrie de ses clusters ; les espaces
/// en fin de ligne débordent sans provoquer de coupure.
fn greedy(paragraph: &str, line: &TextLine, width: f32) -> Vec<Range<usize>> {
    let mut clusters: Vec<_> = line
        .clusters
        .iter()
        .filter(|c| c.start < c.end && c.end <= paragraph.len())
        .map(|c| {
            let blank = paragraph
                .get(c.start..c.end)
                .is_some_and(|s| s.chars().all(char::is_whitespace));
            (c.start, c.rect.left, c.rect.right, blank)
        })
        .collect();
    clusters.sort_by_key(|c| c.0);
    clusters.dedup_by_key(|c| c.0);
    if clusters.is_empty() {
        return std::iter::once(0..paragraph.len()).collect();
    }
    let mut ranges = Vec::new();
    let mut first = 0;
    loop {
        let start = if first == 0 { 0 } else { clusters[first].0 };
        let left = clusters[first].1;
        let mut right = left;
        let mut word = None;
        let mut cut = None;
        for index in first..clusters.len() {
            let (_, l, r, blank) = clusters[index];
            if index > first && !blank && clusters[index - 1].3 {
                word = Some(index);
            }
            if blank {
                continue;
            }
            if index > first && r.max(right) - l.min(left) > width + 0.01 {
                cut = Some(word.unwrap_or(index));
                break;
            }
            right = right.max(r);
        }
        match cut {
            Some(index) => {
                ranges.push(start..clusters[index].0);
                first = index;
            }
            None => {
                ranges.push(start..paragraph.len());
                return ranges;
            }
        }
    }
}

/// Lignes d'un texte coupé aux intervalles `ranges` (un par ligne visuelle, dans
/// l'ordre) : chaque ligne est mise en page seule, octets ramenés au texte entier.
/// Les espaces écartés par une coupure native sont rendus à la ligne qui précède.
pub(crate) fn lines_from_ranges<T: TextMeasurer + ?Sized>(
    text: &mut T,
    value: &str,
    format: TextFormat,
    ranges: Vec<Range<usize>>,
) -> crate::Result<Vec<WrappedLine>> {
    let format = line_format(format);
    let ranges = close_gaps(value, ranges);
    let mut lines = Vec::with_capacity(ranges.len());
    for range in ranges {
        let mut line = text.layout_formatted_line(&value[range.clone()], format)?;
        for caret in &mut line.carets {
            caret.byte += range.start;
        }
        for cluster in &mut line.clusters {
            cluster.start += range.start;
            cluster.end += range.start;
        }
        lines.push(WrappedLine { range, line });
    }
    Ok(lines)
}

/// Intervalles valides, ordonnés et contigus : un écart sans `\n` entre deux lignes
/// (espaces de coupure) rejoint la première ; le reste du texte rejoint la dernière.
fn close_gaps(value: &str, ranges: Vec<Range<usize>>) -> Vec<Range<usize>> {
    let snap = |mut byte: usize| {
        byte = byte.min(value.len());
        while !value.is_char_boundary(byte) {
            byte -= 1;
        }
        byte
    };
    let mut result: Vec<Range<usize>> = Vec::with_capacity(ranges.len().max(1));
    for range in ranges {
        let start = snap(range.start).max(result.last().map_or(0, |r| r.end));
        let end = snap(range.end).max(start);
        if let Some(last) = result.last_mut() {
            let gap = &value[last.end..start];
            match gap.find('\n') {
                Some(offset) => last.end += offset,
                None => last.end = start,
            }
        }
        result.push(start..end);
    }
    if result.is_empty() {
        result.push(0..0);
    }
    let last = result.last_mut().unwrap();
    let rest = &value[last.end..];
    if !rest.contains('\n') {
        last.end = value.len();
    }
    result
}
