//! Résolution d'un groupe d'utilitaires : variables `--tw-*` substituées, `calc()`
//! évalués et propriétés composées (`transform`, `box-shadow`, `filter`, `transition`,
//! dégradés) traduites en propriétés du moteur ; `@keyframes` vérifiés.
use super::{Config, theme, utilities};
use std::collections::BTreeMap;
/// Valeurs initiales des variables `--tw-*` (`@property` de Tailwind) et défauts du
/// thème lus par les utilitaires.
pub(super) fn initial_vars(cfg: &Config) -> BTreeMap<String, String> {
    let mut vars = BTreeMap::new();
    for (name, value) in [
        ("--tw-border-style", "solid"),
        ("--tw-outline-style", "solid"),
        ("--tw-shadow", "0 0 #0000"),
        ("--tw-inset-shadow", "0 0 #0000"),
        ("--tw-ring-shadow", "0 0 #0000"),
        ("--tw-inset-ring-shadow", "0 0 #0000"),
        ("--tw-ring-offset-shadow", "0 0 #0000"),
        ("--tw-ring-offset-width", "0px"),
        ("--tw-ring-offset-color", "#fff"),
        ("--tw-translate-x", "0"),
        ("--tw-translate-y", "0"),
        ("--tw-scale-x", "1"),
        ("--tw-scale-y", "1"),
        ("--tw-gradient-from", "#0000"),
        ("--tw-gradient-via", "#0000"),
        ("--tw-gradient-to", "#0000"),
        ("--tw-gradient-from-position", "0%"),
        ("--tw-gradient-via-position", "50%"),
        ("--tw-gradient-to-position", "100%"),
        ("--tw-content", "\"\""),
    ] {
        vars.insert(name.to_owned(), value.to_owned());
    }
    for name in [
        "--default-transition-duration",
        "--default-transition-timing-function",
    ] {
        if let Some(value) = cfg.get(name) {
            vars.insert(name.to_owned(), value);
        }
    }
    vars
}
/// Remplace les `var(--tw-…)` (et défauts du thème) par leur valeur ; `None` si une
/// variable sans valeur ni repli rend la déclaration invalide. Les autres variables
/// restent pour l'exécution.
fn substitute(value: &str, vars: &BTreeMap<String, String>, depth: usize) -> Option<String> {
    if depth > 16 {
        return None;
    }
    let mut out = String::new();
    let mut rest = value;
    while let Some(at) = rest.find("var(--") {
        out += &rest[..at];
        let tail = &rest[at + 4..];
        let close = theme::matching(tail)?;
        let inner = &tail[..close];
        let (name, fallback) = match inner.split_once(',') {
            Some((n, f)) => (n.trim(), Some(f.trim())),
            None => (inner.trim(), None),
        };
        let ours = name.starts_with("--tw-") || name.starts_with("--default-");
        if !ours {
            // Variable de l'application : ses arguments peuvent contenir des --tw-*.
            match fallback {
                Some(f) => out += &format!("var({name}, {})", substitute(f, vars, depth + 1)?),
                None => out += &format!("var({name})"),
            }
        } else {
            match vars.get(name) {
                Some(v) => out += &substitute(v, vars, depth + 1)?,
                None => out += &substitute(fallback?, vars, depth + 1)?,
            }
        }
        rest = &tail[close + 1..];
    }
    out += rest;
    Some(out)
}
/// Évalue `calc(a op b)` (nombres, px, %, deg, ms) ; `None` sinon.
pub(super) fn calc(value: &str) -> Option<String> {
    let inner = value.trim().strip_prefix("calc(")?.strip_suffix(')')?;
    let words = inner.split_whitespace().collect::<Vec<_>>();
    let [a, op, b] = words.as_slice() else {
        return None;
    };
    // f64 et 9 décimales : calc(1 / 0.75) × 12 px donne 16 px, pas 15,9996.
    let unit = |w: &str| -> Option<(f64, &'static str)> {
        for u in ["px", "%", "deg", "ms", "rem", ""] {
            if let Some(n) = w.strip_suffix(u)
                && let Ok(n) = n.parse::<f64>()
            {
                return Some(if u == "rem" { (n * 16., "px") } else { (n, u) });
            }
        }
        None
    };
    let ((x, ux), (y, uy)) = (unit(a)?, unit(b)?);
    let (n, u) = match *op {
        "+" | "-" if ux == uy || x == 0. || y == 0. => {
            // Un zéro prend l'unité de l'autre terme : `calc(0px + 50%)` vaut 50 %.
            let u = if ux.is_empty() || (x == 0. && !uy.is_empty()) {
                uy
            } else {
                ux
            };
            (if *op == "+" { x + y } else { x - y }, u)
        }
        "*" if ux.is_empty() || uy.is_empty() => (x * y, if ux.is_empty() { uy } else { ux }),
        "/" if uy.is_empty() && y != 0. => (x / y, ux),
        "/" if ux == uy && y != 0. => (x / y, ""),
        _ => return None,
    };
    Some(if n == 0. && u == "px" {
        "0px".into()
    } else {
        format!("{}{u}", utilities::fmt_precise(n))
    })
}
/// Simplifie les `calc()` évaluables d'une valeur.
fn simplify(value: &str) -> String {
    let mut out = String::new();
    let mut rest = value;
    while let Some(at) = rest.find("calc(") {
        out += &rest[..at];
        let tail = &rest[at + 5..];
        let Some(close) = theme::matching(tail) else {
            break;
        };
        let inner = simplify(&tail[..close]);
        let whole = format!("calc({inner})");
        out += &calc(&whole).unwrap_or(whole);
        rest = &tail[close + 1..];
    }
    out + rest
}
fn words(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}
/// Liste à virgules sans éléments vides ni ombres transparentes nulles.
fn layers(value: &str, drop: &[&str]) -> Vec<String> {
    crate::css::parser::split(value, ',')
        .unwrap_or_default()
        .into_iter()
        .map(words)
        .filter(|l| !l.is_empty() && !drop.contains(&l.as_str()))
        .collect()
}
/// Propriétés animables par le moteur pour une liste `transition-property`.
fn transition_properties(list: &str) -> Vec<&'static str> {
    let mut out = Vec::new();
    for p in layers(list, &[]) {
        let mapped = match p.as_str() {
            "all" => "all",
            "opacity" => "opacity",
            "color" => "color",
            "background-color" => "background-color",
            "border-color" => "border-color",
            "width" => "width",
            "height" => "height",
            "transform" | "translate" | "scale" | "rotate" => "transform",
            _ => continue,
        };
        if !out.contains(&mapped) {
            out.push(mapped);
        }
    }
    out
}
/// Déclarations d'un groupe après résolution des variables et des propriétés
/// composées. `base` : déclarations brutes du groupe sans variante (repli des
/// propriétés composées), `None` pour ce groupe lui-même.
pub(super) fn resolve_group(
    raw: &[(String, String)],
    base: Option<&[(String, String)]>,
    vars: &BTreeMap<String, String>,
) -> Result<Vec<(String, String)>, String> {
    let get = |name: &str| -> Option<&String> {
        raw.iter()
            .rev()
            .find(|(n, _)| n == name)
            .or_else(|| base.and_then(|b| b.iter().rev().find(|(n, _)| n == name)))
            .map(|(_, v)| v)
    };
    let own = |name: &str| raw.iter().any(|(n, _)| n == name);
    let resolved = |value: &str| substitute(value, vars, 0).map(|v| simplify(&words(&v)));
    let conditional = base.is_some();
    let mut out: Vec<(String, String)> = Vec::new();
    let push = |out: &mut Vec<(String, String)>, name: &str, value: String| {
        out.retain(|(n, _)| n != name);
        out.push((name.to_owned(), value));
    };
    let text_color = get("color")
        .and_then(|c| resolved(c))
        .filter(|c| crate::css::svg_color(c).is_ok_and(|c| c.is_some()));
    // Transformations : translate, rotate puis scale, du groupe ou de la base.
    if ["translate", "scale", "rotate"].iter().any(|p| own(p)) {
        let mut transform = Vec::new();
        if let Some(t) = get("translate").and_then(|v| resolved(v))
            && t != "none"
        {
            let parts = t.split_whitespace().collect::<Vec<_>>();
            let (x, y) = (
                parts.first().copied().unwrap_or("0"),
                parts.get(1).copied().unwrap_or("0"),
            );
            let zero = |v: &str| v == "0" || v == "0px" || v == "0%";
            if !(zero(x) && zero(y)) {
                transform.push(format!("translate({x}, {y})"));
            }
        }
        if let Some(r) = get("rotate").and_then(|v| resolved(v))
            && r != "none"
            && r != "0deg"
        {
            transform.push(format!("rotate({r})"));
        }
        if let Some(s) = get("scale").and_then(|v| resolved(v))
            && s != "none"
        {
            let factor = |v: &str| -> String {
                match v.strip_suffix('%').and_then(|n| n.parse::<f32>().ok()) {
                    Some(n) => utilities::fmt(n / 100.),
                    None => v.to_owned(),
                }
            };
            let parts = s.split_whitespace().map(factor).collect::<Vec<_>>();
            let (x, y) = (
                parts.first().cloned().unwrap_or("1".into()),
                parts
                    .get(1)
                    .cloned()
                    .unwrap_or_else(|| parts.first().cloned().unwrap_or("1".into())),
            );
            if !(x == "1" && y == "1") {
                transform.push(if x == y {
                    format!("scale({x})")
                } else {
                    format!("scale({x}, {y})")
                });
            }
        }
        if !transform.is_empty() {
            push(&mut out, "transform", transform.join(" "));
        } else if conditional {
            push(&mut out, "transform", "none".into());
        }
    }
    // Transition : raccourci du moteur depuis les propriétés détaillées.
    if [
        "transition-property",
        "transition-duration",
        "transition-timing-function",
        "transition-delay",
    ]
    .iter()
    .any(|p| own(p))
    {
        let properties = get("transition-property")
            .and_then(|v| resolved(v))
            .unwrap_or_else(|| "all".into());
        if properties == "none" {
            push(&mut out, "transition", "none".into());
        } else {
            let duration = get("transition-duration")
                .and_then(|v| resolved(v))
                .unwrap_or_else(|| "0s".into());
            let easing = get("transition-timing-function")
                .and_then(|v| resolved(v))
                .unwrap_or_else(|| "ease".into());
            let delay = get("transition-delay").and_then(|v| resolved(v));
            let list = transition_properties(&properties)
                .iter()
                .map(|p| match &delay {
                    Some(d) => format!("{p} {duration} {easing} {d}"),
                    None => format!("{p} {duration} {easing}"),
                })
                .collect::<Vec<_>>();
            if !list.is_empty() {
                push(&mut out, "transition", list.join(", "));
            }
        }
    }
    for (name, value) in raw {
        if name.starts_with("--tw-")
            || [
                "translate",
                "scale",
                "rotate",
                "transition-property",
                "transition-duration",
                "transition-timing-function",
                "transition-delay",
            ]
            .contains(&name.as_str())
        {
            continue;
        }
        let Some(value) = resolved(value) else {
            continue;
        };
        match name.as_str() {
            "box-shadow" => {
                // Couches invisibles (décalages, flou et étalement nuls) retirées.
                let visible = |l: &String| {
                    crate::css::values_words(l)
                        .iter()
                        .any(|w| utilities::px_length(w).is_some_and(|n| n != 0.))
                };
                let list = layers(&value, &[])
                    .into_iter()
                    .filter(visible)
                    .map(|l| match &text_color {
                        Some(c) => l.replace("currentcolor", c).replace("currentColor", c),
                        None => l,
                    })
                    .collect::<Vec<_>>();
                if !list.is_empty() {
                    push(&mut out, "box-shadow", list.join(", "));
                } else if conditional {
                    push(&mut out, "box-shadow", "none".into());
                }
            }
            "filter" | "backdrop-filter" => {
                if value.is_empty() {
                    if conditional {
                        push(&mut out, name, "none".into());
                    }
                } else {
                    push(&mut out, name, value);
                }
            }
            "font-variant-numeric" => {
                if !value.is_empty() {
                    push(&mut out, name, value);
                }
            }
            "background-image" => {
                // Dégradés : arguments vides (position absente) ôtés.
                let cleaned = match value.split_once('(') {
                    Some((f, args)) if f.ends_with("gradient") => {
                        let args = args.strip_suffix(')').unwrap_or(args);
                        format!("{f}({})", layers(args, &[]).join(", "))
                    }
                    _ => value,
                };
                push(&mut out, name, cleaned);
            }
            "place-items" => {
                push(&mut out, "align-items", value.clone());
                push(&mut out, "justify-items", value);
            }
            "place-content" => {
                push(&mut out, "align-content", value.clone());
                push(&mut out, "justify-content", value);
            }
            "content" => push(&mut out, name, value),
            _ => push(&mut out, name, value),
        }
    }
    // Bordure de certains côtés seulement (`border-b border-white/6`) : la couleur va à
    // ces côtés. Les autres ont une largeur nulle en CSS (préflight de Tailwind), alors
    // que `border-color` seule donne une bordure de 1 px partout dans le moteur.
    let sides = ["top", "right", "bottom", "left"]
        .into_iter()
        .filter(|s| get(&format!("border-{s}-width")).is_some())
        .collect::<Vec<_>>();
    if get("border-width").is_none()
        && !sides.is_empty()
        && let Some(at) = out.iter().position(|(n, _)| n == "border-color")
    {
        let (_, color) = out.remove(at);
        for side in sides {
            push(&mut out, &format!("border-{side}-color"), color.clone());
        }
    }
    Ok(out)
}
/// `@keyframes` de Tailwind vérifiées pour le moteur, transmises telles quelles : le
/// moteur interpole chaque propriété entre ses propres étapes et reprend la valeur de
/// l'élément aux étapes `0%`/`100%` absentes, comme un navigateur.
pub(super) fn complete_keyframes(text: &str) -> Result<String, String> {
    let items = crate::css::parser::parse(text).map_err(|e| e.message)?;
    let Some(crate::css::parser::Item::Keyframes { frames, .. }) = items.into_iter().next() else {
        return Err("Invalid @keyframes".into());
    };
    for frame in frames {
        if let crate::css::parser::Item::Rule { selectors, .. } = frame {
            for selector in selectors {
                let valid = match selector.trim() {
                    "from" | "to" => true,
                    s => s
                        .strip_suffix('%')
                        .is_some_and(|n| n.trim().parse::<f32>().is_ok()),
                };
                if !valid {
                    return Err(format!("Invalid keyframe selector '{selector}'"));
                }
            }
        }
    }
    Ok(text.to_owned())
}
