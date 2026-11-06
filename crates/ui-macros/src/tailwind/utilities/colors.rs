//! Couleurs et fonds : `bg-*`, dégradés (`bg-linear-*`, `bg-radial-*`, `from-*`,
//! `via-*`, `to-*`), `accent-*`, `caret-*`, `fill-*`, `stroke-*`, `mask-*`.
use super::{
    Miss, R, Val,
    composed::GRADIENT_STOPS,
    decls, invalid,
    values::{color, fmt, unknown_color},
};
use crate::tailwind::{Config, candidate::Candidate};

/// Utilitaires fonctionnels de la famille ; `Miss::Unknown` pour une autre racine.
pub(super) fn functional(root: &str, v: &Val, c: &Candidate, cfg: &Config) -> R {
    let one = |prop: &str, value: String| Ok(decls(vec![(prop, value)]));
    match root {
        "bg" => match color(v, c, cfg) {
            Ok(col) => one("background-color", col),
            Err(Miss::Unknown) => match v {
                Val::Arb(value, _) if value.contains("gradient(") || value.starts_with("url(") => {
                    one("background-image", value.clone())
                }
                _ => Err(unknown_color(v, cfg)),
            },
            Err(e) => Err(e),
        },
        "bg-linear" | "bg-gradient" => {
            let position = match v {
                Val::Named(n) if root == "bg-linear" && n.parse::<f32>().is_ok() => {
                    format!(
                        "{}deg",
                        if c.negative {
                            fmt(-n.parse::<f32>().unwrap())
                        } else {
                            n.clone()
                        }
                    )
                }
                Val::Named(n) => match n.strip_prefix("to-") {
                    Some(dir) => {
                        let dir = match dir {
                            "t" => "top",
                            "tr" => "top right",
                            "r" => "right",
                            "br" => "bottom right",
                            "b" => "bottom",
                            "bl" => "bottom left",
                            "l" => "left",
                            "tl" => "top left",
                            _ => return Err(Miss::Unknown),
                        };
                        format!("to {dir}")
                    }
                    None => return Err(Miss::Unknown),
                },
                Val::Arb(v, _) | Val::Var(v, _) => v.clone(),
                Val::Bare => return Err(Miss::Unknown),
            };
            Ok(decls(vec![
                ("--tw-gradient-position", position),
                (
                    "background-image",
                    "linear-gradient(var(--tw-gradient-stops))".into(),
                ),
            ]))
        }
        "bg-radial" => {
            let position = match v {
                Val::Bare => String::new(),
                Val::Arb(v, _) | Val::Var(v, _) => v.clone(),
                _ => return Err(Miss::Unknown),
            };
            Ok(decls(vec![
                ("--tw-gradient-position", position),
                (
                    "background-image",
                    "radial-gradient(var(--tw-gradient-stops))".into(),
                ),
            ]))
        }
        "from" | "via" | "to" => {
            if let Val::Named(n) = v
                && let Some(p) = n.strip_suffix('%')
                && p.parse::<f32>().is_ok()
            {
                let var = match root {
                    "from" => "--tw-gradient-from-position",
                    "via" => "--tw-gradient-via-position",
                    _ => "--tw-gradient-to-position",
                };
                return one(var, n.clone());
            }
            let col = color(v, c, cfg).map_err(|e| match e {
                Miss::Unknown => unknown_color(v, cfg),
                e => e,
            })?;
            Ok(match root {
                "from" => decls(vec![
                    ("--tw-gradient-from", col),
                    ("--tw-gradient-stops", GRADIENT_STOPS.into()),
                ]),
                "to" => decls(vec![
                    ("--tw-gradient-to", col),
                    ("--tw-gradient-stops", GRADIENT_STOPS.into()),
                ]),
                _ => decls(vec![
                    ("--tw-gradient-via", col),
                    ("--tw-gradient-via-stops", "var(--tw-gradient-position), var(--tw-gradient-from) var(--tw-gradient-from-position), var(--tw-gradient-via) var(--tw-gradient-via-position), var(--tw-gradient-to) var(--tw-gradient-to-position)".into()),
                    ("--tw-gradient-stops", "var(--tw-gradient-via-stops)".into()),
                ]),
            })
        }
        "accent" => one(
            "accent-color",
            color(v, c, cfg).map_err(|e| match e {
                Miss::Unknown => unknown_color(v, cfg),
                e => e,
            })?,
        ),
        "caret" => one("caret-color", color(v, c, cfg)?),
        "fill" | "stroke" => {
            if let Ok(col) = color(v, c, cfg) {
                return one(root, col);
            }
            Err(Miss::Unknown)
        }
        "mask" => {
            invalid("mask-* utilities are not implemented; use [mask-image:linear-gradient(…)]")
        }
        _ => Err(Miss::Unknown),
    }
}
