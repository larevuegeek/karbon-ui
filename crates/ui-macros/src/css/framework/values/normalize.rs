//! Normalisation des valeurs : `rem` converti en px (racine de 16 px) et substitution
//! des `var()` connus à la compilation ; relevé des variables lues par une valeur.

/// Taille de police racine supposée pour `rem` (celle des navigateurs).
const ROOT_FONT: f32 = 16.;
/// Remplace les nombres en `rem` par des px (hors chaînes).
pub fn rem_to_px(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = String::with_capacity(value.len());
    let mut i = 0;
    let mut quote = None;
    while i < bytes.len() {
        let c = bytes[i];
        if let Some(q) = quote {
            // Caractère entier : une chaîne (`font-family: "Hélvetica"`) n'est pas que de
            // l'ASCII, et un octet recopié seul en ferait un autre caractère.
            let whole = value[i..].chars().next().unwrap_or(c as char);
            out.push(whole);
            if c == q {
                quote = None;
            }
            i += whole.len_utf8();
            continue;
        }
        if c == b'"' || c == b'\'' {
            quote = Some(c);
            out.push(c as char);
            i += 1;
            continue;
        }
        let starts_number = (c.is_ascii_digit() || c == b'.')
            && (i == 0
                || !(bytes[i - 1].is_ascii_alphanumeric()
                    || bytes[i - 1] == b'-'
                    || bytes[i - 1] == b'_'))
            || ((c == b'-' || c == b'+')
                && bytes
                    .get(i + 1)
                    .is_some_and(|b| b.is_ascii_digit() || *b == b'.')
                && (i == 0 || !(bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_')));
        if starts_number {
            let start = i;
            i += 1;
            while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                i += 1;
            }
            // Exposant.
            if i + 1 < bytes.len()
                && (bytes[i] == b'e' || bytes[i] == b'E')
                && (bytes[i + 1].is_ascii_digit()
                    || ((bytes[i + 1] == b'-' || bytes[i + 1] == b'+')
                        && bytes.get(i + 2).is_some_and(u8::is_ascii_digit)))
            {
                i += 2;
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
            }
            let number = &value[start..i];
            let unit_end = value[i..]
                .find(|c: char| !(c.is_ascii_alphabetic()))
                .map_or(value.len(), |n| i + n);
            if &value[i..unit_end] == "rem"
                && let Ok(n) = number.parse::<f32>()
            {
                out += &format!("{}px", trim_float(n * ROOT_FONT));
                i = unit_end;
            } else {
                out += number;
            }
            continue;
        }
        let ch = value[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}
fn trim_float(n: f32) -> String {
    let s = format!("{n:.4}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-0" {
        "0".into()
    } else {
        s.into()
    }
}
/// Remplace `var(--x[, repli])` par la valeur de `known` quand elle y figure ; les
/// autres restent (leur repli éventuellement substitué).
pub fn substitute(value: &str, known: &dyn Fn(&str) -> Option<String>, depth: usize) -> String {
    if depth > 16 || !value.contains("var(") {
        return value.to_owned();
    }
    let mut out = String::new();
    let mut rest = value;
    while let Some(start) = find_var(rest) {
        out += &rest[..start];
        let open = start + 4;
        let Some(len) = group_len(&rest[open..]) else {
            out += &rest[start..];
            return out;
        };
        let inner = &rest[open..open + len];
        let (name, fallback) = match top_comma(inner) {
            Some(i) => (inner[..i].trim(), Some(&inner[i + 1..])),
            None => (inner.trim(), None),
        };
        match known(name) {
            Some(v) => out += &substitute(&v, known, depth + 1),
            None => {
                out += "var(";
                out += name;
                if let Some(f) = fallback {
                    out += ",";
                    out += &substitute(f, known, depth + 1);
                }
                out += ")";
            }
        }
        rest = &rest[open + len + 1..];
    }
    out += rest;
    out
}
pub(super) fn find_var(text: &str) -> Option<usize> {
    let mut search = 0;
    while let Some(i) = text[search..].find("var(") {
        let at = search + i;
        let before = text[..at].chars().next_back();
        if before.is_none_or(|c| !(c.is_ascii_alphanumeric() || c == '-' || c == '_')) {
            return Some(at);
        }
        search = at + 4;
    }
    None
}
/// Longueur du contenu d'un groupe ouvert (jusqu'à la `)` correspondante exclue).
pub(super) fn group_len(text: &str) -> Option<usize> {
    let mut depth = 1;
    let mut quote = None;
    for (i, c) in text.char_indices() {
        if let Some(q) = quote {
            if c == q {
                quote = None;
            }
            continue;
        }
        match c {
            '"' | '\'' => quote = Some(c),
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}
fn top_comma(text: &str) -> Option<usize> {
    let mut depth = 0;
    let mut quote = None;
    for (i, c) in text.char_indices() {
        if let Some(q) = quote {
            if c == q {
                quote = None;
            }
            continue;
        }
        match c {
            '"' | '\'' => quote = Some(c),
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => return Some(i),
            _ => {}
        }
    }
    None
}
/// Noms des variables lues par une valeur.
pub fn references(value: &str, out: &mut Vec<String>) {
    let mut rest = value;
    while let Some(start) = find_var(rest) {
        let open = start + 4;
        let Some(len) = group_len(&rest[open..]) else {
            return;
        };
        let inner = &rest[open..open + len];
        match top_comma(inner) {
            Some(i) => {
                out.push(inner[..i].trim().to_owned());
                references(&inner[i + 1..], out);
            }
            None => out.push(inner.trim().to_owned()),
        }
        rest = &rest[open + len + 1..];
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rem_and_substitution() {
        assert_eq!(rem_to_px(".375rem .75rem"), "6px 12px");
        assert_eq!(rem_to_px("calc(1.375rem + 1.5vw)"), "calc(22px + 1.5vw)");
        assert_eq!(rem_to_px("\"1rem\" -0.5rem"), "\"1rem\" -8px");
        let known = |n: &str| (n == "--a").then(|| "1px".to_string());
        assert_eq!(
            substitute("var(--a) var(--b, var(--a))", &known, 0),
            "1px var(--b, 1px)"
        );
        let mut refs = Vec::new();
        references("rgba(var(--x), var(--y, var(--z)))", &mut refs);
        assert_eq!(refs, ["--x", "--y", "--z"]);
    }
}
