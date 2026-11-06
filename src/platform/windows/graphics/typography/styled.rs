//! Texte formaté (`FormattedText`) : mise en page avec alignement, interlignage et
//! décorations, puis dessin avec surlignes et soulignements propres.
use super::*;

impl Cache {
    pub(super) fn ensure(
        &mut self,
        factory: &IDWriteFactory,
        format_native: &IDWriteTextFormat,
        key: Key,
    ) -> crate::Result<&Entry> {
        if !self.entries.contains_key(&key)
            && key.ellipsis_start
            && let Some(at) = start_cut(factory, format_native, &key, &key.value)?
        {
            let inner = Key {
                value: format!("\u{2026}{}", &key.value[at..]).into(),
                ellipsis_start: false,
                ..key.clone()
            };
            let entry = self.ensure(factory, format_native, inner)?.clone();
            self.entries.insert(key.clone(), entry);
        }
        if !self.entries.contains_key(&key) {
            let value = key.value.encode_utf16().collect::<Vec<_>>();
            // SAFETY: texte UTF-16 et format vivants pendant l'appel.
            let layout = unsafe {
                factory.CreateTextLayout(
                    &value,
                    format_native,
                    f32::from_bits(key.width),
                    1_000_000.,
                )
            }
            .check("CreateTextLayout styled")?;
            let align = key.alignment();
            // SAFETY: mise en page fraîchement créée, possédée localement.
            unsafe { layout.SetTextAlignment(align) }.check("SetTextAlignment")?;
            // SAFETY: mise en page fraîchement créée, possédée localement.
            unsafe { layout.SetWordWrapping(wrapping(key.wrap)) }
                .check("SetWordWrapping styled")?;
            if key.line != 0 {
                let line = f32::from_bits(key.line);
                let baseline = ((line - f32::from_bits(key.size)) * 0.5
                    + f32::from_bits(key.size) * 0.8)
                    .clamp(0., line);
                // SAFETY: mise en page fraîchement créée, possédée localement.
                unsafe {
                    layout.SetLineSpacing(DWRITE_LINE_SPACING_METHOD_UNIFORM, line, baseline)
                }
                .check("SetLineSpacing")?;
            }
            let range = DWRITE_TEXT_RANGE {
                startPosition: 0,
                length: value.len() as u32,
            };
            if key.decoration.underline && !key.custom_underline {
                // SAFETY: mise en page fraîchement créée, possédée localement.
                unsafe { layout.SetUnderline(true, range) }.check("Set text decoration")?;
            }
            if key.decoration.line_through {
                // SAFETY: mise en page fraîchement créée, possédée localement.
                unsafe { layout.SetStrikethrough(true, range) }.check("Set text decoration")?;
            }
            let clamped = features(factory, &layout, &key, value.len() as u32)?;
            let mut metrics = DWRITE_TEXT_METRICS::default();
            // SAFETY: sortie valide pour la durée de l'appel.
            unsafe { layout.GetMetrics(&mut metrics) }.check("GetMetrics styled")?;
            let mut overlines = Vec::new();
            if key.decoration.overline && !value.is_empty() {
                let mut ranges = vec![DWRITE_HIT_TEST_METRICS::default(); value.len().max(1)];
                let mut count = 0;
                // SAFETY: tampon de sortie et compteur valides pour la durée de l'appel.
                unsafe {
                    layout.HitTestTextRange(
                        0,
                        value.len() as u32,
                        0.,
                        0.,
                        Some(&mut ranges),
                        &mut count,
                    )
                }
                .check("HitTestTextRange overline")?;
                overlines.extend(
                    ranges
                        .into_iter()
                        .take(count as usize)
                        .map(|h| RectF::new(h.left, h.top, h.width, h.height)),
                );
            }
            // Soulignement propre (couleur, décalage, épaisseur) : boîtes des morceaux de
            // ligne et ligne de base de leur ligne, dessinés par `draw`.
            let mut underlines = Vec::new();
            if key.custom_underline && !value.is_empty() {
                let mut lines = vec![DWRITE_LINE_METRICS::default(); 64];
                let mut line_count = 0;
                // SAFETY: tampon de sortie et compteur valides pour la durée de l'appel.
                let mut result =
                    unsafe { layout.GetLineMetrics(Some(&mut lines), &mut line_count) };
                if result.is_err() && line_count as usize > lines.len() {
                    lines.resize(line_count as usize, DWRITE_LINE_METRICS::default());
                    // SAFETY: idem, tampon agrandi au nombre de lignes demandé.
                    result = unsafe { layout.GetLineMetrics(Some(&mut lines), &mut line_count) };
                }
                result.check("GetLineMetrics underline")?;
                let mut tops = Vec::with_capacity(line_count as usize);
                let mut top = 0f32;
                for line in lines.iter().take(line_count as usize) {
                    tops.push((top, line.baseline));
                    top += line.height;
                }
                let mut ranges = vec![DWRITE_HIT_TEST_METRICS::default(); value.len().max(1)];
                let mut count = 0;
                // SAFETY: tampon de sortie et compteur valides pour la durée de l'appel.
                unsafe {
                    layout.HitTestTextRange(
                        0,
                        value.len() as u32,
                        0.,
                        0.,
                        Some(&mut ranges),
                        &mut count,
                    )
                }
                .check("HitTestTextRange underline")?;
                for h in ranges.into_iter().take(count as usize) {
                    // Ligne du morceau : la dernière qui commence au-dessus de lui.
                    let baseline = tops
                        .iter()
                        .rev()
                        .find(|(t, _)| *t <= h.top + 0.5)
                        .map_or(h.height * 0.8, |(t, b)| t + b);
                    underlines.push((RectF::new(h.left, h.top, h.width, h.height), baseline));
                }
            }
            self.entries.insert(
                key.clone(),
                Entry {
                    layout,
                    size: SizeF {
                        width: metrics.widthIncludingTrailingWhitespace,
                        height: clamped.unwrap_or(metrics.height),
                    },
                    overlines,
                    underlines,
                },
            );
        }
        Ok(&self.entries[&key])
    }
    pub fn draw(
        &self,
        target: &ID2D1RenderTarget,
        brush: &ID2D1Brush,
        rect: RectF,
        value: &Arc<str>,
        format: TextFormat,
        wrap: bool,
    ) {
        let key = Key::new(Arc::clone(value), format, rect.right - rect.left, wrap);
        let entry = &self.entries[&key];
        // SAFETY: passe de dessin ouverte ; mise en page du cache et pinceau vivants.
        unsafe {
            if format.clip {
                target.PushAxisAlignedClip(&rect_f(rect), D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
            }
            target.DrawTextLayout(
                Vector2 {
                    X: rect.left,
                    Y: rect.top,
                },
                &entry.layout,
                brush,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
            );
        }
        if format.decoration.overline {
            for r in &entry.overlines {
                let y = rect.top + r.top + format.size * 0.08;
                // SAFETY: passe de dessin ouverte ; pinceau vivant, style par défaut.
                unsafe {
                    target.DrawLine(
                        Vector2 {
                            X: rect.left + r.left,
                            Y: y,
                        },
                        Vector2 {
                            X: rect.left + r.right,
                            Y: y,
                        },
                        brush,
                        (format.size / 16.).max(1.),
                        None,
                    );
                }
            }
        }
        if !entry.underlines.is_empty() {
            let style = format.decoration_style;
            // Épaisseur et position par défaut proches de celles des polices courantes.
            let thickness = style
                .thickness
                .unwrap_or((format.size / 14.).max(1.))
                .max(0.);
            let offset = style.offset.unwrap_or(format.size * 0.12);
            // SAFETY: passe de dessin ouverte ; couleur valide.
            let own = style
                .color
                .and_then(|c| unsafe { target.CreateSolidColorBrush(&color_f(c), None) }.ok());
            let ink: &ID2D1Brush = match &own {
                Some(b) => b,
                None => brush,
            };
            for (r, baseline) in &entry.underlines {
                let top = rect.top + baseline + offset;
                // SAFETY: passe de dessin ouverte ; pinceau vivant.
                unsafe {
                    target.FillRectangle(
                        &rect_f(RectF::new(
                            rect.left + r.left,
                            top,
                            r.right - r.left,
                            thickness,
                        )),
                        ink,
                    );
                }
            }
        }
        if format.clip {
            // SAFETY: dépile le clip poussé ci-dessus.
            unsafe { target.PopAxisAlignedClip() };
        }
    }
}
