//! Coupure des lignes d'un texte simple : retours à la ligne, espaces, ellipse et
//! limite de lignes.
use super::{HeadlessTextMeasurer, clamp_width};
use crate::ui::styling::text::TextFormat;

/// Lignes de `text` coupées à `max_chars` caractères (voir la documentation du type).
pub(super) fn wrap(text: &str, max_chars: usize) -> Vec<&str> {
    let max_chars = max_chars.max(1);
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let mut rest = paragraph.strip_suffix('\r').unwrap_or(paragraph);
        let mut wrapped = false;
        loop {
            if rest.is_empty() && wrapped {
                break;
            }
            if rest.chars().count() <= max_chars {
                lines.push(rest);
                break;
            }
            wrapped = true;
            // Dernier espace atteignable : il peut occuper la position `max_chars`, car il
            // n'est pas compté sur la ligne.
            let space = rest
                .char_indices()
                .take(max_chars + 1)
                .filter(|&(byte, c)| c == ' ' && byte > 0)
                .map(|(byte, _)| byte)
                .last()
                .filter(|&byte| !rest[..byte].trim_end_matches(' ').is_empty());
            if let Some(byte) = space {
                lines.push(rest[..byte].trim_end_matches(' '));
                rest = rest[byte..].trim_start_matches(' ');
            } else {
                let cut = rest
                    .char_indices()
                    .nth(max_chars)
                    .map_or(rest.len(), |(byte, _)| byte);
                lines.push(&rest[..cut]);
                rest = &rest[cut..];
            }
        }
    }
    lines
}

/// Nombre de caractères qui tiennent dans `width` ; la tolérance absorbe les arrondis
/// d'une largeur mesurée puis réutilisée par le layout.
pub(super) fn max_chars(width: f32, size: f32) -> usize {
    (clamp_width(width) / (HeadlessTextMeasurer::ADVANCE * size) + 1e-3).floor() as usize
}

/// Lignes affichées d'un texte au format complet : coupure selon `white-space`,
/// espacement des lettres, ellipse et limite de lignes (`line-clamp`). L'ellipse
/// remplace les derniers caractères qui ne tiennent plus avec `…`.
pub(in crate::platform::headless) fn formatted_lines(
    text: &str,
    format: &TextFormat,
    width: f32,
    wrap_lines: bool,
) -> Vec<String> {
    let advance = HeadlessTextMeasurer::ADVANCE * format.size + format.letter_spacing;
    let fit = if advance > 0. {
        (clamp_width(width) / advance + 1e-3).floor() as usize
    } else {
        usize::MAX
    };
    let wrapping = wrap_lines && format.wraps();
    let mut lines: Vec<String> = wrap(text, if wrapping { fit } else { usize::MAX })
        .into_iter()
        .map(str::to_owned)
        .collect();
    let ellipsize = |line: &mut String, force: bool| {
        let count = line.chars().count();
        if !force && count <= fit {
            return;
        }
        if format.ellipsis_start && !force {
            // Ellipse de début (`direction: rtl`) : la fin de la ligne est gardée.
            let keep = fit.saturating_sub(1);
            *line = "\u{2026}".to_owned()
                + line
                    .chars()
                    .skip(count - keep.min(count))
                    .collect::<String>()
                    .trim_start();
            return;
        }
        let keep = count.min(fit.saturating_sub(1));
        *line = line
            .chars()
            .take(keep)
            .collect::<String>()
            .trim_end()
            .to_owned()
            + "\u{2026}";
    };
    let max = format.max_lines as usize;
    if max > 0 && lines.len() > max {
        lines.truncate(max);
        ellipsize(lines.last_mut().unwrap(), true);
    } else if format.ellipsis || max > 0 {
        for line in &mut lines {
            ellipsize(line, false);
        }
    }
    lines
}
