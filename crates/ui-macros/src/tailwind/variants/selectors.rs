//! Variantes paramétrées : points de rupture, attributs (`data-*`, `aria-*`), états
//! d'un autre élément (`group-*`, `peer-*`…), `nth-*`, `@custom-variant` et variantes
//! arbitraires `[…]`.
use super::{
    Condition, STATE,
    names::{ARIA, pseudo_class, rank},
};
use crate::tailwind::{Config, utilities::px_length};
use crate::template::html;
/// Largeur d'un point de rupture : clé du thème (`md`) ou valeur arbitraire (`[900px]`).
pub(super) fn breakpoint(
    value: &str,
    namespace: &str,
    cfg: &Config,
) -> Result<(String, f32), String> {
    let length = if let Some(inner) = value.strip_prefix('[').and_then(|v| v.strip_suffix(']')) {
        inner.to_owned()
    } else {
        cfg.lookup(namespace, value).ok_or_else(|| {
            let keys = cfg.keys(namespace);
            let keys = keys.iter().map(String::as_str).collect::<Vec<_>>();
            match html::suggest(value, &keys) {
                Some(s) => format!("Unknown {namespace} '{value}'; did you mean '{s}'?"),
                None => format!("Unknown {namespace} '{value}'"),
            }
        })?
    };
    let px = px_length(&length)
        .ok_or_else(|| format!("Breakpoint '{length}' must be a length in px, rem or em"))?;
    Ok((length, px))
}
/// Variante d'attribut `data-[state=open]`, `data-active`, `aria-[sort=ascending]`,
/// `aria-expanded` : prédicat sur les attributs écrits dans le template.
pub(super) fn attribute_state(value: &str) -> Option<Result<String, String>> {
    let (prefix, rest) = value
        .strip_prefix("data-")
        .map(|r| ("data-", r))
        .or_else(|| value.strip_prefix("aria-").map(|r| ("aria-", r)))?;
    let selector = if let Some(inner) = rest.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
        format!("{prefix}{}", inner.replace('_', " "))
    } else if prefix == "aria-" {
        if !ARIA.contains(&rest) {
            return Some(Err(format!(
                "Unknown variant 'aria-{rest}:'; use aria-[{rest}=…] or one of {}",
                ARIA.join(", ")
            )));
        }
        format!("aria-{rest}=\"true\"")
    } else if rest.is_empty()
        || !rest
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Some(Err(format!("Invalid variant 'data-{rest}:'")));
    } else {
        format!("data-{rest}")
    };
    Some(crate::css::attribute_predicate(&selector))
}
/// Prédicat d'une variante qui s'applique à un autre élément (`group-*`, `peer-*`,
/// `not-*`, `in-*`) : pseudo-classe simple, attribut ou sélecteur arbitraire `[…]`.
pub(super) fn inner_state(value: &str) -> Result<String, String> {
    if let Some((state, _)) = pseudo_class(value) {
        return Ok(state);
    }
    if let Some(state) = attribute_state(value) {
        return state;
    }
    if let Some(inner) = value.strip_prefix('[').and_then(|v| v.strip_suffix(']')) {
        let inner = inner.replace('_', " ");
        let inner = inner.trim_start_matches('&');
        if let Some(class) = inner.strip_prefix('.')
            && class
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Ok(format!("{STATE}::Class({class:?})"));
        }
        if let Some(pseudo) = inner.strip_prefix(':')
            && let Some((state, _)) = pseudo_class(pseudo.trim_end_matches("-child"))
        {
            return Ok(state);
        }
        return Err(format!(
            "Arbitrary selector '[{inner}]' is not implemented here; use a pseudo-class or a .class"
        ));
    }
    Err(format!("Unknown state '{value}'"))
}
/// Variante définie par `@custom-variant` (ou `dark` par défaut) : sélecteur avec `&`.
pub(super) fn custom(selector: &str) -> Result<(Vec<String>, Vec<String>), String> {
    let selector = selector.trim();
    if let Some(query) = selector.strip_prefix("@media") {
        let query = query.trim();
        crate::css::media_condition(query).map_err(|m| format!("@custom-variant: {m}"))?;
        return Ok((Vec::new(), vec![query.to_owned()]));
    }
    // Un élément du sélecteur : `.c` (l'élément porte c), `.c *` ou `.c &` (un ancêtre).
    let part = |text: &str| -> Result<String, String> {
        let text = text.trim();
        let words = text.split_whitespace().collect::<Vec<_>>();
        let class = |w: &str| {
            w.strip_prefix('.')
                .filter(|c| {
                    !c.is_empty()
                        && c.chars()
                            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
                })
                .map(|c| format!("{STATE}::Class({c:?})"))
                .ok_or_else(|| {
                    format!("@custom-variant: only class selectors are implemented, not '{text}'")
                })
        };
        match words.as_slice() {
            [w] if w.starts_with("&.") => class(&w[1..]),
            [w] => class(w),
            [w, "*"] | [w, "&"] => Ok(format!("{STATE}::Ancestor(&{})", class(w)?)),
            _ => Err(format!(
                "@custom-variant: selector '{text}' is not implemented; use &, .c &, &.c or &:where(.c, .c *)"
            )),
        }
    };
    if selector == "&" {
        return Ok((Vec::new(), Vec::new()));
    }
    for (prefix, zero) in [("&:where(", true), ("&:is(", false)] {
        if let Some(list) = selector
            .strip_prefix(prefix)
            .and_then(|s| s.strip_suffix(')'))
        {
            let parts = list.split(',').map(part).collect::<Result<Vec<_>, _>>()?;
            let any = if parts.len() == 1 {
                parts[0].clone()
            } else {
                format!("{STATE}::Any(&[{}])", parts.join(","))
            };
            let state = if zero {
                format!("{STATE}::Where(&{any})")
            } else {
                any
            };
            return Ok((vec![state], Vec::new()));
        }
    }
    Ok((vec![part(selector)?], Vec::new()))
}
/// `an+b` d'un `nth-[…]`.
pub(super) fn nth(value: &str) -> Result<(i32, i32), String> {
    match value {
        "odd" => return Ok((2, 1)),
        "even" => return Ok((2, 0)),
        _ => {}
    }
    match value.split_once('n') {
        Some((a, b)) => {
            let a = match a {
                "" | "+" => 1,
                "-" => -1,
                _ => a
                    .parse()
                    .map_err(|_| format!("Invalid nth coefficient '{a}'"))?,
            };
            let b = if b.is_empty() {
                0
            } else {
                b.trim_start_matches('+')
                    .parse()
                    .map_err(|_| format!("Invalid nth offset '{b}'"))?
            };
            Ok((a, b))
        }
        None => Ok((
            0,
            value
                .parse()
                .map_err(|_| format!("Invalid nth '{value}'"))?,
        )),
    }
}
/// Variante arbitraire `[sélecteur avec &]` : `&` doit être le sujet (dernier composé).
pub(super) fn arbitrary(selector: &str, c: &mut Condition) -> Result<(), String> {
    let selector = selector.replace('_', " ");
    if selector.starts_with('@') {
        let query = selector
            .strip_prefix("@media")
            .map(str::trim)
            .ok_or_else(|| {
                format!("Arbitrary at-rule variant '[{selector}]' is not implemented")
            })?;
        crate::css::media_condition(query).map_err(|m| format!("[{selector}]: {m}"))?;
        c.media.push(query.to_owned());
        c.ranks.push(rank::ARBITRARY);
        return Ok(());
    }
    // `[&>div:last-child]` : règle posée sur les enfants qui vérifient le composé.
    if let Some(child) = selector
        .trim()
        .strip_prefix('&')
        .and_then(|r| r.trim_start().strip_prefix('>'))
    {
        if c.child.is_some() || c.pseudo.is_some() {
            return Err(format!(
                "Arbitrary variant '[{selector}]' cannot be combined with another child or pseudo-element variant"
            ));
        }
        let predicate = crate::css::compound_predicate(child)
            .map_err(|m| format!("Arbitrary variant '[{selector}]': {m}"))?;
        c.child = Some(predicate);
        c.ranks.push(rank::ARBITRARY);
        return Ok(());
    }
    let marker = "__tw_subject";
    let css = selector.replace('&', &format!(".{marker}"));
    let (predicate, pseudo) = crate::css::subject_predicate(&css, marker)
        .map_err(|m| format!("Arbitrary variant '[{selector}]': {m}"))?;
    if let Some(p) = predicate {
        c.states.push(p);
    }
    if let Some(p) = pseudo {
        c.pseudo = Some(p);
    }
    c.ranks.push(rank::ARBITRARY);
    Ok(())
}
