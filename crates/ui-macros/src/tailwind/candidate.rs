//! Découpage d'une classe Tailwind : variantes, `!`, signe, utilitaire et modificateur.

/// Classe Tailwind découpée (`hover:-translate-y-1/2!`).
#[derive(Debug, PartialEq)]
pub struct Candidate<'a> {
    /// Variantes, de gauche à droite (`hover`, `md`, `group-hover/a`…).
    pub variants: Vec<&'a str>,
    /// `!` en préfixe ou en suffixe de l'utilitaire.
    pub important: bool,
    /// `-` en préfixe de l'utilitaire (valeur négative).
    pub negative: bool,
    /// Utilitaire sans signe ni modificateur (`translate-y-1`, `bg-white`, `[color:red]`).
    pub base: &'a str,
    /// Modificateur après le dernier `/` hors crochets (`5` de `bg-white/5`).
    pub modifier: Option<&'a str>,
}

/// Découpe `text` aux séparateurs `sep` hors de `[...]` et `(...)`.
pub fn split_top(text: &str, sep: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut start = 0;
    for (i, c) in text.char_indices() {
        match c {
            '[' | '(' => depth += 1,
            ']' | ')' => depth -= 1,
            c if c == sep && depth == 0 => {
                parts.push(&text[start..i]);
                start = i + c.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(&text[start..]);
    parts
}
/// Position du dernier `sep` hors crochets et parenthèses.
fn last_top(text: &str, sep: char) -> Option<usize> {
    let mut depth = 0i32;
    let mut found = None;
    for (i, c) in text.char_indices() {
        match c {
            '[' | '(' => depth += 1,
            ']' | ')' => depth -= 1,
            c if c == sep && depth == 0 => found = Some(i),
            _ => {}
        }
    }
    found
}

pub fn parse(token: &str) -> Result<Candidate<'_>, String> {
    if token.is_empty() {
        return Err("Empty class".into());
    }
    let mut depth = 0i32;
    for c in token.chars() {
        match c {
            '[' | '(' => depth += 1,
            ']' | ')' => {
                depth -= 1;
                if depth < 0 {
                    return Err("Unbalanced brackets".into());
                }
            }
            _ => {}
        }
    }
    if depth != 0 {
        return Err("Unbalanced brackets".into());
    }
    let mut parts = split_top(token, ':');
    let mut utility = parts.pop().unwrap_or_default();
    if parts.iter().any(|v| v.is_empty()) || utility.is_empty() {
        return Err("Empty variant or utility".into());
    }
    let mut important = false;
    if let Some(rest) = utility.strip_prefix('!') {
        utility = rest;
        important = true;
    } else if let Some(rest) = utility.strip_suffix('!') {
        utility = rest;
        important = true;
    }
    let mut negative = false;
    if let Some(rest) = utility.strip_prefix('-') {
        utility = rest;
        negative = true;
    }
    if utility.is_empty() {
        return Err("Empty utility".into());
    }
    let (base, modifier) = if utility.starts_with('[') && utility.ends_with(']') {
        // Propriété arbitraire `[prop:valeur]` : le `/` éventuel lui appartient.
        (utility, None)
    } else {
        match last_top(utility, '/') {
            Some(at) if at > 0 && at + 1 < utility.len() => {
                (&utility[..at], Some(&utility[at + 1..]))
            }
            Some(_) => return Err("Empty modifier".into()),
            None => (utility, None),
        }
    };
    Ok(Candidate {
        variants: parts,
        important,
        negative,
        base,
        modifier,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn candidates_split_variants_sign_and_modifier_outside_brackets() {
        let c = parse("dark:hover:-translate-y-1/2").unwrap();
        assert_eq!(c.variants, ["dark", "hover"]);
        assert!(c.negative);
        assert_eq!((c.base, c.modifier), ("translate-y-1", Some("2")));
        let c = parse("[&:hover+a]:bg-[url(a/b.png)]/50!").unwrap();
        assert_eq!(c.variants, ["[&:hover+a]"]);
        assert!(c.important);
        assert_eq!((c.base, c.modifier), ("bg-[url(a/b.png)]", Some("50")));
        let c = parse("[mask-type:luminance]").unwrap();
        assert_eq!((c.base, c.modifier), ("[mask-type:luminance]", None));
        assert!(parse("hover:").is_err());
        assert!(parse("bg-[red").is_err());
    }
}
