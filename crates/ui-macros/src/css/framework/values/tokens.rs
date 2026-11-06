//! Découpage d'une valeur en code de jetons (`Token::…`) évalués à l'exécution :
//! nombres, dimensions, couleurs constantes, fonctions, `var()`, images `url()`.
use super::normalize::group_len;
use crate::css::framework::images;
use crate::css::values::constant_color;

const TOKEN: &str = "__ui_root::ui::styling::tokens::Token";
const UNIT: &str = "__ui_root::ui::styling::tokens::Unit";
/// Unité d'une dimension, en variante `Unit`.
fn unit(name: &str) -> Option<&'static str> {
    Some(match name.to_ascii_lowercase().as_str() {
        "px" => "Px",
        "rem" => "Rem",
        "em" => "Em",
        "vw" => "Vw",
        "vh" | "dvh" | "svh" | "lvh" => "Vh",
        "dvw" | "svw" | "lvw" => "Vw",
        "vmin" => "Vmin",
        "vmax" => "Vmax",
        "deg" => "Deg",
        "turn" => "Turn",
        "rad" => "Rad",
        "ms" => "Ms",
        "s" => "S",
        "fr" => "Fr",
        "pt" => "Pt",
        "ch" => "Ch",
        _ => return None,
    })
}
/// Découpe `value` en code de jetons (`Token::…`) ; erreur : catégorie de refus.
pub fn tokenize(value: &str) -> Result<Vec<String>, String> {
    let chars = value.chars().collect::<Vec<_>>();
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut i = 0;
    // Le jeton précédent permet de distinguer `-2px` (nombre) de `- 2px` (opérateur).
    let mut separated = true;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            separated = true;
            i += 1;
            continue;
        }
        let number_start = c.is_ascii_digit()
            || (c == '.' && chars.get(i + 1).is_some_and(char::is_ascii_digit))
            || ((c == '-' || c == '+')
                && separated
                && chars
                    .get(i + 1)
                    .is_some_and(|d| d.is_ascii_digit() || *d == '.'));
        if number_start {
            let start = i;
            i += 1;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            if i + 1 < chars.len()
                && (chars[i] == 'e' || chars[i] == 'E')
                && (chars[i + 1].is_ascii_digit()
                    || ((chars[i + 1] == '-' || chars[i + 1] == '+')
                        && chars.get(i + 2).is_some_and(char::is_ascii_digit)))
            {
                i += 2;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
            }
            let text = chars[start..i].iter().collect::<String>();
            let n = text
                .parse::<f32>()
                .ok()
                .filter(|n| n.is_finite())
                .ok_or("invalid number")?;
            if chars.get(i) == Some(&'%') {
                i += 1;
                out.push(format!("{TOKEN}::Percentage({n:?}f32)"));
            } else {
                let unit_start = i;
                while i < chars.len() && chars[i].is_ascii_alphabetic() {
                    i += 1;
                }
                let name = chars[unit_start..i].iter().collect::<String>();
                if name.is_empty() {
                    out.push(format!("{TOKEN}::Number({n:?}f32)"));
                } else {
                    let u = unit(&name).ok_or_else(|| format!("unit {name}"))?;
                    out.push(format!("{TOKEN}::Dimension({n:?}f32,{UNIT}::{u})"));
                }
            }
            separated = false;
            continue;
        }
        match c {
            ',' => {
                out.push(format!("{TOKEN}::Comma"));
                i += 1;
                separated = true;
                continue;
            }
            '/' => {
                out.push(format!("{TOKEN}::Slash"));
                i += 1;
                separated = true;
                continue;
            }
            ')' => {
                if depth == 0 {
                    return Err("unbalanced parentheses".into());
                }
                depth -= 1;
                out.push(format!("{TOKEN}::Close"));
                i += 1;
                separated = false;
                continue;
            }
            '(' => {
                depth += 1;
                out.push(format!("{TOKEN}::Function(\"\")"));
                i += 1;
                separated = true;
                continue;
            }
            '"' | '\'' => {
                let mut text = String::new();
                i += 1;
                while i < chars.len() && chars[i] != c {
                    if chars[i] == '\\' && i + 1 < chars.len() {
                        i += 1;
                    }
                    text.push(chars[i]);
                    i += 1;
                }
                i += 1;
                out.push(format!("{TOKEN}::String({text:?})"));
                separated = false;
                continue;
            }
            '#' => {
                let start = i;
                i += 1;
                while i < chars.len() && chars[i].is_ascii_alphanumeric() {
                    i += 1;
                }
                let text = chars[start..i].iter().collect::<String>();
                let rgba = constant_color(&text)
                    .ok()
                    .flatten()
                    .ok_or_else(|| format!("invalid colour {text}"))?;
                out.push(color_code(rgba));
                separated = false;
                continue;
            }
            '+' | '-' | '*'
                if !(c == '-'
                    && chars
                        .get(i + 1)
                        .is_some_and(|d| d.is_alphabetic() || *d == '-')) =>
            {
                out.push(format!("{TOKEN}::Delim({c:?})"));
                i += 1;
                separated = true;
                continue;
            }
            '!' | ';' | '{' | '}' | '[' | ']' => return Err(format!("character '{c}'")),
            _ => {}
        }
        // Identifiant ou fonction.
        let start = i;
        while i < chars.len()
            && (chars[i].is_alphanumeric()
                || chars[i] == '-'
                || chars[i] == '_'
                || chars[i] == '\\')
        {
            i += 1;
        }
        if i == start {
            return Err(format!("character '{c}'"));
        }
        let name = chars[start..i].iter().collect::<String>();
        if chars.get(i) == Some(&'(') {
            let lower = name.to_ascii_lowercase();
            if lower == "url" {
                // Image SVG en URL `data:` : compilée en `SvgImage` statique.
                let (url, next) = images::url_argument(&chars, i + 1)?;
                out.push(images::token(&url)?);
                i = next;
                separated = false;
                continue;
            }
            if lower == "var" {
                // var(--nom [, repli])
                let mut j = i + 1;
                while j < chars.len() && chars[j].is_whitespace() {
                    j += 1;
                }
                let name_start = j;
                while j < chars.len()
                    && (chars[j].is_alphanumeric() || chars[j] == '-' || chars[j] == '_')
                {
                    j += 1;
                }
                let var = chars[name_start..j].iter().collect::<String>();
                if !var.starts_with("--") {
                    return Err("invalid var()".into());
                }
                out.push(format!("{TOKEN}::Var({var:?})"));
                depth += 1;
                i = j;
                separated = true;
                continue;
            }
            // Fonction de couleur constante : repliée en une couleur.
            if matches!(
                lower.as_str(),
                "rgb" | "rgba" | "hsl" | "hsla" | "oklch" | "oklab" | "color-mix"
            ) && let Some(len) = group_len(&chars[i + 1..].iter().collect::<String>())
            {
                let text = chars[start..i + 1 + len + 1].iter().collect::<String>();
                if !text.contains("var(")
                    && let Ok(Some(rgba)) = constant_color(&text)
                {
                    out.push(color_code(rgba));
                    i += 1 + len + 1;
                    separated = false;
                    continue;
                }
            }
            out.push(format!("{TOKEN}::Function({lower:?})"));
            depth += 1;
            i += 1;
            separated = true;
            continue;
        }
        if !name.starts_with("--")
            && let Ok(Some(rgba)) = constant_color(&name)
        {
            out.push(color_code(rgba));
        } else {
            out.push(format!("{TOKEN}::Ident({:?})", name.to_ascii_lowercase()));
        }
        separated = false;
    }
    if depth != 0 {
        return Err("unbalanced parentheses".into());
    }
    if out.len() > 512 {
        return Err("value longer than 512 tokens".into());
    }
    Ok(out)
}
fn color_code(c: [f32; 4]) -> String {
    let q = |v: f32| (v.clamp(0., 1.) * 255.).round() as u8;
    format!(
        "{TOKEN}::Color(__ui_root::ui::theme::Color::rgba({},{},{},{}))",
        q(c[0]),
        q(c[1]),
        q(c[2]),
        q(c[3])
    )
}
/// Code `&[Token]` d'une liste de jetons.
pub fn token_slice(tokens: &[String]) -> String {
    format!("&[{}]", tokens.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tokens() {
        let t = tokenize("0 0 0 .25rem rgba(var(--bs-x), .5)").unwrap();
        assert_eq!(t.len(), 10);
        assert!(t[3].contains("Rem"));
        assert!(t[4].contains("Function(\"rgba\")"));
        assert!(tokenize("calc(1px - -2px)").unwrap()[2].contains("Delim('-')"));
        assert!(tokenize("url(x.svg)").is_err());
        assert!(tokenize("rgba(0,0,0,.5)").unwrap()[0].contains("Color("));
    }
}
