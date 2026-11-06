use super::*;
/// Valid caret position in a measured line of text.
#[derive(Clone)]
pub struct TextCaret {
    /// UTF-8 byte offset of the position.
    pub byte: usize,
    /// Horizontal position in logical units (relative to the line start in a
    /// [`TextLine`], in window coordinates in an [`AccessibleText`](super::AccessibleText)).
    pub x: f32,
}
/// Grapheme cluster of a measured line: the smallest unit of selection.
#[derive(Clone)]
pub struct TextCluster {
    /// Start byte offset (inclusive).
    pub start: usize,
    /// End byte offset (exclusive).
    pub end: usize,
    /// Cluster rectangle (line-relative in a [`TextLine`], window coordinates
    /// in an [`AccessibleText`](super::AccessibleText)).
    pub rect: RectF,
}
/// Single line of text measured by the backend: caret stops, clusters and size,
/// in logical units relative to the line origin.
pub struct TextLine {
    /// Caret stops in ascending byte order, starting at byte 0.
    pub carets: Vec<TextCaret>,
    /// Grapheme clusters.
    pub clusters: Vec<TextCluster>,
    /// Advance width of the line.
    pub width: f32,
    /// Line height.
    pub height: f32,
}
impl TextLine {
    pub(super) fn x(&self, byte: usize) -> f32 {
        self.carets
            .iter()
            .find(|c| c.byte == byte)
            .map_or(self.width, |c| c.x)
    }

    pub(super) fn hit(&self, x: f32) -> usize {
        self.carets
            .iter()
            .min_by(|a, b| (a.x - x).abs().total_cmp(&(b.x - x).abs()))
            .map_or(0, |c| c.byte)
    }

    pub(super) fn previous(&self, byte: usize) -> usize {
        self.carets
            .iter()
            .map(|c| c.byte)
            .filter(|&b| b < byte)
            .max()
            .unwrap_or(0)
    }

    pub(super) fn next(&self, byte: usize) -> usize {
        self.carets
            .iter()
            .map(|c| c.byte)
            .filter(|&b| b > byte)
            .min()
            .unwrap_or(byte)
    }

    pub(super) fn snap(&self, byte: usize) -> usize {
        self.carets
            .iter()
            .map(|c| c.byte)
            .filter(|&b| b <= byte)
            .max()
            .unwrap_or(0)
    }

    pub(super) fn selection(&self, range: Range<usize>) -> Vec<RectF> {
        let mut rects: Vec<_> = self
            .clusters
            .iter()
            .filter(|c| c.start < range.end && c.end > range.start)
            .map(|c| c.rect)
            .collect();
        rects.sort_by(|a, b| a.left.total_cmp(&b.left));
        let mut result: Vec<RectF> = Vec::new();
        for rect in rects {
            if let Some(last) = result.last_mut()
                && (last.top - rect.top).abs() < 0.5
                && rect.left <= last.right + 0.5
            {
                last.right = last.right.max(rect.right);
            } else {
                result.push(rect);
            }
        }
        result
    }
}
