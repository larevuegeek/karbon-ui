//! Mise en page : flex et grille (`flex-*`, `grow`, `grid-cols-*`, `col-span-*`…),
//! ordre, `z-*`, `aspect-*`, conteneurs `@container`, curseur et refus documentés
//! (`columns-*`, `auto-cols-*`).
use super::{
    Miss, Out, R, Val, decls, invalid,
    values::{Len, integer, length, number},
};
use crate::tailwind::{Config, candidate::Candidate};

/// Utilitaires fonctionnels de la famille ; `Miss::Unknown` pour une autre racine.
pub(super) fn functional(root: &str, v: &Val, c: &Candidate, cfg: &Config) -> R {
    let one = |prop: &str, value: String| Ok(decls(vec![(prop, value)]));
    match root {
        "flex" => {
            match v {
                Val::Named(n) if n.parse::<u32>().is_ok() && c.modifier.is_none() => {
                    one("flex", n.clone())
                }
                Val::Named(n) if c.modifier.is_some() => {
                    // `flex-1/2` : flex-basis fractionnaire.
                    one(
                        "flex",
                        length(
                            &Val::Named(n.clone()),
                            c,
                            cfg,
                            Len {
                                fraction: true,
                                ..Default::default()
                            },
                        )?,
                    )
                }
                Val::Arb(v, _) | Val::Var(v, _) => one("flex", v.clone()),
                _ => Err(Miss::Unknown),
            }
        }
        "grow" => one("flex-grow", number(v, c)?),
        "shrink" => one("flex-shrink", number(v, c)?),
        "order" => one("order", integer(v, c)?),
        "z" => {
            if let Val::Named(n) = v
                && n == "auto"
            {
                return one("z-index", "auto".into());
            }
            one("z-index", integer(v, c)?)
        }
        "grid-cols" | "grid-rows" => {
            let property = if root == "grid-cols" {
                "grid-template-columns"
            } else {
                "grid-template-rows"
            };
            match v {
                Val::Named(n) if n == "none" => one(property, "none".into()),
                Val::Named(n) if n == "subgrid" => {
                    invalid("subgrid is not implemented by the engine")
                }
                Val::Named(n) if n.parse::<u32>().is_ok() => {
                    one(property, format!("repeat({n}, minmax(0, 1fr))"))
                }
                Val::Arb(v, _) | Val::Var(v, _) => one(property, v.clone()),
                _ => Err(Miss::Unknown),
            }
        }
        "col-span" | "row-span" => {
            let property = if root == "col-span" {
                "grid-column"
            } else {
                "grid-row"
            };
            match v {
                Val::Named(n) if n == "full" => one(property, "1 / -1".into()),
                Val::Named(n) if n.parse::<u32>().is_ok() => one(property, format!("span {n}")),
                Val::Arb(v, _) | Val::Var(v, _) => one(property, format!("span {v}")),
                _ => Err(Miss::Unknown),
            }
        }
        "col" | "row" => {
            let property = if root == "col" {
                "grid-column"
            } else {
                "grid-row"
            };
            match v {
                Val::Named(n) if n == "auto" => one(property, "auto".into()),
                Val::Arb(v, _) | Val::Var(v, _) => one(property, v.clone()),
                _ => one(property, integer(v, c)?),
            }
        }
        "col-start" | "row-start" | "col-end" | "row-end" => {
            let property = match root {
                "col-start" => "grid-column-start",
                "row-start" => "grid-row-start",
                "col-end" => "grid-column-end",
                _ => "grid-row-end",
            };
            if let Val::Named(n) = v
                && n == "auto"
            {
                return one(property, "auto".into());
            }
            one(property, integer(v, c)?)
        }
        "aspect" => match v {
            Val::Named(n) if n == "video" => one("aspect-ratio", "16 / 9".into()),
            Val::Named(n) if c.modifier.is_some() && n.parse::<u32>().is_ok() => {
                one("aspect-ratio", format!("{n} / {}", c.modifier.unwrap()))
            }
            Val::Named(n) => match cfg.lookup("aspect", n) {
                Some(v) => one("aspect-ratio", v),
                None => Err(Miss::Unknown),
            },
            Val::Arb(v, _) | Val::Var(v, _) => one("aspect-ratio", v.replace('/', " / ")),
            Val::Bare => Err(Miss::Unknown),
        },
        "@container" => match v {
            Val::Named(_) | Val::Arb(..) | Val::Var(..) => Err(Miss::Unknown),
            Val::Bare => Err(Miss::Unknown),
        },
        "cursor" => match v {
            Val::Arb(v, _) | Val::Var(v, _) => one("cursor", v.clone()),
            _ => Err(Miss::Unknown),
        },
        "will-change" => Ok(Out::default()),
        "columns" => invalid("columns-* (multi-column layout) is not implemented by the engine"),
        "auto-cols" | "auto-rows" => {
            invalid("grid-auto-columns/rows are not implemented by the engine")
        }
        _ => Err(Miss::Unknown),
    }
}
