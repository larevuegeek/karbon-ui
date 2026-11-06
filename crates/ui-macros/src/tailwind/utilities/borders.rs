//! Bordures et contours : `rounded-*`, `border-*`, `divide-*`, `outline-*`, `ring-*`,
//! `ring-offset-*`.
use super::{
    Miss, R, Val,
    composed::SHADOW,
    decls, invalid,
    spacing::sides,
    values::{color, looks_like_color, negate, px, rem_to_px, unknown_color},
};
use crate::tailwind::{Config, candidate::Candidate};
use crate::template::html;

/// Utilitaires fonctionnels de la famille ; `Miss::Unknown` pour une autre racine.
pub(super) fn functional(root: &str, v: &Val, c: &Candidate, cfg: &Config) -> R {
    let one = |prop: &str, value: String| Ok(decls(vec![(prop, value)]));
    match root {
        "rounded" => {
            if c.negative || c.modifier.is_some() {
                return Err(Miss::Unknown);
            }
            one("border-radius", radius(v, cfg)?)
        }
        _ if root.starts_with("rounded-") && corners(&root[8..]).is_some() => {
            if c.negative || c.modifier.is_some() {
                return Err(Miss::Unknown);
            }
            let value = radius(v, cfg)?;
            Ok(decls(
                corners(&root[8..])
                    .unwrap()
                    .iter()
                    .map(|corner| (format!("border-{corner}-radius"), value.clone()))
                    .collect(),
            ))
        }
        "border" | "border-x" | "border-y" | "border-t" | "border-r" | "border-b" | "border-l"
        | "border-s" | "border-e" => {
            let sides = sides(root.strip_prefix("border-").unwrap_or("")).unwrap();
            if let Ok(width) = border_width(v, c) {
                if root == "border" {
                    return Ok(decls(vec![
                        (
                            "border-style".to_owned(),
                            "var(--tw-border-style)".to_owned(),
                        ),
                        ("border-width".to_owned(), width),
                    ]));
                }
                let mut list = Vec::new();
                for side in sides {
                    list.push((
                        format!("border-{side}-style"),
                        "var(--tw-border-style)".to_owned(),
                    ));
                    list.push((format!("border-{side}-width"), width.clone()));
                }
                return Ok(decls(list));
            }
            let col = color(v, c, cfg).map_err(|e| match e {
                Miss::Unknown => unknown_color(v, cfg),
                e => e,
            })?;
            if root == "border" {
                return one("border-color", col);
            }
            Ok(decls(
                sides
                    .iter()
                    .map(|side| (format!("border-{side}-color"), col.clone()))
                    .collect(),
            ))
        }
        "divide-x" | "divide-y" => {
            let width = border_width(v, c)?;
            let (style, property) = if root == "divide-y" {
                ("border-bottom-style", "border-bottom-width")
            } else {
                ("border-right-style", "border-right-width")
            };
            Ok(decls(vec![
                ("--tw-child-not-last", String::new()),
                (style, "var(--tw-border-style)".into()),
                (property, width),
            ]))
        }
        "divide" => {
            let col = color(v, c, cfg)?;
            Ok(decls(vec![
                ("--tw-child-not-last", String::new()),
                ("border-color", col),
            ]))
        }
        "outline" => {
            if let Ok(width) = border_width(v, c) {
                return Ok(decls(vec![
                    ("outline-style", "var(--tw-outline-style)".into()),
                    ("outline-width", width),
                ]));
            }
            one("outline-color", color(v, c, cfg)?)
        }
        "outline-offset" => {
            let value = match v {
                Val::Named(n) if n.parse::<u32>().is_ok() => px(n.parse::<f32>().unwrap()),
                Val::Arb(v, _) | Val::Var(v, _) => v.clone(),
                _ => return Err(Miss::Unknown),
            };
            one(
                "outline-offset",
                if c.negative { negate(&value) } else { value },
            )
        }
        "ring" | "inset-ring" => {
            let (shadow, color_var, inset) = if root == "ring" {
                (
                    "--tw-ring-shadow",
                    "--tw-ring-color",
                    "var(--tw-ring-inset,) ",
                )
            } else {
                ("--tw-inset-ring-shadow", "--tw-inset-ring-color", "inset ")
            };
            if let Ok(width) = border_width(v, c) {
                let spread = if root == "ring" {
                    format!("calc({width} + var(--tw-ring-offset-width))")
                } else {
                    width
                };
                return Ok(decls(vec![
                    (
                        shadow,
                        format!("{inset}0 0 0 {spread} var({color_var}, currentcolor)"),
                    ),
                    ("box-shadow", SHADOW.into()),
                ]));
            }
            one(
                color_var,
                color(v, c, cfg).map_err(|e| match e {
                    Miss::Unknown => unknown_color(v, cfg),
                    e => e,
                })?,
            )
        }
        "ring-offset" => {
            if let Ok(width) = border_width(v, c) {
                return Ok(decls(vec![
                    ("--tw-ring-offset-width", width),
                    ("--tw-ring-offset-shadow", "var(--tw-ring-inset,) 0 0 0 var(--tw-ring-offset-width) var(--tw-ring-offset-color)".into()),
                ]));
            }
            one("--tw-ring-offset-color", color(v, c, cfg)?)
        }
        _ => Err(Miss::Unknown),
    }
}
/// Coins d'un utilitaire `rounded-*`.
fn corners(suffix: &str) -> Option<&'static [&'static str]> {
    Some(match suffix {
        "t" => &["top-left", "top-right"],
        "r" | "e" => &["top-right", "bottom-right"],
        "b" => &["bottom-right", "bottom-left"],
        "l" | "s" => &["top-left", "bottom-left"],
        "tl" | "ss" => &["top-left"],
        "tr" | "se" => &["top-right"],
        "br" | "ee" => &["bottom-right"],
        "bl" | "es" => &["bottom-left"],
        _ => return None,
    })
}
pub(super) fn radius(v: &Val, cfg: &Config) -> Result<String, Miss> {
    Ok(match v {
        Val::Bare => "4px".into(),
        Val::Named(n) if n == "none" => "0".into(),
        Val::Named(n) if n == "full" => "9999px".into(),
        Val::Named(n) => match cfg.lookup("radius", n) {
            Some(v) => rem_to_px(&v),
            None => {
                let keys = cfg.keys("radius");
                let mut keys = keys.iter().map(String::as_str).collect::<Vec<_>>();
                keys.extend(["none", "full"]);
                return invalid(match html::suggest(n, &keys) {
                    Some(s) => format!("Unknown radius '{n}'; did you mean '{s}'?"),
                    None => format!("Unknown radius '{n}'"),
                });
            }
        },
        Val::Arb(v, _) | Val::Var(v, _) => v.clone(),
    })
}
pub(super) fn border_width(v: &Val, c: &Candidate) -> Result<String, Miss> {
    if c.negative || c.modifier.is_some() {
        return Err(Miss::Unknown);
    }
    match v {
        Val::Bare => Ok("1px".into()),
        Val::Named(n) if n.parse::<u32>().is_ok() => Ok(px(n.parse::<f32>().unwrap())),
        Val::Arb(v, hint)
            if matches!(hint.as_deref(), Some("length" | "line-width"))
                || (hint.is_none() && !looks_like_color(v)) =>
        {
            Ok(v.clone())
        }
        Val::Var(v, Some(h)) if h == "length" || h == "line-width" => Ok(v.clone()),
        _ => Err(Miss::Unknown),
    }
}
