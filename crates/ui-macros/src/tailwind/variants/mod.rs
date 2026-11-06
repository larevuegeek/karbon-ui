//! Variantes Tailwind (`hover:`, `md:`, `@3xl:`, `group-hover:`, `dark:`…) traduites en
//! prédicats `StyleState`, requêtes `@media`/`@container` et pseudo-éléments.
mod names;
mod selectors;
use super::{Config, utilities::px_length};
use names::{pseudo_class, rank, unsupported};
use selectors::{arbitrary, attribute_state, breakpoint, custom, inner_state, nth};

const STATE: &str = "__ui_root::ui::styling::StyleState";

/// Condition d'application d'un utilitaire.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Condition {
    /// Prédicats (conjonction) en code `StyleState`.
    pub states: Vec<String>,
    /// Requêtes `@media` (`(width >= 40rem)`) ou `@container …`.
    pub media: Vec<String>,
    /// Pseudo-élément généré (`before`, `after`).
    pub pseudo: Option<&'static str>,
    /// Rangs des variantes dans l'ordre de Tailwind, décroissants (tri des règles).
    pub ranks: Vec<u32>,
    /// Règle posée par l'élément sur ses enfants, avec le prédicat que l'enfant doit
    /// vérifier : `Not(LastChild)` pour `space-y-*` et `divide-y`, celui du composé
    /// d'une variante `[&>div:last-child]`.
    pub child: Option<String>,
}
impl Condition {
    /// Clé d'égalité indépendante de l'ordre des variantes.
    pub fn key(&self) -> String {
        let mut states = self.states.clone();
        states.sort();
        let mut media = self.media.clone();
        media.sort();
        format!("{states:?}{media:?}{:?}{:?}", self.pseudo, self.child)
    }
}

/// Conditions d'une suite de variantes.
pub fn apply(variants: &[&str], cfg: &Config) -> Result<Condition, String> {
    let mut c = Condition::default();
    for variant in variants {
        variant_into(variant, cfg, &mut c)?;
    }
    c.ranks.sort_unstable_by(|a, b| b.cmp(a));
    Ok(c)
}
fn variant_into(variant: &str, cfg: &Config, c: &mut Condition) -> Result<(), String> {
    if let Some(selector) = cfg.variants.get(variant) {
        let (states, media) = custom(selector)?;
        c.states.extend(states);
        c.media.extend(media);
        c.ranks.push(if variant == "dark" {
            rank::DARK
        } else {
            rank::DARK + 1
        });
        return Ok(());
    }
    if let Some((state, r)) = pseudo_class(variant) {
        c.states.push(state);
        c.ranks.push(r);
        return Ok(());
    }
    match variant {
        "before" | "after" => {
            if c.pseudo.is_some() {
                return Err("Only one of before: and after: per utility".into());
            }
            c.pseudo = Some(if variant == "before" {
                "before"
            } else {
                "after"
            });
            c.ranks.push(if variant == "before" {
                rank::BEFORE
            } else {
                rank::AFTER
            });
            return Ok(());
        }
        "dark" => {
            // Stratégie par classe de Tailwind : `.dark` sur l'élément ou un ancêtre.
            let (states, _) = custom("&:where(.dark, .dark *)")?;
            c.states.extend(states);
            c.ranks.push(rank::DARK);
            return Ok(());
        }
        "placeholder" => {
            // `::placeholder` : couleur du texte indicatif du champ.
            if c.pseudo.is_some() || c.child.is_some() {
                return Err("placeholder: cannot be combined with another pseudo-element".into());
            }
            c.pseudo = Some("placeholder");
            c.ranks.push(rank::PLACEHOLDER);
            return Ok(());
        }
        "*" => {
            // `*:p-2` : règle posée sur tous les enfants (`:is(& > *)`).
            if c.child.is_some() || c.pseudo.is_some() {
                return Err(
                    "*: cannot be combined with another child or pseudo-element variant".into(),
                );
            }
            c.child = Some(format!("{STATE}::All(&[])"));
            c.ranks.push(rank::ARBITRARY);
            return Ok(());
        }
        "motion-safe" | "motion-reduce" => {
            c.media.push(format!(
                "(prefers-reduced-motion: {})",
                if variant == "motion-safe" {
                    "no-preference"
                } else {
                    "reduce"
                }
            ));
            c.ranks.push(rank::MOTION);
            return Ok(());
        }
        "portrait" | "landscape" => {
            c.media.push(format!("(orientation: {variant})"));
            c.ranks.push(rank::ORIENTATION);
            return Ok(());
        }
        _ => {}
    }
    if variant.starts_with('[') && variant.ends_with(']') {
        return arbitrary(&variant[1..variant.len() - 1], c);
    }
    if let Some(state) = attribute_state(variant) {
        c.states.push(state?);
        c.ranks.push(if variant.starts_with("aria-") {
            rank::ARIA
        } else {
            rank::DATA
        });
        return Ok(());
    }
    if let Some(rest) = variant.strip_prefix("in-") {
        // `in-data-[x]`, `in-focus` : un ancêtre quelconque (`:where(*:…) &`).
        let inner = inner_state(rest).map_err(|m| format!("{variant}: {m}"))?;
        c.states
            .push(format!("{STATE}::Ancestor(&{STATE}::Where(&{inner}))"));
        c.ranks.push(rank::IN);
        return Ok(());
    }
    if let Some(rest) = variant.strip_prefix('@') {
        // Requêtes de conteneur : `@md`, `@max-md`, `@[500px]`, `@min-[…]`, `@lg/nom`.
        let (rest, name) = match rest.rsplit_once('/') {
            Some((r, n))
                if !r.ends_with(']')
                    || n.chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') =>
            {
                (r, Some(n))
            }
            _ => (rest, None),
        };
        let (max, value) = if let Some(v) = rest.strip_prefix("max-") {
            (true, v)
        } else if let Some(v) = rest.strip_prefix("min-") {
            (false, v)
        } else {
            (false, rest)
        };
        let (length, px) = breakpoint(value, "container", cfg)?;
        let name = name.map(|n| format!("{n} ")).unwrap_or_default();
        let op = if max { "<" } else { ">=" };
        c.media
            .push(format!("@container {name}(width {op} {length})"));
        c.ranks.push(if max {
            rank::CONTAINER_MAX + 4095 - (px as u32).min(4095)
        } else {
            rank::CONTAINER_MIN + (px as u32).min(4095)
        });
        return Ok(());
    }
    if let Some(value) = variant.strip_prefix("max-") {
        let (length, px) = breakpoint(value, "breakpoint", cfg)?;
        c.media.push(format!("(width < {length})"));
        c.ranks.push(rank::MAX + 4095 - (px as u32).min(4095));
        return Ok(());
    }
    if let Some(value) = variant.strip_prefix("min-") {
        let (length, px) = breakpoint(value, "breakpoint", cfg)?;
        c.media.push(format!("(width >= {length})"));
        c.ranks.push(rank::MIN + (px as u32).min(4095));
        return Ok(());
    }
    if let Some(px) = cfg
        .lookup("breakpoint", variant)
        .and_then(|l| px_length(&l).map(|px| (l, px)))
    {
        c.media.push(format!("(width >= {})", px.0));
        c.ranks.push(rank::MIN + (px.1 as u32).min(4095));
        return Ok(());
    }
    for (prefix, relation, r) in [
        ("group-", "Ancestor", rank::GROUP),
        ("peer-", "AnyPreviousSibling", rank::PEER),
    ] {
        if let Some(rest) = variant.strip_prefix(prefix) {
            // `group-hover/nom` : groupe nommé `group/nom`.
            let (state, marker) = match rest.rsplit_once('/') {
                Some((s, n)) if !s.is_empty() && !n.contains(']') => {
                    (s, format!("{}/{n}", &prefix[..prefix.len() - 1]))
                }
                _ => (rest, prefix[..prefix.len() - 1].to_owned()),
            };
            let inner = inner_state(state).map_err(|m| format!("{variant}: {m}"))?;
            c.states.push(format!(
                "{STATE}::{relation}(&{STATE}::All(&[{STATE}::Where(&{STATE}::Class({marker:?})),{inner}]))"
            ));
            c.ranks.push(r);
            return Ok(());
        }
    }
    if let Some(rest) = variant.strip_prefix("not-") {
        let inner = inner_state(rest).map_err(|m| format!("{variant}: {m}"))?;
        c.states.push(format!("{STATE}::Not(&{inner})"));
        c.ranks.push(rank::NOT);
        return Ok(());
    }
    if let Some(rest) = variant.strip_prefix("nth-") {
        let (a, b) = if let Some(inner) = rest.strip_prefix('[').and_then(|v| v.strip_suffix(']')) {
            nth(&inner.replace('_', ""))?
        } else {
            (
                0,
                rest.parse::<i32>()
                    .map_err(|_| format!("Invalid nth-{rest}"))?,
            )
        };
        c.states.push(format!("{STATE}::NthChild({a},{b})"));
        c.ranks.push(rank::NTH);
        return Ok(());
    }
    Err(unsupported(variant))
}
