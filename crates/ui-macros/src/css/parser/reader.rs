pub(super) use super::scan::{block_end, scan, skip_space};
use super::*;

#[derive(Clone, Debug)]
pub struct Declaration {
    pub name: String,
    pub value: String,
    pub at: usize,
}
#[derive(Clone, Debug)]
pub enum Item {
    Media {
        query: String,
        items: Vec<Item>,
        at: usize,
    },
    Keyframes {
        name: String,
        frames: Vec<Item>,
        at: usize,
    },
    Import {
        path: String,
        at: usize,
    },
    Rule {
        selectors: Vec<String>,
        declarations: Vec<Declaration>,
        at: usize,
    },
    /// `@theme [options] { --name: value; @keyframes … }` (Tailwind) : variables du
    /// thème, et animations (`@keyframes` complets, texte source).
    Theme {
        declarations: Vec<Declaration>,
        keyframes: Vec<(String, String)>,
        at: usize,
    },
    /// `@custom-variant name (selector);` (Tailwind).
    CustomVariant {
        name: String,
        selector: String,
        at: usize,
    },
}
#[derive(Debug)]
pub struct Error {
    pub at: usize,
    pub message: String,
}
pub(super) type Result<T> = std::result::Result<T, Error>;
pub(super) fn error(at: usize, message: impl Into<String>) -> Error {
    Error {
        at,
        message: message.into(),
    }
}
/// Profondeur maximale des blocs `@media`, `@container` et `@keyframes` imbriqués : la
/// lecture et la compilation de la feuille sont récursives.
const MAX_DEPTH: usize = 32;
/// Règles et at-rules d'une feuille, dans l'ordre du texte.
pub fn parse(source: &str) -> Result<Vec<Item>> {
    read(source, 0)
}
/// Profondeur du bloc ouvert à `at` dans un bloc de profondeur `depth`.
fn deeper(depth: usize, at: usize) -> Result<usize> {
    if depth + 1 > MAX_DEPTH {
        return Err(error(
            at,
            format!("CSS blocks nest deeper than {MAX_DEPTH} levels"),
        ));
    }
    Ok(depth + 1)
}
fn read(source: &str, depth: usize) -> Result<Vec<Item>> {
    // Replace comments with spaces without changing byte offsets used in diagnostics.
    let mut bytes = source.as_bytes().to_vec();
    let mut i = 0;
    let mut quote = None;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            i += 2;
            continue;
        }
        if let Some(q) = quote {
            if bytes[i] == q {
                quote = None;
            }
            i += 1;
            continue;
        }
        if bytes[i] == b'\"' || bytes[i] == b'\'' {
            quote = Some(bytes[i]);
            i += 1;
            continue;
        }
        if bytes.get(i..i + 2) == Some(b"/*") {
            let start = i;
            i += 2;
            while i + 1 < bytes.len() && &bytes[i..i + 2] != b"*/" {
                i += 1;
            }
            if i + 1 >= bytes.len() {
                return Err(error(start, "Unclosed CSS comment"));
            }
            i += 2;
            for byte in &mut bytes[start..i] {
                if *byte != b'\n' && *byte != b'\r' {
                    *byte = b' ';
                }
            }
        } else {
            i += 1;
        }
    }
    let text = String::from_utf8(bytes).unwrap();
    let mut cursor = 0;
    let mut result = Vec::new();
    while cursor < text.len() {
        cursor = skip_space(&text, cursor);
        if cursor == text.len() {
            break;
        }
        let at = cursor;
        if text[cursor..].starts_with("@import") {
            cursor += 7;
            let end = scan(&text, cursor, ';')?;
            let value = text[cursor..end].trim();
            if value.len() < 2
                || !(value.starts_with('"') && value.ends_with('"')
                    || value.starts_with('\'') && value.ends_with('\''))
            {
                return Err(error(
                    at,
                    "Use @import \"relative.css\"; (local files only)",
                ));
            }
            let path = &value[1..value.len() - 1];
            if path.contains('\\') || path.contains("://") {
                return Err(error(at, "Use a local CSS import path with / separators"));
            }
            result.push(Item::Import {
                path: path.into(),
                at,
            });
            cursor = end + 1;
            continue;
        }
        if text[cursor..].starts_with("@media ") || text[cursor..].starts_with("@container ") {
            let open = scan(&text, cursor, '{')?;
            let end = block_end(&text, open)?;
            let query = conditional_query(&text[cursor..open]);
            let inner = deeper(depth, at)?;
            let mut items = read(&text[open + 1..end], inner).map_err(|mut e| {
                e.at += open + 1;
                e
            })?;
            shift(&mut items, open + 1);
            result.push(Item::Media { query, items, at });
            cursor = end + 1;
            continue;
        }
        if text[cursor..].starts_with("@keyframes ") {
            let open = scan(&text, cursor, '{')?;
            let name = text[cursor + 11..open].trim();
            class_name(name).map_err(|m| error(at, m))?;
            let end = block_end(&text, open)?;
            // Positions des étapes relatives au texte, comme celles des blocs `@media`.
            let inner = deeper(depth, at)?;
            let mut frames = read(&text[open + 1..end], inner).map_err(|mut e| {
                e.at += open + 1;
                e
            })?;
            shift(&mut frames, open + 1);
            result.push(Item::Keyframes {
                name: name.into(),
                frames,
                at,
            });
            cursor = end + 1;
            continue;
        }
        if text[cursor..].starts_with("@theme")
            && text[cursor + 6..].starts_with(|c: char| c == '{' || c.is_ascii_whitespace())
        {
            let open = scan(&text, cursor, '{')?;
            let options = text[cursor + 6..open].split_whitespace();
            for option in options {
                if !["inline", "static", "default", "reference"].contains(&option) {
                    return Err(error(at, format!("Unknown @theme option '{option}'")));
                }
            }
            let end = block_end(&text, open)?;
            let (declarations, keyframes) = theme_block(&text, open + 1, end)?;
            result.push(Item::Theme {
                declarations,
                keyframes,
                at,
            });
            cursor = end + 1;
            continue;
        }
        if text[cursor..].starts_with("@custom-variant ") {
            let end = scan(&text, cursor, ';')?;
            let body = text[cursor + 16..end].trim();
            let (name, selector) = body.split_once(char::is_whitespace).ok_or_else(|| {
                error(at, "Use @custom-variant name (selector); e.g. @custom-variant dark (&:where(.dark, .dark *));")
            })?;
            let selector = selector
                .trim()
                .strip_prefix('(')
                .and_then(|s| s.strip_suffix(')'))
                .ok_or_else(|| error(at, "@custom-variant expects a selector in parentheses; the block form with @slot is not implemented"))?;
            result.push(Item::CustomVariant {
                name: name.into(),
                selector: selector.trim().into(),
                at,
            });
            cursor = end + 1;
            continue;
        }
        if text[cursor..].starts_with("@source ") {
            // Tailwind lit les classes directement dans les templates compilés : les
            // sources à analyser n'ont pas d'objet ici.
            cursor = scan(&text, cursor, ';')? + 1;
            continue;
        }
        if text[cursor..].starts_with('@') {
            return Err(error(
                at,
                "CSS at-rule not implemented; supported: local @import, @media, @container, @keyframes, and with Tailwind @theme, @custom-variant and @source",
            ));
        }
        let open = scan(&text, cursor, '{')?;
        let selector_text = text[cursor..open].to_owned();
        let selectors = split(&text[cursor..open], ',')
            .map_err(|m| error(at, m))?
            .into_iter()
            .map(str::to_owned)
            .collect();
        cursor = open + 1;
        let mut declarations = Vec::new();
        // Règles conditionnelles imbriquées (`.a { @container (…) { … } }`), placées après
        // la règle : mêmes sélecteurs, déclarations du bloc.
        let mut nested = Vec::new();
        loop {
            cursor = skip_space(&text, cursor);
            if text.as_bytes().get(cursor) == Some(&b'}') {
                cursor += 1;
                break;
            }
            if cursor == text.len() {
                return Err(error(at, "Missing closing } in CSS rule"));
            }
            let at = cursor;
            if text[cursor..].starts_with("@media ") || text[cursor..].starts_with("@container ") {
                let inner_open = scan(&text, cursor, '{')?;
                let inner_end = block_end(&text, inner_open)?;
                let query = conditional_query(&text[cursor..inner_open]);
                // Le bloc est relu comme une règle des mêmes sélecteurs ; les positions
                // des diagnostics restent celles du fichier.
                let padding = inner_open + 1 - selector_text.len() - 1;
                let body = format!(
                    "{}{selector_text}{{{}}}",
                    " ".repeat(padding),
                    &text[inner_open + 1..inner_end]
                );
                let items = read(&body, deeper(depth, at)?)?;
                nested.push(Item::Media { query, items, at });
                cursor = inner_end + 1;
                continue;
            }
            if text[cursor..].starts_with('@') {
                return Err(error(
                    at,
                    "Only @media and @container can be nested inside a CSS rule",
                ));
            }
            let colon = scan(&text, cursor, ':')?;
            let name = text[cursor..colon].trim();
            let name = if name.starts_with("--") {
                name.into()
            } else {
                name.to_ascii_lowercase()
            };
            if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
                return Err(error(at, "Invalid CSS property name"));
            }
            let rest = &text[colon + 1..];
            let mut end = colon + 1;
            // The declaration can end at ';' or the final '}', outside functions/strings.
            let mut q = None;
            let mut escape = false;
            let mut depth = 0;
            for c in rest.chars() {
                if escape {
                    escape = false;
                    end += c.len_utf8();
                    continue;
                }
                if c == '\\' {
                    escape = true;
                    end += 1;
                    continue;
                }
                if let Some(current) = q {
                    if c == current {
                        q = None;
                    }
                    end += c.len_utf8();
                    continue;
                }
                match c {
                    '\'' | '"' => q = Some(c),
                    '(' | '[' => depth += 1,
                    ')' | ']' => {
                        if depth == 0 {
                            return Err(error(end, "Unbalanced CSS function"));
                        }
                        depth -= 1;
                    }
                    ';' | '}' if depth == 0 => break,
                    '{' => return Err(error(end, "Nested CSS rules are not implemented")),
                    _ => {}
                }
                end += c.len_utf8();
            }
            if end == text.len() {
                return Err(error(at, "Unclosed CSS declaration or rule"));
            }
            let value = text[colon + 1..end].trim();
            if value.is_empty() {
                return Err(error(at, "Empty CSS value"));
            }
            if value.contains("!important") {
                return Err(error(
                    at,
                    "!important is not implemented; classes apply in the declared order",
                ));
            }
            split(value, ',').map_err(|m| error(at, m))?;
            declarations.push(Declaration {
                name,
                value: value.into(),
                at,
            });
            cursor = end;
            if text.as_bytes()[cursor] == b';' {
                cursor += 1;
            }
        }
        result.push(Item::Rule {
            selectors,
            declarations,
            at,
        });
        result.extend(nested);
    }
    Ok(result)
}
/// Requête d'un en-tête `@media …` ou `@container …` ; une requête de conteneur garde
/// le préfixe `@container ` pour la compilation de la condition.
pub(super) fn conditional_query(header: &str) -> String {
    match header.strip_prefix("@media ") {
        Some(query) => query.trim().to_owned(),
        None => format!(
            "@container {}",
            header.trim_start_matches("@container").trim()
        ),
    }
}
/// Déclarations et `@keyframes` (nom, texte) d'un bloc `@theme`.
pub(super) type ThemeBlock = (Vec<Declaration>, Vec<(String, String)>);
/// Contenu d'un bloc `@theme` : déclarations `--nom: valeur;` et `@keyframes` imbriqués.
pub(super) fn theme_block(text: &str, mut cursor: usize, end: usize) -> Result<ThemeBlock> {
    let mut declarations = Vec::new();
    let mut keyframes = Vec::new();
    loop {
        cursor = skip_space(text, cursor);
        if cursor >= end {
            break;
        }
        let at = cursor;
        if text[cursor..].starts_with("@keyframes ") {
            let open = scan(text, cursor, '{')?;
            let name = text[cursor + 11..open].trim();
            class_name(name).map_err(|m| error(at, m))?;
            let close = block_end(text, open)?;
            keyframes.push((name.to_owned(), text[cursor..=close].to_owned()));
            cursor = close + 1;
            continue;
        }
        if !text[cursor..].starts_with("--") {
            return Err(error(
                at,
                "@theme accepts custom properties (--name: value;) and @keyframes only",
            ));
        }
        let colon = scan(text, cursor, ':')?;
        let name = text[cursor..colon].trim();
        if !name[2..]
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'*')
        {
            return Err(error(at, "Invalid @theme variable name"));
        }
        let stop = match scan(text, colon + 1, ';') {
            Ok(stop) if stop < end => stop,
            _ => end,
        };
        let value = text[colon + 1..stop]
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if value.is_empty() {
            return Err(error(at, "Empty @theme value"));
        }
        declarations.push(Declaration {
            name: name.to_owned(),
            value,
            at,
        });
        cursor = stop + 1;
    }
    Ok((declarations, keyframes))
}

pub(super) fn shift(items: &mut [Item], offset: usize) {
    for item in items {
        match item {
            Item::Import { at, .. } => *at += offset,
            Item::Rule {
                at, declarations, ..
            } => {
                *at += offset;
                for d in declarations {
                    d.at += offset;
                }
            }
            Item::Media { at, items, .. } => {
                *at += offset;
                shift(items, offset);
            }
            Item::Keyframes { at, frames, .. } => {
                *at += offset;
                shift(frames, offset);
            }
            Item::Theme {
                at, declarations, ..
            } => {
                *at += offset;
                for d in declarations {
                    d.at += offset;
                }
            }
            Item::CustomVariant { at, .. } => *at += offset,
        }
    }
}
