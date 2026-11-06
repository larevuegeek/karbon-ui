//! TextPattern des champs de saisie (une ou plusieurs lignes) : plages COM et géométrie
//! issue du même layout que le rendu.
//! Les plages elles-mêmes (`ITextRangeProvider`) sont dans `range`.
mod range;
#[cfg(test)]
mod tests;
use super::{provider::boundary, *};
use range::new_range;
use std::ops::Range;
use windows::{
    Win32::{
        System::Com::SAFEARRAY,
        UI::Accessibility::{
            ITextRangeProvider, SupportedTextSelection, SupportedTextSelection_Single, UiaPoint,
        },
    },
    core::IUnknown,
};

pub(super) struct TextData {
    pub value: Arc<str>,
    pub selection: Range<usize>,
    carets: Vec<(usize, f64)>,
    characters: Vec<usize>,
    clusters: Vec<(Range<usize>, UiaRect)>,
    /// Lignes visuelles : octets affichés et rectangle.
    lines: Vec<(Range<usize>, UiaRect)>,
    clip: UiaRect,
}
impl TextData {
    pub(super) fn physical(
        t: crate::ui::text_input::AccessibleText,
        origin: Point,
        scale: f32,
    ) -> Self {
        let mut characters = vec![0, t.value.len()];
        characters.extend(t.carets.iter().map(|c| c.byte));
        characters.sort_unstable();
        characters.dedup();
        Self {
            value: t.value,
            selection: t.selection,
            characters,
            carets: t
                .carets
                .into_iter()
                .map(|c| (c.byte, origin.x as f64 + c.x as f64 * scale as f64))
                .collect(),
            clusters: t
                .clusters
                .into_iter()
                .map(|c| (c.start..c.end, physical(c.rect, origin, scale)))
                .collect(),
            lines: t
                .lines
                .into_iter()
                .map(|l| (l.range, physical(l.rect, origin, scale)))
                .collect(),
            clip: physical(t.clip, origin, scale),
        }
    }
    fn boundaries(&self, unit: i32) -> Result<Vec<usize>> {
        let mut points = vec![0, self.value.len()];
        match unit {
            0 => return Ok(self.characters.clone()),
            2 => {
                let mut whitespace = true;
                for (byte, c) in self.value.char_indices() {
                    if whitespace && !c.is_whitespace() {
                        points.push(byte);
                    }
                    whitespace = c.is_whitespace();
                }
                // Même géométrie de caractères que le champ, y compris les ligatures.
                points.retain(|p| self.characters.binary_search(p).is_ok());
            }
            3 => points.extend(self.lines.iter().map(|(r, _)| r.start)),
            4 => points.extend(self.value.match_indices('\n').map(|(i, _)| i + 1)),
            1 | 5 | 6 => {} // Format uniforme, une seule page, un document.
            _ => return Err(fail(INVALID_ARGUMENT)),
        }
        points.sort_unstable();
        points.dedup();
        Ok(points)
    }
    fn snap(&self, byte: usize) -> usize {
        let index = self
            .characters
            .partition_point(|&b| b <= byte)
            .saturating_sub(1);
        self.characters[index]
    }
    /// Arrêt de caret le plus proche du point, sur la ligne la plus proche de `y`.
    fn hit(&self, x: f64, y: f64) -> usize {
        let distance = |r: &UiaRect| {
            if y < r.top {
                r.top - y
            } else {
                (y - r.top - r.height).max(0.)
            }
        };
        let line = self
            .lines
            .iter()
            .min_by(|a, b| distance(&a.1).total_cmp(&distance(&b.1)))
            .map(|(r, _)| r.clone());
        self.carets
            .iter()
            .filter(|c| line.as_ref().is_none_or(|r| r.start <= c.0 && c.0 <= r.end))
            .min_by(|a, b| (a.1 - x).abs().total_cmp(&(b.1 - x).abs()))
            .map_or(0, |c| c.0)
    }
    fn visible(&self, offscreen: bool) -> Vec<Range<usize>> {
        let mut spans: Vec<_> = if offscreen {
            Vec::new()
        } else {
            self.clusters
                .iter()
                .filter(|(_, r)| intersection(*r, self.clip).is_some())
                .map(|(r, _)| r.clone())
                .collect()
        };
        spans.sort_by_key(|r| r.start);
        let mut result: Vec<Range<usize>> = Vec::new();
        for span in spans {
            if let Some(last) = result.last_mut()
                && span.start <= last.end
            {
                last.end = last.end.max(span.end);
            } else {
                result.push(span);
            }
        }
        if result.is_empty() {
            result.push(0..0);
        }
        result
    }
    fn rectangles(&self, range: Range<usize>) -> Vec<f64> {
        if range.is_empty() {
            return Vec::new();
        }
        let mut rects: Vec<_> = self
            .clusters
            .iter()
            .filter(|(r, _)| r.start < range.end && r.end > range.start)
            .filter_map(|(_, r)| intersection(*r, self.clip))
            .collect();
        rects.sort_by(|a, b| a.top.total_cmp(&b.top).then(a.left.total_cmp(&b.left)));
        let mut merged: Vec<UiaRect> = Vec::new();
        for r in rects {
            if let Some(last) = merged.last_mut()
                && (last.top - r.top).abs() < 0.01
                && (last.height - r.height).abs() < 0.01
                && r.left <= last.left + last.width + 0.01
            {
                last.width = (last.left + last.width).max(r.left + r.width) - last.left;
            } else {
                merged.push(r);
            }
        }
        merged
            .into_iter()
            .flat_map(|r| [r.left, r.top, r.width, r.height])
            .collect()
    }
}
fn intersection(a: UiaRect, b: UiaRect) -> Option<UiaRect> {
    let left = a.left.max(b.left);
    let top = a.top.max(b.top);
    let width = (a.left + a.width).min(b.left + b.width) - left;
    let height = (a.top + a.height).min(b.top + b.height) - top;
    (width > 0. && height > 0.).then_some(UiaRect {
        left,
        top,
        width,
        height,
    })
}
fn data(bridge: &Bridge, key: NodeId) -> Result<(Arc<Node>, Arc<TextData>)> {
    let n = bridge.node(key)?;
    let text = n.text.clone().ok_or_else(|| fail(UNSUPPORTED))?;
    Ok((n, text))
}

fn range_array(
    bridge: &Arc<Bridge>,
    key: NodeId,
    text: &TextData,
    spans: Vec<Range<usize>>,
) -> Result<*mut SAFEARRAY> {
    let ranges: Vec<IUnknown> = spans
        .into_iter()
        .map(|span| new_range(bridge, key, Arc::clone(&text.value), span).into())
        .collect();
    unknowns(&ranges)
}
/// ITextProvider : appelé par les classes de `provider` qui exposent le pattern texte.
impl provider::Core {
    pub(super) fn text_selection(&self) -> Result<*mut SAFEARRAY> {
        let (_, text) = data(&self.bridge, self.key)?;
        range_array(&self.bridge, self.key, &text, vec![text.selection.clone()])
    }
    pub(super) fn visible_ranges(&self) -> Result<*mut SAFEARRAY> {
        let (node, text) = data(&self.bridge, self.key)?;
        range_array(&self.bridge, self.key, &text, text.visible(node.offscreen))
    }
    pub(super) fn range_from_child(&self) -> Result<ITextRangeProvider> {
        data(&self.bridge, self.key)?;
        Err(fail(INVALID_ARGUMENT)) // Aucun objet incorporé dans un champ de saisie.
    }
    pub(super) fn range_from_point(&self, point: UiaPoint) -> Result<ITextRangeProvider> {
        if !point.x.is_finite() || !point.y.is_finite() {
            return Err(fail(INVALID_ARGUMENT));
        }
        let (_, text) = data(&self.bridge, self.key)?;
        let byte = text.hit(point.x, point.y);
        Ok(new_range(
            &self.bridge,
            self.key,
            Arc::clone(&text.value),
            byte..byte,
        ))
    }
    pub(super) fn document_range(&self) -> Result<ITextRangeProvider> {
        let (_, text) = data(&self.bridge, self.key)?;
        Ok(new_range(
            &self.bridge,
            self.key,
            Arc::clone(&text.value),
            0..text.value.len(),
        ))
    }
    pub(super) fn supported_text_selection(&self) -> Result<SupportedTextSelection> {
        data(&self.bridge, self.key)?;
        Ok(SupportedTextSelection_Single)
    }
}
