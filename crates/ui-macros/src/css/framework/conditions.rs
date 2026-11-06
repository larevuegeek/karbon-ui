//! Conditions des at-rules évaluées à la compilation : listes de media queries
//! (préférences et types connus, dimensions laissées à l'exécution) et `@supports`
//! (déclarations essayées contre le classement des valeurs, sélecteurs contre le
//! lecteur de sélecteurs).
use super::{selectors, values};
use std::collections::BTreeMap;

/// Résultat de l'évaluation d'une condition à la compilation.
pub(super) enum Condition {
    Static(bool),
    Runtime(String),
}
/// Évalue une liste de media queries : préférences et types connus à la compilation,
/// dimensions à l'exécution.
pub(super) fn media(prelude: &str) -> Result<Condition, String> {
    let mut runtime = Vec::new();
    for alternative in selectors::split_list(prelude) {
        let lower = alternative.to_ascii_lowercase();
        let mut words = lower.as_str();
        let mut invert = false;
        if let Some(rest) = words.strip_prefix("not ") {
            invert = true;
            words = rest.trim();
        } else if let Some(rest) = words.strip_prefix("only ") {
            words = rest.trim();
        }
        let mut result: Option<bool> = Some(true);
        let mut dynamic = Vec::new();
        for part in split_and(words) {
            let part = part.trim();
            match part {
                "all" | "screen" => {}
                "print" | "speech" | "tty" | "tv" | "projection" | "handheld" | "braille"
                | "embossed" | "aural" => result = Some(false),
                p if p.starts_with('(') => match feature(p) {
                    Some(true) => {}
                    Some(false) => result = Some(false),
                    None => dynamic.push(p.to_owned()),
                },
                _ => return Err(format!("query '{part}'")),
            }
        }
        if result == Some(false) {
            if invert {
                return Ok(Condition::Static(true));
            }
            continue;
        }
        if dynamic.is_empty() {
            if invert {
                continue;
            }
            return Ok(Condition::Static(true));
        }
        let query = format!(
            "{}{}",
            if invert { "not all and " } else { "" },
            dynamic.join(" and ")
        );
        runtime.push(crate::css::media::condition(&query).map_err(|_| {
            let f = dynamic.join(" and ");
            let name = f
                .trim_start_matches('(')
                .split([':', ')', '<', '>', '='])
                .next()
                .unwrap_or("")
                .trim()
                .to_owned();
            format!("feature ({name})")
        })?);
    }
    Ok(match runtime.len() {
        0 => Condition::Static(false),
        1 => Condition::Runtime(runtime.remove(0)),
        _ => Condition::Runtime(format!(
            "__ui_root::ui::styling::StyleState::Any(&[{}])",
            runtime.join(",")
        )),
    })
}
fn split_and(text: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0;
    let mut current = String::new();
    let chars = text.chars().collect::<Vec<_>>();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ => {}
        }
        if depth == 0
            && text[text.char_indices().nth(i).map_or(0, |(b, _)| b)..].starts_with(" and ")
        {
            parts.push(std::mem::take(&mut current));
            i += 5;
            continue;
        }
        current.push(c);
        i += 1;
    }
    parts.push(current);
    parts
}
/// Caractéristique connue à la compilation (`None` : dimensions, évaluées au runtime).
fn feature(part: &str) -> Option<bool> {
    let inner = part
        .trim_start_matches('(')
        .trim_end_matches(')')
        .replace(' ', "");
    let (name, value) = inner.split_once(':').unwrap_or((&inner, ""));
    Some(match (name, value) {
        ("prefers-reduced-motion", "reduce") => false,
        ("prefers-reduced-motion", "no-preference") => true,
        ("prefers-color-scheme", "dark") => false,
        ("prefers-color-scheme", "light") => true,
        ("prefers-contrast", "more" | "less" | "custom") => false,
        ("prefers-contrast", "no-preference") => true,
        ("forced-colors", "active") => false,
        ("forced-colors", "none") => true,
        ("hover" | "any-hover", "hover") => true,
        ("hover" | "any-hover", "none") => false,
        ("pointer" | "any-pointer", "fine") => true,
        ("pointer" | "any-pointer", "coarse" | "none") => false,
        ("prefers-reduced-transparency", "reduce") => false,
        ("inverted-colors", "inverted") => false,
        ("scripting", "enabled") => true,
        ("color", "") => true,
        ("display-mode", "browser") => true,
        ("display-mode", _) => false,
        (n, _)
            if n.contains("resolution")
                || n.contains("device-pixel-ratio")
                || n.starts_with("-webkit-") =>
        {
            false
        }
        _ => return None,
    })
}
/// `@supports` : déclarations essayées contre le compilateur, sélecteurs contre le
/// lecteur de sélecteurs.
pub(super) fn supports(condition: &str) -> bool {
    fn eval(text: &str) -> bool {
        let text = text.trim();
        if let Some(rest) = text
            .strip_prefix("not ")
            .or_else(|| text.strip_prefix("not("))
        {
            let rest = if text.starts_with("not(") {
                format!("({rest}")
            } else {
                rest.to_owned()
            };
            return !eval(&rest);
        }
        // Alternatives et conjonctions au niveau 0.
        let groups = top_level(text);
        if groups.len() > 1 {
            let mut result = eval(&groups[0]);
            let mut i = 1;
            while i + 1 < groups.len() {
                let value = eval(&groups[i + 1]);
                match groups[i].as_str() {
                    "and" => result = result && value,
                    "or" => result = result || value,
                    _ => return false,
                }
                i += 2;
            }
            return result;
        }
        if let Some(inner) = text
            .strip_prefix("selector(")
            .and_then(|s| s.strip_suffix(')'))
        {
            return selectors::parse(inner).is_ok();
        }
        let Some(inner) = text.strip_prefix('(').and_then(|s| s.strip_suffix(')')) else {
            return false;
        };
        let inner = inner.trim();
        if inner.starts_with('(') || inner.starts_with("not") || inner.starts_with("selector(") {
            return eval(inner);
        }
        match inner.split_once(':') {
            Some((name, value)) => {
                let name = name.trim().to_ascii_lowercase();
                !matches!(
                    values::classify(&name, value.trim(), &BTreeMap::new()),
                    values::Kind::Skip(_)
                ) && dynamic_or_static_ok(&name, value.trim())
            }
            None => false,
        }
    }
    eval(condition)
}
/// Une valeur refusée par le compilateur commun mais acceptée par l'évaluation à
/// l'exécution ne prouve pas que la fonction testée (`color-mix(in lab, …)`) est rendue.
fn dynamic_or_static_ok(name: &str, value: &str) -> bool {
    match values::classify(name, value, &BTreeMap::new()) {
        values::Kind::Dynamic(..) => !value.contains("in lab") && !value.contains("from "),
        values::Kind::Skip(_) => false,
        _ => true,
    }
}
/// Termes de niveau 0 : groupes parenthésés et mots `and` / `or`.
fn top_level(text: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0;
    let mut current = String::new();
    for c in text.chars() {
        match c {
            '(' => {
                depth += 1;
                current.push(c);
            }
            ')' => {
                depth -= 1;
                current.push(c);
                if depth == 0 {
                    parts.push(std::mem::take(&mut current).trim().to_owned());
                }
            }
            c if c.is_whitespace() && depth == 0 => {
                if !current.trim().is_empty() {
                    parts.push(std::mem::take(&mut current).trim().to_owned());
                }
                current.clear();
            }
            _ => current.push(c),
        }
    }
    if !current.trim().is_empty() {
        parts.push(current.trim().to_owned());
    }
    // `selector(…)` et `not(…)` collés : rattacher le nom à son groupe.
    let mut merged: Vec<String> = Vec::new();
    for p in parts {
        if let Some(last) = merged.last_mut()
            && (last == "selector" || last == "not")
            && p.starts_with('(')
        {
            last.push_str(&p);
            continue;
        }
        merged.push(p);
    }
    merged
}
