//! Valeurs des utilitaires : nombres et longueurs (converties en px, 1rem = 16 px),
//! couleurs du thème avec modificateur d'opacité, durées, valeurs arbitraires `[…]`
//! et variables `(--var)`.
use super::{Miss, Val, invalid};
use crate::tailwind::{Config, candidate::Candidate};
use crate::template::html;
/// Nombre court (`8`, `0.375`, `-4`).
pub(in crate::tailwind) fn fmt(n: f32) -> String {
    let n = (n as f64 * 10000.).round() / 10000.;
    if n == 0. {
        return "0".into();
    }
    let s = format!("{n}");
    s.strip_suffix(".0").map(str::to_owned).unwrap_or(s)
}
/// Nombre d'un calcul (`calc()` évalué), à 9 décimales : un rapport comme
/// `1 / 0.75` garde sa précision une fois multiplié par la taille du texte.
pub(in crate::tailwind) fn fmt_precise(n: f64) -> String {
    let n = (n * 1e9).round() / 1e9;
    if n == 0. {
        return "0".into();
    }
    let s = format!("{n}");
    s.strip_suffix(".0").map(str::to_owned).unwrap_or(s)
}
pub(super) fn px(n: f32) -> String {
    if n == 0. {
        "0".into()
    } else {
        format!("{}px", fmt(n))
    }
}
/// Longueur en px d'une valeur `Npx`, `Nrem`/`Nem` (16 px) ou `0`.
pub(in crate::tailwind) fn px_length(value: &str) -> Option<f32> {
    let value = value.trim();
    if value == "0" {
        return Some(0.);
    }
    let (number, factor) = match value.strip_suffix("px") {
        Some(n) => (n, 1.),
        None => (value.strip_suffix("em")?.trim_end_matches('r'), 16.),
    };
    number
        .trim()
        .parse::<f32>()
        .ok()
        .filter(|n| n.is_finite())
        .map(|n| n * factor)
}
/// Valeur arbitraire : `_` devient une espace (sauf `\_` et dans `url()`), les `rem`
/// simples deviennent des px.
pub(super) fn decode(value: &str) -> String {
    let mut out = String::new();
    let mut chars = value.chars().peekable();
    let mut url = 0usize;
    while let Some(c) = chars.next() {
        if c == '\\' && chars.peek() == Some(&'_') {
            out.push('_');
            chars.next();
            continue;
        }
        if out.ends_with("url") && c == '(' {
            url += 1;
        } else if c == ')' && url > 0 {
            url -= 1;
        }
        out.push(if c == '_' && url == 0 { ' ' } else { c });
    }
    rem_to_px(&math_spaces(&out))
}
/// Convertit les longueurs `Nrem` d'une valeur en px, y compris dans les fonctions.
pub(super) fn rem_to_px(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        let boundary = start == 0
            || !(bytes[start - 1].is_ascii_alphanumeric()
                || bytes[start - 1] == b'-' && start >= 2 && bytes[start - 2] == b'-');
        if boundary && (bytes[i].is_ascii_digit() || bytes[i] == b'.' || bytes[i] == b'-') {
            let mut j = i + usize::from(bytes[i] == b'-');
            while j < bytes.len() && (bytes[j].is_ascii_digit() || bytes[j] == b'.') {
                j += 1;
            }
            if value[j..].starts_with("rem")
                && !value[j + 3..].starts_with(|c: char| c.is_ascii_alphanumeric())
                && let Ok(n) = value[start..j].parse::<f32>()
            {
                out += &px(n * 16.);
                i = j + 3;
                continue;
            }
            out += &value[start..j.max(start + 1)];
            i = j.max(start + 1);
            continue;
        }
        let c = value[i..].chars().next().unwrap();
        out.push(c);
        i += c.len_utf8();
    }
    out
}
/// [`rem_to_px`] pour les autres modules.
#[cfg(test)]
pub(in crate::tailwind) fn rem(value: &str) -> String {
    rem_to_px(value)
}
/// [`math_spaces`] pour les autres modules.
#[cfg(test)]
pub(in crate::tailwind) fn spaced(value: &str) -> String {
    math_spaces(value)
}
/// Espaces autour des opérateurs dans `calc()`, `min()`, `max()` et `clamp()`, comme
/// Tailwind (`calc(100dvh-330px)` → `calc(100dvh - 330px)`).
fn math_spaces(value: &str) -> String {
    let chars = value.chars().collect::<Vec<_>>();
    let mut out = String::new();
    let mut math = Vec::new();
    for (i, &c) in chars.iter().enumerate() {
        match c {
            '(' => {
                let head = out
                    .rsplit(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
                    .next()
                    .unwrap_or("");
                math.push(
                    ["calc", "min", "max", "clamp"].contains(&head)
                        || math.last() == Some(&true) && head.is_empty(),
                );
                out.push(c);
            }
            ')' => {
                math.pop();
                out.push(c);
            }
            '+' | '*' | '/' | '-' if math.last() == Some(&true) => {
                let prev = out.chars().last().unwrap_or(' ');
                let next = chars.get(i + 1).copied().unwrap_or(' ');
                let operand_before = prev.is_ascii_alphanumeric() || prev == '%' || prev == ')';
                let operand_after =
                    next.is_ascii_digit() || next == '(' || next == '.' || next == 'v' && c != '-';
                if c == '-'
                    && !(operand_before && (next.is_ascii_digit() || next == '(' || next == '.'))
                {
                    out.push(c);
                } else if operand_before && (operand_after || c != '-') {
                    out += &format!(" {c} ");
                } else {
                    out.push(c);
                }
            }
            _ => out.push(c),
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}
/// Indications de type de Tailwind (`[length:…]`, `(color:--x)`).
const HINTS: &[&str] = &[
    "length",
    "color",
    "number",
    "percentage",
    "url",
    "image",
    "family-name",
    "absolute-size",
    "relative-size",
    "position",
    "bg-size",
    "line-width",
    "integer",
    "ratio",
    "angle",
    "any",
    "vector",
    "shadow",
];
pub(super) fn value(text: &str) -> Val {
    if let Some(inner) = text.strip_prefix('[').and_then(|t| t.strip_suffix(']')) {
        if let Some((hint, rest)) = inner.split_once(':')
            && HINTS.contains(&hint)
        {
            return Val::Arb(decode(rest), Some(hint.to_owned()));
        }
        return Val::Arb(decode(inner), None);
    }
    if let Some(inner) = text.strip_prefix('(').and_then(|t| t.strip_suffix(')')) {
        let (hint, name) = match inner.split_once(':') {
            Some((h, n)) if HINTS.contains(&h) => (Some(h.to_owned()), n),
            _ => (None, inner),
        };
        return Val::Var(format!("var({})", decode(name)), hint);
    }
    Val::Named(text.to_owned())
}
pub(super) fn negate(value: &str) -> String {
    if let Some(n) = px_length(value) {
        return px(-n);
    }
    if let Some(n) = value.strip_suffix('%').and_then(|n| n.parse::<f32>().ok()) {
        return format!("{}%", fmt(-n));
    }
    if let Some(n) = value
        .strip_suffix("deg")
        .and_then(|n| n.parse::<f32>().ok())
    {
        return format!("{}deg", fmt(-n));
    }
    if let Ok(n) = value.parse::<f32>() {
        return fmt(-n);
    }
    format!("calc({value} * -1)")
}
pub(super) fn looks_like_color(value: &str) -> bool {
    value.starts_with("var(")
        || value.starts_with("color-mix(")
        || crate::css::svg_color(value).is_ok()
}
/// Options de lecture d'une longueur.
#[derive(Clone, Copy, Default)]
pub(super) struct Len {
    /// Fractions `1/2` → 50 %.
    pub(super) fraction: bool,
    /// Clés `--container-*` (`max-w-md`).
    pub(super) container: bool,
    /// `full`, `auto`, `screen`… (mot-clé → valeur).
    pub(super) keywords: &'static [(&'static str, &'static str)],
    /// Valeur négative permise.
    pub(super) negative: bool,
}
pub(super) const SIZE_W: &[(&str, &str)] = &[
    ("auto", "auto"),
    ("full", "100%"),
    ("screen", "100vw"),
    ("dvw", "100vw"),
    ("svw", "100vw"),
    ("lvw", "100vw"),
    ("min", "min-content"),
    ("max", "max-content"),
    ("fit", "fit-content"),
    ("px", "1px"),
];
pub(super) const SIZE_H: &[(&str, &str)] = &[
    ("auto", "auto"),
    ("full", "100%"),
    ("screen", "100vh"),
    ("dvh", "100vh"),
    ("svh", "100vh"),
    ("lvh", "100vh"),
    ("min", "min-content"),
    ("max", "max-content"),
    ("fit", "fit-content"),
    ("px", "1px"),
];
pub(super) const SIZE_BOTH: &[(&str, &str)] = &[
    ("auto", "auto"),
    ("full", "100%"),
    ("min", "min-content"),
    ("max", "max-content"),
    ("fit", "fit-content"),
    ("px", "1px"),
];
pub(super) const INSET: &[(&str, &str)] = &[("auto", "auto"), ("full", "100%"), ("px", "1px")];
pub(super) const SPACE: &[(&str, &str)] = &[("px", "1px")];
pub(super) const MARGIN: &[(&str, &str)] = &[("auto", "auto"), ("px", "1px")];
/// Longueur d'un utilitaire d'espacement ou de taille.
pub(super) fn length(v: &Val, c: &Candidate, cfg: &Config, opts: Len) -> Result<String, Miss> {
    if c.negative && !opts.negative {
        return Err(Miss::Unknown);
    }
    let value = match v {
        Val::Named(name) => {
            let name = name.as_str();
            if let Some(modifier) = c.modifier {
                // Fraction : `w-1/2`, `translate-x-1/3`.
                if !opts.fraction {
                    return invalid(format!("'{name}/{modifier}' is not a valid value"));
                }
                let (a, b) = (name.parse::<f32>(), modifier.parse::<f32>());
                match (a, b) {
                    (Ok(a), Ok(b)) if b > 0. => format!("{}%", fmt(a / b * 100.)),
                    _ => return invalid(format!("Invalid fraction '{name}/{modifier}'")),
                }
            } else if let Some((_, k)) = opts.keywords.iter().find(|(k, _)| *k == name) {
                (*k).to_owned()
            } else if let Ok(n) = name.parse::<f32>() {
                if n < 0. || (n * 4.).fract() != 0. {
                    return invalid(format!(
                        "Spacing '{name}' must be a positive multiple of 0.25"
                    ));
                }
                let step = cfg
                    .spacing()
                    .ok_or_else(|| Miss::Invalid("The theme has no --spacing".into()))?;
                px(n * step)
            } else if let Some(v) = cfg.lookup("spacing", name) {
                rem_to_px(&v)
            } else if opts.container
                && let Some(v) = cfg.lookup("container", name)
            {
                rem_to_px(&v)
            } else {
                let mut keys = opts.keywords.iter().map(|(k, _)| *k).collect::<Vec<_>>();
                let containers = cfg.keys("container");
                if opts.container {
                    keys.extend(containers.iter().map(String::as_str));
                }
                return invalid(match html::suggest(name, &keys) {
                    Some(s) => format!("Unknown size '{name}'; did you mean '{s}'?"),
                    None => format!(
                        "Unknown size '{name}': use a spacing multiple (4, 2.5), a fraction (1/2), a keyword or [value]"
                    ),
                });
            }
        }
        Val::Arb(value, hint) => {
            if c.modifier.is_some() {
                return Err(Miss::Unknown);
            }
            if hint
                .as_deref()
                .is_some_and(|h| h != "length" && h != "percentage")
            {
                return Err(Miss::Unknown);
            }
            if hint.is_none() && looks_like_color(value) && !value.starts_with("var(") {
                return Err(Miss::Unknown);
            }
            value.clone()
        }
        Val::Var(value, hint) => {
            if hint
                .as_deref()
                .is_some_and(|h| h != "length" && h != "percentage")
            {
                return Err(Miss::Unknown);
            }
            value.clone()
        }
        Val::Bare => return Err(Miss::Unknown),
    };
    Ok(if c.negative { negate(&value) } else { value })
}
/// Couleur, avec le modificateur d'opacité (`/5`, `/[0.37]`, `/(--a)`).
pub(super) fn color(v: &Val, c: &Candidate, cfg: &Config) -> Result<String, Miss> {
    if c.negative {
        return Err(Miss::Unknown);
    }
    let base = match v {
        Val::Named(name) => match name.as_str() {
            "current" => "currentColor".to_owned(),
            "transparent" => "transparent".to_owned(),
            "inherit" => return invalid("inherit is not implemented by the engine"),
            _ => match cfg.lookup("color", name) {
                Some(color) => color,
                None => return Err(Miss::Unknown),
            },
        },
        Val::Arb(value, hint) => match hint.as_deref() {
            Some("color") => value.clone(),
            None if looks_like_color(value) => value.clone(),
            _ => return Err(Miss::Unknown),
        },
        Val::Var(value, hint) => match hint.as_deref() {
            None | Some("color") => value.clone(),
            _ => return Err(Miss::Unknown),
        },
        Val::Bare => return Err(Miss::Unknown),
    };
    let Some(modifier) = c.modifier else {
        return Ok(base);
    };
    let percent = if let Ok(n) = modifier.parse::<f32>() {
        if !(0. ..=100.).contains(&n) {
            return invalid(format!(
                "Opacity modifier '/{modifier}' must be within 0..=100"
            ));
        }
        format!("{}%", fmt(n))
    } else {
        match value(modifier) {
            Val::Arb(v, _) => match v.strip_suffix('%') {
                Some(_) => v,
                None => match v.parse::<f32>() {
                    Ok(n) => format!("{}%", fmt(n * 100.)),
                    Err(_) => return invalid(format!("Invalid opacity modifier '/{modifier}'")),
                },
            },
            Val::Var(v, _) => v,
            _ => return invalid(format!("Invalid opacity modifier '/{modifier}'")),
        }
    };
    if percent == "100%" {
        return Ok(base);
    }
    Ok(format!(
        "color-mix(in oklab, {base} {percent}, transparent)"
    ))
}
/// Message d'une couleur inconnue, avec la clé du thème la plus proche.
pub(super) fn unknown_color(v: &Val, cfg: &Config) -> Miss {
    let Val::Named(name) = v else {
        return Miss::Unknown;
    };
    let keys = cfg.keys("color");
    let mut keys = keys.iter().map(String::as_str).collect::<Vec<_>>();
    keys.extend(["current", "transparent"]);
    match html::suggest(name, &keys) {
        Some(s) => Miss::Invalid(format!("Unknown color '{name}'; did you mean '{s}'?")),
        None => Miss::Unknown,
    }
}
pub(super) fn number(v: &Val, c: &Candidate) -> Result<String, Miss> {
    if c.modifier.is_some() {
        return Err(Miss::Unknown);
    }
    let n = match v {
        Val::Named(n) if n.parse::<f32>().is_ok() => n.clone(),
        Val::Arb(n, None | Some(_)) => n.clone(),
        Val::Var(n, _) => n.clone(),
        _ => return Err(Miss::Unknown),
    };
    Ok(if c.negative { negate(&n) } else { n })
}
pub(super) fn integer(v: &Val, c: &Candidate) -> Result<String, Miss> {
    let n = number(v, c)?;
    if let Val::Named(name) = v
        && name.parse::<u32>().is_err()
    {
        return invalid(format!("'{name}' must be an integer"));
    }
    Ok(n)
}
pub(super) fn percent_number(v: &Val, c: &Candidate) -> Result<String, Miss> {
    match v {
        Val::Named(n) => {
            if c.modifier.is_some() {
                return Err(Miss::Unknown);
            }
            n.parse::<f32>().map_err(|_| Miss::Unknown)?;
            Ok(format!(
                "{}%",
                if c.negative {
                    fmt(-n.parse::<f32>().unwrap())
                } else {
                    n.clone()
                }
            ))
        }
        _ => number(v, c),
    }
}
pub(super) fn duration(v: &Val, c: &Candidate) -> Result<String, Miss> {
    match v {
        Val::Named(n) if n.parse::<u32>().is_ok() && c.modifier.is_none() => Ok(format!("{n}ms")),
        Val::Named(n) if n == "initial" => Ok("initial".into()),
        Val::Arb(v, _) | Val::Var(v, _) => Ok(v.clone()),
        _ => Err(Miss::Unknown),
    }
}
