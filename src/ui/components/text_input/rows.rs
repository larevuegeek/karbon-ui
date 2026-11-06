//! Lignes visuelles d'un champ multiligne : position verticale de chaque ligne, ligne
//! d'un octet (avec l'affinité aux coupures automatiques) ou d'une ordonnée, et ligne
//! fusionnée pour la navigation par cluster commune au champ monoligne.
use super::*;
use crate::ui::styling::TextAlign;

/// Ligne visuelle placée : octets affichés, haut depuis le début du texte, géométrie.
pub(super) struct Row {
    pub range: Range<usize>,
    pub top: f32,
    pub line: TextLine,
    /// Largeur sans les espaces de fin (alignement).
    content: f32,
}
impl Row {
    pub fn bottom(&self) -> f32 {
        self.top + self.line.height
    }
}
pub(super) struct Rows {
    pub lines: Vec<Row>,
    /// Hauteur totale du texte.
    pub height: f32,
    /// Largeur de coupure.
    pub width: f32,
}
impl Rows {
    /// `text` : texte réel (les octets des lignes y renvoient, même masqué).
    pub fn new(lines: Vec<WrappedLine>, width: f32, text: &str) -> Self {
        let mut top = 0.;
        let mut rows = Vec::with_capacity(lines.len().max(1));
        for WrappedLine { range, line } in lines {
            let content = line
                .clusters
                .iter()
                .filter(|c| {
                    text.get(c.start..c.end)
                        .is_none_or(|s| !s.chars().all(char::is_whitespace))
                })
                .map(|c| c.rect.right)
                .fold(0f32, f32::max);
            let height = line.height;
            rows.push(Row {
                range,
                top,
                line,
                content,
            });
            top += height;
        }
        if rows.is_empty() {
            rows.push(Row {
                range: 0..0,
                top: 0.,
                line: TextLine {
                    carets: vec![TextCaret { byte: 0, x: 0. }],
                    clusters: Vec::new(),
                    width: 0.,
                    height: 0.,
                },
                content: 0.,
            });
        }
        Self {
            lines: rows,
            height: top,
            width,
        }
    }

    /// Ligne fusionnée : tous les arrêts de caret et clusters, pour les déplacements
    /// par cluster et l'ajustement des positions (abscisses sans signification).
    pub fn flatten(&self) -> TextLine {
        let mut carets: Vec<TextCaret> = self
            .lines
            .iter()
            .flat_map(|r| r.line.carets.iter().cloned())
            .collect();
        carets.sort_by_key(|c| c.byte);
        carets.dedup_by_key(|c| c.byte);
        TextLine {
            carets,
            clusters: self
                .lines
                .iter()
                .flat_map(|r| r.line.clusters.iter().cloned())
                .collect(),
            width: self.width,
            height: self.height,
        }
    }

    /// La ligne `index` se termine par une coupure automatique (la suivante commence
    /// au même octet).
    pub fn soft(&self, index: usize) -> bool {
        self.lines
            .get(index + 1)
            .is_some_and(|next| next.range.start == self.lines[index].range.end)
    }

    /// Ligne qui affiche le caret à `byte` : à une coupure automatique, la ligne
    /// suivante, ou la précédente avec `upstream`.
    pub fn index(&self, byte: usize, upstream: bool) -> usize {
        let index = self
            .lines
            .partition_point(|r| r.range.start <= byte)
            .saturating_sub(1);
        if upstream && index > 0 && self.soft(index - 1) && self.lines[index].range.start == byte {
            index - 1
        } else {
            index
        }
    }

    /// Ligne à l'ordonnée `y` (depuis le haut du texte), bornée à la première et à la
    /// dernière.
    pub fn at_y(&self, y: f32) -> usize {
        self.lines
            .partition_point(|r| r.bottom() <= y)
            .min(self.lines.len() - 1)
    }

    /// Décalage horizontal d'une ligne selon l'alignement.
    pub fn offset(&self, index: usize, align: TextAlign) -> f32 {
        let extra = (self.width - self.lines[index].content).max(0.);
        match align {
            TextAlign::End => extra,
            TextAlign::Center => extra / 2.,
            _ => 0.,
        }
    }

    /// Octet le plus proche de l'abscisse `x` (depuis le bord du texte) sur la ligne
    /// `index`, et son affinité.
    pub fn hit(&self, index: usize, x: f32, align: TextAlign) -> (usize, bool) {
        let row = &self.lines[index];
        let byte = row.line.hit(x - self.offset(index, align));
        (byte, byte == row.range.end && self.soft(index))
    }
}
