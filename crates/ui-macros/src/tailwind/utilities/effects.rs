//! Effets : `opacity-*`, ombres (`shadow-*`, `inset-shadow-*`, `drop-shadow-*`) et
//! filtres (`blur-*`, `brightness-*`…, `backdrop-*`).
use super::{
    Miss, R, Val,
    composed::{BACKDROP, FILTER, SHADOW, shadow_layers},
    decls, invalid,
    values::{color, fmt, looks_like_color, negate, number, percent_number, rem_to_px},
};
use crate::tailwind::{Config, candidate::Candidate};
use crate::template::html;

/// Utilitaires fonctionnels de la famille ; `Miss::Unknown` pour une autre racine.
pub(super) fn functional(root: &str, v: &Val, c: &Candidate, cfg: &Config) -> R {
    let one = |prop: &str, value: String| Ok(decls(vec![(prop, value)]));
    match root {
        "opacity" => {
            let value = match v {
                Val::Named(n) => match n.parse::<f32>() {
                    Ok(n) if (0. ..=100.).contains(&n) && c.modifier.is_none() => fmt(n / 100.),
                    _ => return Err(Miss::Unknown),
                },
                _ => number(v, c)?,
            };
            return one("opacity", value);
        }
        "shadow" | "inset-shadow" => {
            let (var, color_var, namespace) = if root == "shadow" {
                ("--tw-shadow", "--tw-shadow-color", "shadow")
            } else {
                (
                    "--tw-inset-shadow",
                    "--tw-inset-shadow-color",
                    "inset-shadow",
                )
            };
            let theme = match v {
                Val::Bare if root == "shadow" => cfg.get("--shadow").or_else(|| {
                    Some("0 1px 3px 0 rgb(0 0 0 / 0.1), 0 1px 2px -1px rgb(0 0 0 / 0.1)".into())
                }),
                Val::Named(n) => cfg.lookup(namespace, n),
                _ => None,
            };
            if let Some(theme) = theme {
                if c.modifier.is_some() {
                    return invalid(
                        "Shadow opacity modifiers are not implemented; use shadow-color/50",
                    );
                }
                return Ok(decls(vec![
                    (var, shadow_layers(&rem_to_px(&theme), color_var)),
                    ("box-shadow", SHADOW.into()),
                ]));
            }
            if let Val::Arb(value, hint) = v
                && (hint.as_deref() == Some("shadow")
                    || (hint.is_none() && !looks_like_color(value)))
            {
                return Ok(decls(vec![
                    (var, shadow_layers(value, color_var)),
                    ("box-shadow", SHADOW.into()),
                ]));
            }
            return match color(v, c, cfg) {
                Ok(col) => one(color_var, col),
                Err(Miss::Unknown) => {
                    if let Val::Named(n) = v {
                        let mut keys = cfg.keys(namespace);
                        keys.extend(cfg.keys("color"));
                        let keys = keys.iter().map(String::as_str).collect::<Vec<_>>();
                        if let Some(s) = html::suggest(n, &keys) {
                            return invalid(format!(
                                "Unknown shadow '{n}'; did you mean '{root}-{s}'?"
                            ));
                        }
                    }
                    Err(Miss::Unknown)
                }
                Err(e) => Err(e),
            };
        }
        "drop-shadow" => {
            let value = match v {
                Val::Bare => cfg.get("--drop-shadow").unwrap_or_else(|| {
                    "0 1px 2px rgb(0 0 0 / 0.1), 0 1px 1px rgb(0 0 0 / 0.06)".into()
                }),
                Val::Named(n) if n == "none" => {
                    return Ok(decls(vec![
                        ("--tw-drop-shadow", " ".into()),
                        ("filter", FILTER.into()),
                    ]));
                }
                Val::Named(n) => cfg.lookup("drop-shadow", n).ok_or(Miss::Unknown)?,
                Val::Arb(v, _) | Val::Var(v, _) => v.clone(),
            };
            let shadows = crate::css::parser::split(&value, ',')
                .unwrap_or_default()
                .iter()
                .map(|s| format!("drop-shadow({s})"))
                .collect::<Vec<_>>()
                .join(" ");
            return Ok(decls(vec![
                ("--tw-drop-shadow", shadows),
                ("filter", FILTER.into()),
            ]));
        }
        _ => {}
    }
    // Filtres et filtres d'arrière-plan.
    let (backdrop, name) = match root.strip_prefix("backdrop-") {
        Some(name) => (true, name),
        None => (false, root),
    };
    let filter = match name {
        "blur" => {
            let value = match v {
                Val::Bare => cfg.get("--blur").unwrap_or_else(|| "8px".into()),
                Val::Named(n) if n == "none" => String::new(),
                Val::Named(n) => cfg.lookup("blur", n).ok_or(Miss::Unknown)?,
                Val::Arb(v, _) | Val::Var(v, _) => v.clone(),
            };
            Some(if value.is_empty() {
                " ".into()
            } else {
                format!("blur({value})")
            })
        }
        "brightness" | "contrast" | "saturate" => {
            Some(format!("{name}({})", percent_number(v, c)?))
        }
        "grayscale" | "invert" | "sepia" => Some(format!(
            "{name}({})",
            match v {
                Val::Bare => "100%".into(),
                _ => percent_number(v, c)?,
            }
        )),
        "hue-rotate" => {
            let value = match v {
                Val::Named(n) if n.parse::<f32>().is_ok() => format!("{n}deg"),
                Val::Arb(v, _) | Val::Var(v, _) => v.clone(),
                _ => return Err(Miss::Unknown),
            };
            Some(format!(
                "hue-rotate({})",
                if c.negative { negate(&value) } else { value }
            ))
        }
        "opacity" if backdrop => Some(format!("opacity({})", percent_number(v, c)?)),
        _ => None,
    };
    if let Some(filter) = filter {
        let var = format!("--tw-{}{name}", if backdrop { "backdrop-" } else { "" });
        let property = if backdrop {
            "backdrop-filter"
        } else {
            "filter"
        };
        return Ok(decls(vec![
            (var, filter),
            (
                property.to_owned(),
                if backdrop { BACKDROP } else { FILTER }.into(),
            ),
        ]));
    }
    Err(Miss::Unknown)
}
