//! Images `url("data:image/svg+xml,…")` des feuilles de frameworks : l'URL est décodée
//! (pourcentages ou base64), son SVG compilé par le compilateur des `<svg>` de
//! templates en un `SvgImage` statique, et la valeur devient un jeton
//! `Token::Image`. Rien n'est analysé à l'exécution ; une URL déjà vue pendant la
//! compilation de la crate réutilise son code.
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    /// Code du jeton (ou erreur) par URL.
    static CACHE: RefCell<HashMap<String, Result<String, String>>> = RefCell::new(HashMap::new());
}

/// Contenu d'un `url(…)` commençant à `chars[open]` (juste après la parenthèse) :
/// texte de l'URL (guillemets retirés) et index après la `)` fermante.
pub fn url_argument(chars: &[char], open: usize) -> Result<(String, usize), String> {
    let mut i = open;
    while i < chars.len() && chars[i].is_whitespace() {
        i += 1;
    }
    let mut out = String::new();
    match chars.get(i) {
        Some(&q @ ('"' | '\'')) => {
            i += 1;
            while i < chars.len() && chars[i] != q {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    i += 1;
                }
                out.push(chars[i]);
                i += 1;
            }
            if i >= chars.len() {
                return Err("unclosed url() string".into());
            }
            i += 1;
        }
        _ => {
            while i < chars.len() && chars[i] != ')' && !chars[i].is_whitespace() {
                out.push(chars[i]);
                i += 1;
            }
        }
    }
    while i < chars.len() && chars[i].is_whitespace() {
        i += 1;
    }
    if chars.get(i) != Some(&')') {
        return Err("invalid url()".into());
    }
    Ok((out, i + 1))
}

/// Texte SVG d'une URL `data:image/svg+xml[;charset=…][;base64],…`.
pub fn decode(url: &str) -> Result<String, String> {
    let rest = url
        .trim()
        .strip_prefix("data:")
        .ok_or("url() other than an SVG data URL")?;
    let (header, data) = rest.split_once(',').ok_or("invalid data URL")?;
    let mut params = header.split(';');
    if !params
        .next()
        .is_some_and(|m| m.trim().eq_ignore_ascii_case("image/svg+xml"))
    {
        return Err("url() other than an SVG data URL".into());
    }
    let base64 = params.any(|p| p.trim().eq_ignore_ascii_case("base64"));
    let bytes = if base64 {
        base64_decode(&percent_decode(data.as_bytes()))?
    } else {
        percent_decode(data.as_bytes())
    };
    String::from_utf8(bytes).map_err(|_| "data URL is not UTF-8".into())
}
fn percent_decode(data: &[u8]) -> Vec<u8> {
    let hex = |b: u8| (b as char).to_digit(16).map(|d| d as u8);
    let mut out = Vec::with_capacity(data.len());
    let mut i = 0;
    while i < data.len() {
        if data[i] == b'%'
            && let (Some(a), Some(b)) = (
                data.get(i + 1).copied().and_then(hex),
                data.get(i + 2).copied().and_then(hex),
            )
        {
            out.push(a * 16 + b);
            i += 3;
            continue;
        }
        out.push(data[i]);
        i += 1;
    }
    out
}
fn base64_decode(data: &[u8]) -> Result<Vec<u8>, String> {
    let value = |c: u8| -> Option<u32> {
        Some(match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            _ => return None,
        } as u32)
    };
    let mut out = Vec::with_capacity(data.len() * 3 / 4);
    let (mut buffer, mut bits) = (0u32, 0u32);
    for &c in data {
        if c == b'=' {
            break;
        }
        if c.is_ascii_whitespace() {
            continue;
        }
        let v = value(c).ok_or("invalid base64 in data URL")?;
        buffer = (buffer << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }
    Ok(out)
}

/// Code `Token::Image(…)` de l'URL ; erreur : catégorie du rapport.
pub fn token(url: &str) -> Result<String, String> {
    if let Some(found) = CACHE.with(|c| c.borrow().get(url).cloned()) {
        return found;
    }
    let result = decode(url).and_then(|svg| compile(&svg));
    CACHE.with(|c| c.borrow_mut().insert(url.to_owned(), result.clone()));
    result
}
fn compile(svg: &str) -> Result<String, String> {
    const TOKEN: &str = "__ui_root::ui::styling::tokens::Token";
    // Hors d'une macro (tests unitaires du compilateur), pas de span : le texte est
    // seulement vérifié grossièrement et le code produit n'est jamais compilé.
    if !proc_macro::is_available() {
        return if svg.trim_start().starts_with("<svg") {
            Ok(format!("{TOKEN}::Image(/* svg */)"))
        } else {
            Err("url(): SVG root expected".into())
        };
    }
    use crate::template::markup::Node;
    let span = proc_macro::Span::call_site();
    let nodes =
        crate::template::markup::parse_text(crate::svg::strip_prolog(svg), "data URL", span)
            .map_err(|e| format!("url(): {}", short(&e.message)))?;
    let mut roots = nodes
        .into_iter()
        .filter(|n| !matches!(n, Node::Text(t, _) if t.trim().is_empty()));
    let (Some(Node::Element(root)), None) = (roots.next(), roots.next()) else {
        return Err("url(): SVG root expected".into());
    };
    if root.name != "svg" {
        return Err("url(): SVG root expected".into());
    }
    let compiled =
        crate::svg::compile(&root).map_err(|e| format!("url(): {}", short(&e.message)))?;
    Ok(format!("{TOKEN}::Image({})", compiled.image))
}
/// Message d'erreur sans position (catégorie du rapport).
fn short(message: &str) -> String {
    let text = message.rsplit(": ").next().unwrap_or(message);
    text.chars().take(60).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decodes_percent_and_base64_urls() {
        assert_eq!(
            decode("data:image/svg+xml,%3csvg xmlns='http://www.w3.org/2000/svg'/%3e").unwrap(),
            "<svg xmlns='http://www.w3.org/2000/svg'/>"
        );
        // « <svg/> » en base64.
        assert_eq!(
            decode("data:image/svg+xml;base64,PHN2Zy8+").unwrap(),
            "<svg/>"
        );
        assert!(decode("data:image/png;base64,AAAA").is_err());
        assert!(decode("images/icon.svg").is_err());
        let chars = "url(\"data:x,a)b\") 1px".chars().collect::<Vec<_>>();
        let (url, next) = url_argument(&chars, 4).unwrap();
        assert_eq!((url.as_str(), next), ("data:x,a)b", 17));
    }
}
