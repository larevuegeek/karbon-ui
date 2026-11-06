//! Lecture des sélecteurs : identifiants échappés, listes au niveau 0, composés,
//! combinateurs, attributs, pseudo-classes (`:is()`, `:where()`, `:not()`,
//! `:nth-*()`) et pseudo-éléments. Une erreur porte la catégorie du rapport.
use super::{AttrOp, AttrSel, Combinator, Complex, Compound, Pseudo, PseudoElement};

/// Décode un identifiant CSS à partir de `at` (échappements `\:`, `\31 `…) ; renvoie
/// le texte et la position suivante.
fn ident(chars: &[char], mut at: usize) -> (String, usize) {
    let mut out = String::new();
    while at < chars.len() {
        let c = chars[at];
        if c == '\\' {
            at += 1;
            let mut hex = String::new();
            while at < chars.len() && hex.len() < 6 && chars[at].is_ascii_hexdigit() {
                hex.push(chars[at]);
                at += 1;
            }
            if hex.is_empty() {
                if at < chars.len() {
                    out.push(chars[at]);
                    at += 1;
                }
            } else {
                out.push(
                    u32::from_str_radix(&hex, 16)
                        .ok()
                        .and_then(char::from_u32)
                        .unwrap_or('\u{fffd}'),
                );
                if at < chars.len() && chars[at].is_whitespace() {
                    at += 1;
                }
            }
            continue;
        }
        if c.is_alphanumeric() || c == '-' || c == '_' || !c.is_ascii() {
            out.push(c);
            at += 1;
        } else {
            break;
        }
    }
    (out, at)
}
/// Texte entre parenthèses équilibrées à partir de `at` (après `(`) ; renvoie le
/// contenu et la position après `)`.
fn group(chars: &[char], mut at: usize) -> Result<(String, usize), String> {
    let mut depth = 1;
    let mut out = String::new();
    let mut quote = None;
    while at < chars.len() {
        let c = chars[at];
        if c == '\\' && at + 1 < chars.len() {
            out.push(c);
            out.push(chars[at + 1]);
            at += 2;
            continue;
        }
        if let Some(q) = quote {
            if c == q {
                quote = None;
            }
        } else if c == '"' || c == '\'' {
            quote = Some(c);
        } else if c == '(' {
            depth += 1;
        } else if c == ')' {
            depth -= 1;
            if depth == 0 {
                return Ok((out, at + 1));
            }
        }
        out.push(c);
        at += 1;
    }
    Err("unbalanced parentheses in selector".into())
}
/// Découpe une liste de sélecteurs aux virgules de niveau 0.
pub fn split_list(text: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut current = String::new();
    let mut quote = None;
    let mut escape = false;
    for c in text.chars() {
        if escape {
            escape = false;
            current.push(c);
            continue;
        }
        if c == '\\' {
            escape = true;
            current.push(c);
            continue;
        }
        if let Some(q) = quote {
            if c == q {
                quote = None;
            }
            current.push(c);
            continue;
        }
        match c {
            '"' | '\'' => quote = Some(c),
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(current.trim().to_owned());
                current.clear();
                continue;
            }
            _ => {}
        }
        current.push(c);
    }
    if !current.trim().is_empty() || parts.is_empty() {
        parts.push(current.trim().to_owned());
    }
    parts
}
fn parse_list(text: &str) -> Result<Vec<Complex>, String> {
    split_list(text).iter().map(|s| parse(s)).collect()
}
fn nth(value: &str) -> Result<(i32, i32), String> {
    let v = value.replace(' ', "").to_ascii_lowercase();
    if v.contains("of") {
        return Err("nth-child(… of S) is not supported".into());
    }
    Ok(match v.as_str() {
        "odd" => (2, 1),
        "even" => (2, 0),
        _ => {
            if let Some((a, b)) = v.split_once('n') {
                let a = match a {
                    "" | "+" => 1,
                    "-" => -1,
                    _ => a.parse::<i32>().map_err(|_| "invalid an+b")?,
                };
                let b = if b.is_empty() {
                    0
                } else {
                    b.trim_start_matches('+')
                        .parse::<i32>()
                        .map_err(|_| "invalid an+b")?
                };
                (a, b)
            } else {
                (0, v.parse::<i32>().map_err(|_| "invalid an+b")?)
            }
        }
    })
}
/// Pseudo-classe simple vers sa variante `StyleState` (ou une catégorie de refus).
fn pseudo_class(name: &str) -> Result<Pseudo, String> {
    Ok(Pseudo::State(match name {
        "hover" => "Hover",
        "active" => "Active",
        "focus" => "Focus",
        "focus-visible" => "FocusVisible",
        "focus-within" => "FocusWithin",
        "disabled" => "Disabled",
        "enabled" => "Enabled",
        "checked" => "Checked",
        "indeterminate" => "Indeterminate",
        "read-only" => "ReadOnly",
        "read-write" => "ReadWrite",
        "placeholder-shown" => "PlaceholderShown",
        "first-child" => "FirstChild",
        "last-child" => "LastChild",
        "only-child" => "OnlyChild",
        "empty" => "Empty",
        "first-of-type" => "FirstOfType",
        "last-of-type" => "LastOfType",
        "only-of-type" => "OnlyOfType",
        "root" | "host" => return Ok(Pseudo::Root),
        // Pas de liens, d'ancres, de validation de formulaire ni de plein écran natifs.
        "link" | "visited" | "any-link" | "target" | "invalid" | "valid" | "user-invalid"
        | "user-valid" | "required" | "optional" | "in-range" | "out-of-range" | "autofill"
        | "default" | "fullscreen" | "modal" | "popover-open" => {
            return Ok(Pseudo::Never);
        }
        _ => return Err(format!("pseudo-class :{name}")),
    }))
}
/// Lit un sélecteur complexe ; l'erreur est une catégorie de refus.
pub fn parse(text: &str) -> Result<Complex, String> {
    let chars = text.trim().chars().collect::<Vec<_>>();
    if chars.is_empty() {
        return Err("empty selector".into());
    }
    let mut parts: Vec<(Combinator, Compound)> = Vec::new();
    let mut current = Compound::default();
    let mut started = false;
    let mut pending: Option<Combinator> = None;
    let mut element = None;
    let mut at = 0;
    let flush = |current: &mut Compound,
                 started: &mut bool,
                 pending: &mut Option<Combinator>,
                 parts: &mut Vec<(Combinator, Compound)>|
     -> Result<(), String> {
        if !*started {
            return Ok(());
        }
        let combinator = if parts.is_empty() {
            Combinator::Descendant
        } else {
            pending.take().unwrap_or(Combinator::Descendant)
        };
        parts.push((combinator, std::mem::take(current)));
        *started = false;
        Ok(())
    };
    while at < chars.len() {
        let c = chars[at];
        // Pièces de curseur : pseudo-classes permises après (`::-webkit-slider-thumb:active`),
        // appliquées à l'hôte.
        let part_state = matches!(
            element,
            Some(PseudoElement::SliderThumb | PseudoElement::SliderTrack)
        ) && c == ':'
            && chars.get(at + 1) != Some(&':');
        if element.is_some() && !c.is_whitespace() && !part_state {
            return Err("pseudo-element before the end of the selector".into());
        }
        match c {
            c if c.is_whitespace() || c == '>' || c == '+' || c == '~' => {
                flush(&mut current, &mut started, &mut pending, &mut parts)?;
                if !c.is_whitespace() {
                    if parts.is_empty() {
                        return Err("relative selector".into());
                    }
                    if matches!(pending, Some(p) if p != Combinator::Descendant) {
                        return Err("two consecutive combinators".into());
                    }
                    pending = Some(match c {
                        '>' => Combinator::Child,
                        '+' => Combinator::Next,
                        _ => Combinator::Subsequent,
                    });
                } else if pending.is_none() && !parts.is_empty() {
                    pending = Some(Combinator::Descendant);
                }
                at += 1;
            }
            '*' => {
                started = true;
                at += 1;
            }
            '&' => return Err("nesting selector".into()),
            '|' => return Err("namespace selector".into()),
            '.' => {
                let (name, next) = ident(&chars, at + 1);
                if name.is_empty() {
                    return Err("invalid class selector".into());
                }
                current.classes.push(name);
                started = true;
                at = next;
            }
            '#' => {
                let (name, next) = ident(&chars, at + 1);
                if name.is_empty() {
                    return Err("invalid id selector".into());
                }
                current.ids.push(name);
                started = true;
                at = next;
            }
            '[' => {
                let close = chars[at..]
                    .iter()
                    .position(|c| *c == ']')
                    .ok_or("unclosed attribute selector")?;
                let inner = chars[at + 1..at + close].iter().collect::<String>();
                current.attrs.push(attribute(&inner)?);
                started = true;
                at += close + 1;
            }
            ':' => {
                if chars.get(at + 1) == Some(&':') {
                    let (name, next) = ident(&chars, at + 2);
                    element = Some(match name.as_str() {
                        "before" => PseudoElement::Before,
                        "after" => PseudoElement::After,
                        "placeholder" => PseudoElement::Placeholder,
                        "-webkit-slider-thumb" => PseudoElement::SliderThumb,
                        "-webkit-slider-runnable-track" => PseudoElement::SliderTrack,
                        _ => return Err(format!("pseudo-element ::{name}")),
                    });
                    started = true;
                    at = next;
                    continue;
                }
                let (name, next) = ident(&chars, at + 1);
                let name = name.to_ascii_lowercase();
                at = next;
                if chars.get(at) == Some(&'(') {
                    let (arg, after) = group(&chars, at + 1)?;
                    at = after;
                    current.pseudos.push(match name.as_str() {
                        "not" => Pseudo::Not(parse_list(&arg)?),
                        "is" | "matches" | "-webkit-any" => Pseudo::Is(parse_list(&arg)?),
                        "where" => Pseudo::Where(parse_list(&arg)?),
                        "nth-child" | "nth-last-child" | "nth-of-type" | "nth-last-of-type" => {
                            let (a, b) = nth(&arg)?;
                            Pseudo::Nth {
                                variant: match name.as_str() {
                                    "nth-child" => "NthChild",
                                    "nth-last-child" => "NthLastChild",
                                    "nth-of-type" => "NthOfType",
                                    _ => "NthLastOfType",
                                },
                                a,
                                b,
                            }
                        }
                        "has" => return Err("pseudo-class :has()".into()),
                        _ => return Err(format!("pseudo-class :{name}()")),
                    });
                } else {
                    match name.as_str() {
                        // Syntaxe à un deux-points des pseudo-éléments historiques.
                        "before" | "after" => {
                            element = Some(if name == "before" {
                                PseudoElement::Before
                            } else {
                                PseudoElement::After
                            });
                        }
                        _ => current.pseudos.push(pseudo_class(&name)?),
                    }
                }
                started = true;
            }
            _ => {
                let (name, next) = ident(&chars, at);
                if name.is_empty() {
                    return Err(format!("unexpected '{c}' in selector"));
                }
                if started {
                    return Err("type selector after other selectors".into());
                }
                current.tag = Some(name.to_ascii_lowercase());
                started = true;
                at = next;
            }
        }
    }
    if matches!(pending, Some(p) if p != Combinator::Descendant) && !started {
        return Err("selector ends with a combinator".into());
    }
    flush(&mut current, &mut started, &mut pending, &mut parts)?;
    if parts.is_empty() {
        return Err("empty selector".into());
    }
    if parts.len() > 16 {
        return Err("selector chain longer than 16 compounds".into());
    }
    Ok(Complex { parts, element })
}
fn attribute(inner: &str) -> Result<AttrSel, String> {
    let inner = inner.trim();
    let chars = inner.chars().collect::<Vec<_>>();
    let (name, mut at) = ident(&chars, 0);
    if name.is_empty() {
        return Err("invalid attribute selector".into());
    }
    let name = name.to_ascii_lowercase();
    while at < chars.len() && chars[at].is_whitespace() {
        at += 1;
    }
    if at == chars.len() {
        return Ok(AttrSel {
            name,
            op: AttrOp::Exists,
            value: String::new(),
            insensitive: false,
        });
    }
    let op = match chars[at] {
        '=' => AttrOp::Equals,
        '~' => AttrOp::Includes,
        '|' => AttrOp::Dash,
        '^' => AttrOp::Prefix,
        '$' => AttrOp::Suffix,
        '*' => AttrOp::Substring,
        _ => return Err("invalid attribute operator".into()),
    };
    at += if op == AttrOp::Equals { 1 } else { 2 };
    let rest = chars[at..].iter().collect::<String>();
    let rest = rest.trim();
    let (value, flags) = if let Some(q) = rest.chars().next().filter(|c| *c == '"' || *c == '\'') {
        let end = rest[1..].find(q).ok_or("unclosed attribute value")? + 1;
        (rest[1..end].replace("\\\"", "\""), rest[end + 1..].trim())
    } else {
        let chars = rest.chars().collect::<Vec<_>>();
        let (v, next) = ident(&chars, 0);
        (
            v,
            rest[chars[..next].iter().map(|c| c.len_utf8()).sum()..].trim(),
        )
    };
    let insensitive = match flags {
        "" | "s" | "S" => false,
        "i" | "I" => true,
        _ => return Err("invalid attribute flag".into()),
    };
    Ok(AttrSel {
        name,
        op,
        value,
        insensitive,
    })
}

#[cfg(test)]
mod tests {
    use super::super::{Combinator, Pseudo, PseudoElement};
    use super::*;
    #[test]
    fn specificity_follows_selectors_4() {
        let s = |t: &str| parse(t).unwrap().specificity();
        assert_eq!(s("*"), (0, 0, 0));
        assert_eq!(s(".a.b:hover"), (0, 3, 0));
        assert_eq!(s("#x a[href]"), (1, 1, 1));
        assert_eq!(s(":where(.a .b) .c"), (0, 1, 0));
        assert_eq!(s(":is(#a, .b) p::before"), (1, 0, 2));
        assert_eq!(s(".btn:not(:disabled):not(.disabled)"), (0, 3, 0));
    }
    #[test]
    fn escapes_and_attributes() {
        let x = parse(".md\\:flex>[data-bs-theme=dark] .\\31 0").unwrap();
        assert_eq!(x.parts[0].1.classes, ["md:flex"]);
        assert_eq!(x.parts[1].0, Combinator::Child);
        assert_eq!(x.parts[1].1.attrs[0].value, "dark");
        assert_eq!(x.parts[2].1.classes, ["10"]);
        assert!(parse("a:has(b)").is_err());
        assert!(parse("::selection").is_err());
    }
    #[test]
    fn slider_parts_accept_states_on_the_host() {
        let x = parse(".form-range:focus::-webkit-slider-thumb").unwrap();
        assert_eq!(x.element, Some(PseudoElement::SliderThumb));
        let x = parse(".form-range::-webkit-slider-thumb:active").unwrap();
        assert_eq!(x.element, Some(PseudoElement::SliderThumb));
        assert_eq!(x.subject().pseudos, [Pseudo::State("Active")]);
        let x = parse(".form-range::-webkit-slider-runnable-track").unwrap();
        assert_eq!(x.element, Some(PseudoElement::SliderTrack));
        assert!(parse(".a::before:hover").is_err());
        assert!(parse(".form-range::-moz-range-thumb").is_err());
    }
}
