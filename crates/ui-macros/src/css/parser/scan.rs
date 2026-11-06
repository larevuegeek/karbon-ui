//! Balayage du texte CSS : découpage au niveau supérieur, fin d'un bloc, recherche
//! d'un caractère hors chaînes et parenthèses.
use super::{Result, error};
/// Split only at the top level, preserving quoted strings and function arguments.
pub fn split(value: &str, separator: char) -> std::result::Result<Vec<&str>, String> {
    let mut quote = None;
    let mut escape = false;
    let mut stack = Vec::new();
    let mut start = 0;
    let mut parts = Vec::new();
    for (i, c) in value.char_indices() {
        if escape {
            escape = false;
            continue;
        }
        if c == '\\' {
            escape = true;
            continue;
        }
        if let Some(q) = quote {
            if c == q {
                quote = None;
            }
            continue;
        }
        match c {
            '\'' | '"' => quote = Some(c),
            '(' | '[' => stack.push(c),
            ')' => {
                if stack.pop() != Some('(') {
                    return Err("Unbalanced parentheses".into());
                }
            }
            ']' => {
                if stack.pop() != Some('[') {
                    return Err("Unbalanced brackets".into());
                }
            }
            c if c == separator && stack.is_empty() => {
                parts.push(value[start..i].trim());
                start = i + c.len_utf8();
            }
            _ => {}
        }
    }
    if quote.is_some() || !stack.is_empty() || escape {
        return Err("Unclosed CSS string or function".into());
    }
    parts.push(value[start..].trim());
    Ok(parts)
}
pub(super) fn block_end(text: &str, open: usize) -> Result<usize> {
    let mut depth = 0usize;
    let mut quote = None;
    let mut escape = false;
    for (offset, c) in text[open..].char_indices() {
        if escape {
            escape = false;
            continue;
        }
        if c == '\\' {
            escape = true;
            continue;
        }
        if let Some(q) = quote {
            if c == q {
                quote = None;
            }
            continue;
        }
        match c {
            '"' | '\'' => quote = Some(c),
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(open + offset);
                }
            }
            _ => {}
        }
    }
    Err(error(open, "Unclosed CSS block"))
}
pub(super) fn skip_space(text: &str, mut i: usize) -> usize {
    while text.as_bytes().get(i).is_some_and(u8::is_ascii_whitespace) {
        i += 1;
    }
    i
}
pub(super) fn scan(text: &str, start: usize, wanted: char) -> Result<usize> {
    let mut quote = None;
    let mut escape = false;
    let mut depth = 0;
    for (offset, c) in text[start..].char_indices() {
        if escape {
            escape = false;
            continue;
        }
        if c == '\\' {
            escape = true;
            continue;
        }
        if let Some(q) = quote {
            if c == q {
                quote = None;
            }
            continue;
        }
        if c == '"' || c == '\'' {
            quote = Some(c);
            continue;
        }
        if depth == 0 && c == wanted {
            return Ok(start + offset);
        }
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => {
                if depth == 0 {
                    return Err(error(start + offset, "Unbalanced CSS delimiters"));
                }
                depth -= 1;
            }
            '}' | ';' | '{' if depth == 0 => {
                return Err(error(start + offset, format!("Expected '{wanted}'")));
            }
            _ => {}
        }
    }
    Err(error(start, format!("Expected '{wanted}'")))
}
