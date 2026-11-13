pub(super) use super::super::{Config, candidate::Candidate};
pub(super) use super::statics::statics;
pub(super) use super::values::{decode, value};
use super::*;
pub(super) use crate::template::html;

/// Déclarations d'un utilitaire.
#[derive(Default, Debug)]
pub struct Out {
    pub decls: Vec<(String, String)>,
    /// `@keyframes` nécessaires (texte CSS).
    pub keyframes: Vec<String>,
}
/// Échec d'une interprétation : racine inconnue, ou valeur refusée avec un message.
pub enum Miss {
    Unknown,
    Invalid(String),
}
pub(super) type R = Result<Out, Miss>;

/// Valeur d'un utilitaire fonctionnel.
#[derive(Clone, Debug)]
pub(super) enum Val {
    /// Clé nommée (`4`, `white`, `lg`, `1/2` pour les fractions).
    Named(String),
    /// `[valeur]`, avec l'indication de type éventuelle (`[length:…]`).
    Arb(String, Option<String>),
    /// `(--var)`, devenue `var(--var)`, avec l'indication éventuelle.
    Var(String, Option<String>),
    /// Racine seule (`border`, `rounded`, `shadow`).
    Bare,
}

pub(super) fn out(decls: &[(&str, &str)]) -> Out {
    Out {
        decls: decls
            .iter()
            .map(|(n, v)| ((*n).to_owned(), (*v).to_owned()))
            .collect(),
        keyframes: Vec::new(),
    }
}
pub(super) fn decls<N: Into<String>>(list: Vec<(N, String)>) -> Out {
    Out {
        decls: list.into_iter().map(|(n, v)| (n.into(), v)).collect(),
        keyframes: Vec::new(),
    }
}
pub(super) fn invalid<T>(message: impl Into<String>) -> Result<T, Miss> {
    Err(Miss::Invalid(message.into()))
}

/// Interprétation des utilitaires fonctionnels d'une famille de racines.
pub(super) type Family = fn(&str, &Val, &Candidate, &Config) -> R;
/// Familles d'utilitaires fonctionnels ; chacune ne reconnaît que ses racines et
/// renvoie `Miss::Unknown` pour les autres (aucune racine n'appartient à deux familles).
pub(super) const FAMILIES: [Family; 8] = [
    spacing::functional,
    transforms::functional,
    layout::functional,
    borders::functional,
    effects::functional,
    typography::functional,
    colors::functional,
    motion::functional,
];
/// Utilitaire fonctionnel `racine-valeur`.
pub(super) fn functional(root: &str, v: &Val, c: &Candidate, cfg: &Config) -> R {
    for family in FAMILIES {
        match family(root, v, c, cfg) {
            Err(Miss::Unknown) => {}
            result => return result,
        }
    }
    Err(Miss::Unknown)
}

/// Racines connues pour les suggestions.
pub(super) const ROOTS: &[&str] = &[
    "p",
    "px",
    "py",
    "pt",
    "pr",
    "pb",
    "pl",
    "m",
    "mx",
    "my",
    "mt",
    "mr",
    "mb",
    "ml",
    "gap",
    "gap-x",
    "gap-y",
    "space-x",
    "space-y",
    "inset",
    "top",
    "right",
    "bottom",
    "left",
    "w",
    "h",
    "size",
    "min-w",
    "max-w",
    "min-h",
    "max-h",
    "basis",
    "flex",
    "grow",
    "shrink",
    "order",
    "z",
    "grid-cols",
    "grid-rows",
    "col-span",
    "row-span",
    "col-start",
    "row-start",
    "aspect",
    "rounded",
    "opacity",
    "leading",
    "tracking",
    "line-clamp",
    "font",
    "text",
    "bg",
    "from",
    "via",
    "to",
    "border",
    "outline",
    "ring",
    "shadow",
    "blur",
    "brightness",
    "scale",
    "rotate",
    "translate-x",
    "translate-y",
    "origin",
    "duration",
    "ease",
    "delay",
    "animate",
    "divide",
];
pub(super) const STATICS: &[&str] = &[
    "block",
    "flex",
    "inline-flex",
    "grid",
    "inline-grid",
    "hidden",
    "static",
    "fixed",
    "absolute",
    "relative",
    "flex-row",
    "flex-col",
    "flex-wrap",
    "flex-nowrap",
    "flex-auto",
    "flex-none",
    "grow",
    "shrink",
    "items-start",
    "items-end",
    "items-center",
    "items-baseline",
    "items-stretch",
    "self-start",
    "self-end",
    "self-center",
    "self-stretch",
    "justify-start",
    "justify-end",
    "justify-center",
    "justify-between",
    "justify-self-center",
    "overflow-auto",
    "overflow-hidden",
    "overflow-y-auto",
    "overflow-x-auto",
    "truncate",
    "text-left",
    "text-center",
    "text-right",
    "uppercase",
    "lowercase",
    "capitalize",
    "italic",
    "underline",
    "whitespace-nowrap",
    "whitespace-normal",
    "tabular-nums",
    "sr-only",
    "object-cover",
    "object-contain",
    "aspect-square",
    "aspect-video",
    "outline-none",
    "transition",
    "transition-all",
    "transition-colors",
    "transition-opacity",
    "transition-transform",
    "rounded-full",
    "pointer-events-none",
    "cursor-pointer",
    "select-none",
    "group",
    "peer",
];

/// Déclarations de l'utilitaire de `c`.
pub fn compile(c: &Candidate, cfg: &Config) -> Result<Out, String> {
    let base = c.base;
    if let Some(inner) = base.strip_prefix('[').and_then(|b| b.strip_suffix(']')) {
        // Propriété arbitraire `[prop:valeur]`.
        let (property, value) = inner
            .split_once(':')
            .ok_or_else(|| format!("Arbitrary property '{base}' must be [property:value]"))?;
        let valid = property.starts_with("--")
            || property
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b == b'-');
        if !valid || property.is_empty() || value.is_empty() || c.negative {
            return Err(format!("Invalid arbitrary property '{base}'"));
        }
        return Ok(decls(vec![(property, decode(value))]));
    }
    if !c.negative && c.modifier.is_none() {
        if let Some(o) = statics(base) {
            return Ok(o);
        }
        if base == "group" || base == "peer" || base == "dark" {
            // Marqueurs : la classe portée par l'élément sert aux variantes
            // `group-*`, `peer-*` et `dark:`.
            return Ok(Out::default());
        }
    }
    if (base == "group" || base == "peer") && c.modifier.is_some() && !c.negative {
        // Groupe nommé `group/nom` : la classe seule sert de marqueur.
        return Ok(Out::default());
    }
    if base == "@container"
        && !c.negative
        && let Some(name) = c.modifier
    {
        return Ok(decls(vec![
            ("container-type", "inline-size".into()),
            ("container-name", name.to_owned()),
        ]));
    }
    // Racines possibles, de la plus longue à la plus courte (`border-t-white`).
    let stop = base.find(['[', '(']).unwrap_or(base.len());
    let mut errors = Vec::new();
    let mut splits = base[..stop]
        .match_indices('-')
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    splits.reverse();
    for at in splits {
        let (root, rest) = (&base[..at], &base[at + 1..]);
        if root.is_empty() || rest.is_empty() {
            continue;
        }
        match functional(root, &value(rest), c, cfg) {
            Ok(o) => return Ok(o),
            Err(Miss::Invalid(m)) => errors.push(m),
            Err(Miss::Unknown) => {}
        }
    }
    match functional(base, &Val::Bare, c, cfg) {
        Ok(o) => return Ok(o),
        Err(Miss::Invalid(m)) => errors.push(m),
        Err(Miss::Unknown) => {}
    }
    if let Some(m) = errors.into_iter().next() {
        return Err(m);
    }
    let shown = format!("{}{base}", if c.negative { "-" } else { "" });
    let mut names = STATICS.to_vec();
    if let Some(s) = html::suggest(base, &names) {
        return Err(format!("Unknown utility '{shown}'; did you mean '{s}'?"));
    }
    let root = &base[..base.rfind('-').unwrap_or(base.len())];
    names = ROOTS.to_vec();
    if let Some(s) = html::suggest(root, &names) {
        return Err(format!(
            "Unknown utility '{shown}'; did you mean '{s}-{}'?",
            base[root.len()..].trim_start_matches('-')
        ));
    }
    Err(format!(
        "Unknown utility '{shown}': not a stylesheet class nor a Tailwind utility"
    ))
}
