//! Attributs appliqués à une mise en page (espacement, italique, chiffres tabulaires,
//! ellipse, limite de lignes) et ellipse de début mesurée.
use super::*;

/// Espacement, italique, chiffres tabulaires, ellipse et limite de lignes d'une mise
/// en page ; renvoie la hauteur conservée quand `line-clamp` coupe des lignes.
pub(super) fn features(
    factory: &IDWriteFactory,
    layout: &IDWriteTextLayout,
    key: &Key,
    length: u32,
) -> crate::Result<Option<f32>> {
    let range = DWRITE_TEXT_RANGE {
        startPosition: 0,
        length,
    };
    let spacing = f32::from_bits(key.spacing);
    // SAFETY (bloc) : mise en page fraîchement créée, possédée localement ; structures
    // lues pendant chaque appel.
    unsafe {
        if spacing != 0. {
            let layout: IDWriteTextLayout1 = layout.cast().check("QueryInterface TextLayout1")?;
            layout
                .SetCharacterSpacing(0., spacing, 0., range)
                .check("SetCharacterSpacing")?;
        }
        if key.italic {
            layout
                .SetFontStyle(DWRITE_FONT_STYLE_ITALIC, range)
                .check("SetFontStyle italic")?;
        }
        if key.tabular {
            let typography = factory.CreateTypography().check("CreateTypography")?;
            typography
                .AddFontFeature(DWRITE_FONT_FEATURE {
                    nameTag: DWRITE_FONT_FEATURE_TAG_TABULAR_FIGURES,
                    parameter: 1,
                })
                .check("AddFontFeature tnum")?;
            layout
                .SetTypography(&typography, range)
                .check("SetTypography")?;
        }
        if key.ellipsis || key.max_lines > 0 {
            let sign = factory
                .CreateEllipsisTrimmingSign(layout)
                .check("CreateEllipsisTrimmingSign")?;
            let trimming = DWRITE_TRIMMING {
                granularity: DWRITE_TRIMMING_GRANULARITY_CHARACTER,
                delimiter: 0,
                delimiterCount: 0,
            };
            layout.SetTrimming(&trimming, &sign).check("SetTrimming")?;
        }
        if key.max_lines > 0 {
            let mut count = 0;
            // Premier appel : nombre de lignes (E_NOT_SUFFICIENT_BUFFER attendu).
            let _ = layout.GetLineMetrics(None, &mut count);
            let mut lines = vec![DWRITE_LINE_METRICS::default(); count as usize];
            layout
                .GetLineMetrics(Some(&mut lines), &mut count)
                .check("GetLineMetrics")?;
            if count > key.max_lines as u32 {
                let height = lines[..key.max_lines as usize]
                    .iter()
                    .map(|l| l.height)
                    .sum::<f32>();
                layout.SetMaxHeight(height).check("SetMaxHeight")?;
                return Ok(Some(height));
            }
        }
    }
    Ok(None)
}
/// Largeur naturelle (sans retour à la ligne) d'un texte au format de `key`.
fn natural_width(
    factory: &IDWriteFactory,
    format_native: &IDWriteTextFormat,
    key: &Key,
    text: &str,
) -> crate::Result<f32> {
    let value = text.encode_utf16().collect::<Vec<_>>();
    // SAFETY: texte UTF-16 et format vivants pendant l'appel.
    let layout = unsafe { factory.CreateTextLayout(&value, format_native, 1_000_000., 1_000_000.) }
        .check("CreateTextLayout probe")?;
    // SAFETY: mise en page fraîchement créée, possédée localement.
    unsafe { layout.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP) }
        .check("SetWordWrapping probe")?;
    let probe = Key {
        ellipsis: false,
        max_lines: 0,
        ..key.clone()
    };
    features(factory, &layout, &probe, value.len() as u32)?;
    let mut metrics = DWRITE_TEXT_METRICS::default();
    // SAFETY: sortie valide pour la durée de l'appel.
    unsafe { layout.GetMetrics(&mut metrics) }.check("GetMetrics probe")?;
    Ok(metrics.widthIncludingTrailingWhitespace)
}
/// Ellipse de début : octet où commence la partie gardée, si le texte déborde.
pub(super) fn start_cut(
    factory: &IDWriteFactory,
    format_native: &IDWriteTextFormat,
    key: &Key,
    text: &str,
) -> crate::Result<Option<usize>> {
    let width = f32::from_bits(key.width);
    if text.is_empty() || natural_width(factory, format_native, key, text)? <= width {
        return Ok(None);
    }
    crate::ui::styling::text::cut_start(text, |candidate| {
        Ok(natural_width(factory, format_native, key, candidate)? <= width)
    })
    .map(Some)
}
