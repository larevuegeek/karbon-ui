//! Ligne de saisie : carets et clusters graphèmes d'une ligne sans retour (positions de
//! curseur de Pango), et coupure d'un texte éditable multiligne par Pango.
use super::{Cache, Fonts, plain};
use crate::ui::{
    scene::RectF,
    styling::{TextAlign, text::TextFormat},
    text_input::{TextCaret, TextCluster, TextLine},
};
use std::sync::Arc;

impl Cache {
    pub fn line(
        &mut self,
        fonts: &mut Fonts,
        value: &str,
        mut format: TextFormat,
    ) -> crate::Result<TextLine> {
        format.align = TextAlign::Start;
        // Un champ vide garde la hauteur d'une ligne de texte.
        let source: Arc<str> = Arc::from(if value.is_empty() { "M" } else { value });
        let layout = self.layout(fonts, &plain(&source, format), 1_000_000., false)?;
        let line = &layout.lines[0];
        let mut result = TextLine {
            carets: Vec::new(),
            clusters: Vec::new(),
            width: 0.,
            height: line.height,
        };
        if value.is_empty() {
            result.carets.push(TextCaret { byte: 0, x: 0. });
            return Ok(result);
        }
        // Frontières des graphèmes : un attribut par caractère, plus la fin du texte.
        let attrs = layout.main().log_attrs();
        let mut stops = value
            .char_indices()
            .enumerate()
            .filter(|(i, _)| *i == 0 || attrs.get(*i).is_none_or(|a| a.is_cursor_position()))
            .map(|(_, (byte, _))| byte)
            .collect::<Vec<_>>();
        stops.push(value.len());
        for pair in stops.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let (x0, x1) = (line.offset(a), line.offset(b));
            result.carets.push(TextCaret { byte: a, x: x0 });
            result.clusters.push(TextCluster {
                start: a,
                end: b,
                rect: RectF::new(x0.min(x1), 0., (x1 - x0).abs(), line.height),
            });
            result.carets.push(TextCaret { byte: b, x: x1 });
            result.width = result.width.max(x0.max(x1));
        }
        Ok(result)
    }
    /// Intervalles d'octets des lignes visuelles de `value` coupé à `width` : ceux de
    /// la mise en page Pango du dessin (retours explicites exclus, espaces de coupure
    /// gardés en fin de ligne).
    pub fn wrap_ranges(
        &mut self,
        fonts: &mut Fonts,
        value: &str,
        format: TextFormat,
        width: f32,
    ) -> crate::Result<Vec<std::ops::Range<usize>>> {
        let format = crate::ui::text_input::line_format(format);
        let source: Arc<str> = Arc::from(value);
        let layout = self.layout(fonts, &plain(&source, format), width.max(1.), true)?;
        Ok(layout
            .lines
            .iter()
            .map(|l| l.start..l.end.max(l.start))
            .collect())
    }
}
