//! Texte : interlignage, espacement des lettres, `line-clamp-*`, `font-*`, `text-*`
//! (taille et couleur), décorations, retrait, `align-*`, `content-*`.
use super::{
    Miss, R, Val,
    borders::border_width,
    decls, invalid, out,
    values::{Len, SPACE, color, integer, length, looks_like_color, negate, px, rem_to_px, value},
};
use crate::tailwind::{Config, candidate::Candidate};
use crate::template::html;

/// Utilitaires fonctionnels de la famille ; `Miss::Unknown` pour une autre racine.
pub(super) fn functional(root: &str, v: &Val, c: &Candidate, cfg: &Config) -> R {
    let one = |prop: &str, value: String| Ok(decls(vec![(prop, value)]));
    let len = |opts: Len| length(v, c, cfg, opts);
    match root {
        "leading" => {
            let value = match v {
                Val::Named(n) if n == "none" => "1".into(),
                Val::Named(n) if c.modifier.is_none() && cfg.lookup("leading", n).is_some() => {
                    cfg.lookup("leading", n).unwrap()
                }
                _ => len(Len {
                    keywords: SPACE,
                    ..Default::default()
                })?,
            };
            Ok(decls(vec![
                ("--tw-leading", value.clone()),
                ("line-height", value),
            ]))
        }
        "tracking" => {
            let value = match v {
                Val::Named(n) => match cfg.lookup("tracking", n) {
                    Some(v) => v,
                    None => return Err(Miss::Unknown),
                },
                Val::Arb(v, _) | Val::Var(v, _) => v.clone(),
                Val::Bare => return Err(Miss::Unknown),
            };
            let value = if c.negative { negate(&value) } else { value };
            Ok(decls(vec![
                ("--tw-tracking", value.clone()),
                ("letter-spacing", value),
            ]))
        }
        "line-clamp" => {
            if let Val::Named(n) = v
                && n == "none"
            {
                return Ok(out(&[
                    ("overflow", "visible"),
                    ("display", "block"),
                    ("-webkit-line-clamp", "none"),
                ]));
            }
            let n = integer(v, c)?;
            Ok(decls(vec![
                ("overflow", "hidden".into()),
                ("display", "-webkit-box".into()),
                ("-webkit-box-orient", "vertical".into()),
                ("-webkit-line-clamp", n),
            ]))
        }
        "font" => match v {
            Val::Named(n) => {
                if let Some(weight) = cfg.lookup("font-weight", n) {
                    Ok(decls(vec![
                        ("--tw-font-weight", weight.clone()),
                        ("font-weight", weight),
                    ]))
                } else if let Some(family) = cfg.lookup("font", n) {
                    one("font-family", family)
                } else {
                    let mut keys = cfg.keys("font-weight");
                    keys.extend(cfg.keys("font"));
                    let keys = keys.iter().map(String::as_str).collect::<Vec<_>>();
                    match html::suggest(n, &keys) {
                        Some(s) => invalid(format!("Unknown font '{n}'; did you mean 'font-{s}'?")),
                        None => Err(Miss::Unknown),
                    }
                }
            }
            Val::Arb(value, hint) | Val::Var(value, hint) => {
                let numeric = value.parse::<f32>().is_ok() || hint.as_deref() == Some("number");
                if numeric {
                    Ok(decls(vec![
                        ("--tw-font-weight", value.clone()),
                        ("font-weight", value.clone()),
                    ]))
                } else {
                    one("font-family", value.clone())
                }
            }
            Val::Bare => Err(Miss::Unknown),
        },
        "text" => {
            // Taille du thème (avec sa hauteur de ligne), longueur, ou couleur.
            let size = match v {
                Val::Named(n) => cfg.lookup("text", n).map(|s| {
                    (
                        rem_to_px(&s),
                        cfg.lookup("text", &format!("{n}--line-height")),
                    )
                }),
                Val::Arb(value, hint)
                    if matches!(
                        hint.as_deref(),
                        Some("length" | "absolute-size" | "relative-size" | "percentage")
                    ) || (hint.is_none() && !looks_like_color(value)) =>
                {
                    Some((value.clone(), None))
                }
                Val::Var(value, Some(h)) if h == "length" => Some((value.clone(), None)),
                _ => None,
            };
            if let Some((size, line)) = size {
                if c.negative {
                    return Err(Miss::Unknown);
                }
                let mut list = vec![("font-size", size)];
                match c.modifier {
                    Some(m) => {
                        let leading = match value(m) {
                            Val::Named(n) => match cfg.lookup("leading", &n) {
                                Some(v) => v,
                                None => match (n.parse::<f32>(), cfg.spacing()) {
                                    (Ok(k), Some(step)) => px(k * step),
                                    _ => return invalid(format!("Unknown line height '/{n}'")),
                                },
                            },
                            Val::Arb(v, _) | Val::Var(v, _) => v,
                            Val::Bare => return Err(Miss::Unknown),
                        };
                        list.push(("line-height", leading));
                    }
                    None => {
                        if let Some(line) = line {
                            let line = crate::tailwind::resolve::calc(&line).unwrap_or(line);
                            list.push(("line-height", format!("var(--tw-leading, {line})")));
                        }
                    }
                }
                return Ok(decls(list));
            }
            match color(v, c, cfg) {
                Ok(col) => one("color", col),
                Err(Miss::Unknown) => {
                    if let Val::Named(n) = v {
                        let mut keys = cfg.keys("text");
                        keys.retain(|k| !k.contains("--"));
                        keys.extend(cfg.keys("color"));
                        let keys = keys.iter().map(String::as_str).collect::<Vec<_>>();
                        if let Some(s) = html::suggest(n, &keys) {
                            return invalid(format!(
                                "Unknown text size or color '{n}'; did you mean 'text-{s}'?"
                            ));
                        }
                    }
                    Err(Miss::Unknown)
                }
                Err(e) => Err(e),
            }
        }
        "decoration" => {
            if let Ok(col) = color(v, c, cfg) {
                return one("text-decoration-color", col);
            }
            if let Val::Named(n) = v
                && ["solid", "double", "dotted", "dashed", "wavy"].contains(&n.as_str())
            {
                return one("text-decoration-style", n.clone());
            }
            one("text-decoration-thickness", border_width(v, c)?)
        }
        "underline-offset" => one("text-underline-offset", border_width(v, c)?),
        "indent" => one(
            "text-indent",
            len(Len {
                negative: true,
                keywords: SPACE,
                ..Default::default()
            })?,
        ),
        "align" => Err(Miss::Invalid(
            "vertical-align (align-*) is not implemented by the engine".into(),
        )),
        "content" => match v {
            Val::Arb(v, _) | Val::Var(v, _) => Ok(decls(vec![
                ("--tw-content", v.clone()),
                ("content", "var(--tw-content)".into()),
            ])),
            _ => Err(Miss::Unknown),
        },
        "text-shadow" => invalid("text-shadow is not implemented by the engine"),
        _ => Err(Miss::Unknown),
    }
}
