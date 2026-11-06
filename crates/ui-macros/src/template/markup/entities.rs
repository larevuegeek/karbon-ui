//! Entités HTML du texte et des valeurs d'attribut.
use super::Loc;
use crate::Result;

/// Entités HTML usuelles.
fn entity(name: &str) -> Option<char> {
    Some(match name {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        "nbsp" => '\u{a0}',
        "copy" => '©',
        "reg" => '®',
        "trade" => '™',
        "hellip" => '…',
        "mdash" => '—',
        "ndash" => '–',
        "laquo" => '«',
        "raquo" => '»',
        "lsquo" => '‘',
        "rsquo" => '’',
        "ldquo" => '“',
        "rdquo" => '”',
        "euro" => '€',
        "middot" => '·',
        "bull" => '•',
        "times" => '×',
        "divide" => '÷',
        "deg" => '°',
        "larr" => '←',
        "rarr" => '→',
        "uarr" => '↑',
        "darr" => '↓',
        "thinsp" => '\u{2009}',
        "ensp" => '\u{2002}',
        "emsp" => '\u{2003}',
        "lbrace" => '{',
        "rbrace" => '}',
        _ => return None,
    })
}
/// Décode les entités de `raw` (`&amp;`, `&#233;`, `&#xE9;`…).
pub(super) fn decode(raw: &str, loc: Loc) -> Result<String> {
    if !raw.contains('&') {
        return Ok(raw.to_owned());
    }
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let end = rest[1..]
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '#'))
            .map(|i| i + 1);
        match end {
            Some(end) if rest[end..].starts_with(';') && end > 1 => {
                let name = &rest[1..end];
                let c = if let Some(number) = name.strip_prefix('#') {
                    let value = if let Some(hex) = number.strip_prefix(['x', 'X']) {
                        u32::from_str_radix(hex, 16).ok()
                    } else {
                        number.parse::<u32>().ok()
                    };
                    value
                        .and_then(char::from_u32)
                        .ok_or_else(|| loc.error(format!("Invalid character reference &{name};")))?
                } else {
                    entity(name).ok_or_else(|| {
                        loc.error(format!(
                            "Unknown HTML entity &{name}; (write &amp; for a literal '&')"
                        ))
                    })?
                };
                out.push(c);
                rest = &rest[end + 1..];
            }
            _ => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    Ok(out)
}
