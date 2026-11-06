//! Ligne de saisie : carets et clusters d'une ligne sans retour, pour l'édition de texte,
//! et coupure d'un texte éditable multiligne par DirectWrite.
use super::*;

impl Cache {
    pub fn line(
        &mut self,
        factory: &IDWriteFactory,
        native: &IDWriteTextFormat,
        value: &str,
        mut format: TextFormat,
    ) -> crate::Result<crate::ui::text_input::TextLine> {
        use crate::ui::text_input::{TextCaret, TextCluster, TextLine};
        format.align = TextAlign::Start;
        let size = format.size;
        let source = if value.is_empty() { "M" } else { value };
        let utf16 = source.encode_utf16().collect::<Vec<_>>();
        let line = &self
            .ensure(
                factory,
                native,
                Key::new(Arc::from(source), format, 1_000_000., false),
            )?
            .layout;
        let mut native = vec![DWRITE_CLUSTER_METRICS::default(); utf16.len().max(1)];
        let mut count = 0;
        // SAFETY: tampon de sortie et compteur valides pour la durée de l'appel.
        unsafe { line.GetClusterMetrics(Some(&mut native), &mut count) }
            .check("GetClusterMetrics")?;
        let mut map = vec![0usize; utf16.len() + 1];
        let mut unit = 0;
        for (byte, c) in source.char_indices() {
            for _ in 0..c.len_utf16() {
                map[unit] = byte;
                unit += 1;
            }
            map[unit] = byte + c.len_utf8();
        }
        let mut result = TextLine {
            carets: Vec::new(),
            clusters: Vec::new(),
            width: 0.,
            height: size * 1.4,
        };
        let mut position = 0u32;
        for cluster in native.into_iter().take(count as usize) {
            let mut x = 0.;
            let mut y = 0.;
            let mut metrics = DWRITE_HIT_TEST_METRICS::default();
            // SAFETY: sorties valides pour la durée de l'appel.
            unsafe { line.HitTestTextPosition(position, false, &mut x, &mut y, &mut metrics) }
                .check("HitTestTextPosition")?;
            result.height = metrics.height;
            result.width = result.width.max(metrics.left + metrics.width);
            if !value.is_empty() {
                let start = map[position as usize];
                let end = map[(position + cluster.length as u32) as usize];
                result.carets.push(TextCaret { byte: start, x });
                result.clusters.push(TextCluster {
                    start,
                    end,
                    rect: RectF::new(metrics.left, metrics.top, metrics.width, metrics.height),
                });
                // SAFETY: idem, pour le bord de fin du cluster.
                unsafe { line.HitTestTextPosition(position, true, &mut x, &mut y, &mut metrics) }
                    .check("HitTestTextPosition trailing")?;
                result.carets.push(TextCaret { byte: end, x });
            }
            position += cluster.length as u32;
        }
        if value.is_empty() {
            result.width = 0.;
            result.carets.push(TextCaret { byte: 0, x: 0. });
        }
        Ok(result)
    }
    /// Intervalles d'octets des lignes visuelles de `value` coupé à `width`, d'après
    /// `GetLineMetrics` de la mise en page du dessin : retours explicites exclus,
    /// espaces de coupure gardés en fin de ligne.
    pub fn wrap_ranges(
        &mut self,
        factory: &IDWriteFactory,
        native: &IDWriteTextFormat,
        value: &str,
        format: TextFormat,
        width: f32,
    ) -> crate::Result<Vec<std::ops::Range<usize>>> {
        if value.is_empty() {
            return Ok(std::iter::once(0..0).collect());
        }
        let format = crate::ui::text_input::line_format(format);
        let layout = &self
            .ensure(
                factory,
                native,
                Key::new(Arc::from(value), format, width, true),
            )?
            .layout;
        let mut lines = vec![DWRITE_LINE_METRICS::default(); 16];
        let mut count = 0;
        // SAFETY: tampon de sortie et compteur valides pour la durée de l'appel.
        let mut result = unsafe { layout.GetLineMetrics(Some(&mut lines), &mut count) };
        if result.is_err() && count as usize > lines.len() {
            lines.resize(count as usize, DWRITE_LINE_METRICS::default());
            // SAFETY: idem, tampon agrandi au nombre de lignes demandé.
            result = unsafe { layout.GetLineMetrics(Some(&mut lines), &mut count) };
        }
        result.check("GetLineMetrics (wrap)")?;
        // Octet de chaque unité UTF-16 (et de la fin du texte).
        let mut map = Vec::with_capacity(value.len() + 1);
        for (byte, c) in value.char_indices() {
            for _ in 0..c.len_utf16() {
                map.push(byte);
            }
        }
        map.push(value.len());
        let units = map.len() - 1;
        let mut ranges = Vec::with_capacity(count as usize);
        let mut unit = 0usize;
        for line in lines.iter().take(count as usize) {
            let end = (unit + line.length as usize).min(units);
            let visible = end.saturating_sub(line.newlineLength as usize).max(unit);
            ranges.push(map[unit]..map[visible]);
            unit = end;
        }
        if ranges.is_empty() {
            ranges.push(0..value.len());
        }
        Ok(ranges)
    }
}
