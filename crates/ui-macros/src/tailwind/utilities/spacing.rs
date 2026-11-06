//! Espacements, positions et dimensions : `p-*`, `m-*`, `scroll-p-*`, `scroll-m-*`,
//! `gap-*`, `space-*`, `inset-*`, `top-*`…, `w-*`, `h-*`, `size-*`, `basis-*`.
use super::{
    Miss, R, Val, decls, invalid,
    values::{INSET, Len, MARGIN, SIZE_BOTH, SIZE_H, SIZE_W, SPACE, length},
};
use crate::tailwind::{Config, candidate::Candidate};

/// Utilitaires fonctionnels de la famille ; `Miss::Unknown` pour une autre racine.
pub(super) fn functional(root: &str, v: &Val, c: &Candidate, cfg: &Config) -> R {
    let one = |prop: &str, value: String| Ok(decls(vec![(prop, value)]));
    // Espacements.
    for (prefix, property) in [
        ("p", "padding"),
        ("m", "margin"),
        ("scroll-p", "scroll-padding"),
        ("scroll-m", "scroll-margin"),
    ] {
        if let Some(suffix) = root.strip_prefix(prefix)
            && let Some(sides) = sides(suffix)
        {
            let margin = prefix.ends_with('m');
            let opts = Len {
                negative: margin,
                keywords: if margin { MARGIN } else { SPACE },
                ..Default::default()
            };
            let value = length(v, c, cfg, opts)?;
            if suffix.is_empty() {
                return one(property, value);
            }
            return Ok(decls(
                sides
                    .iter()
                    .map(|side| (format!("{property}-{side}"), value.clone()))
                    .collect(),
            ));
        }
    }
    let len = |opts: Len| length(v, c, cfg, opts);
    let inset = Len {
        fraction: true,
        negative: true,
        keywords: INSET,
        ..Default::default()
    };
    match root {
        "gap" => one(
            "gap",
            len(Len {
                keywords: SPACE,
                ..Default::default()
            })?,
        ),
        "gap-x" => one(
            "column-gap",
            len(Len {
                keywords: SPACE,
                ..Default::default()
            })?,
        ),
        "gap-y" => one(
            "row-gap",
            len(Len {
                keywords: SPACE,
                ..Default::default()
            })?,
        ),
        "space-x" | "space-y" => {
            let value = len(Len {
                keywords: SPACE,
                negative: true,
                ..Default::default()
            })?;
            let property = if root == "space-y" {
                "margin-bottom"
            } else {
                "margin-right"
            };
            Ok(decls(vec![
                ("--tw-child-not-last", String::new()),
                (property, value),
            ]))
        }
        "inset" => one("inset", len(inset)?),
        "inset-x" => {
            let value = len(inset)?;
            Ok(decls(vec![("left", value.clone()), ("right", value)]))
        }
        "inset-y" => {
            let value = len(inset)?;
            Ok(decls(vec![("top", value.clone()), ("bottom", value)]))
        }
        "top" | "right" | "bottom" | "left" => one(root, len(inset)?),
        "start" => one("left", len(inset)?),
        "end" => one("right", len(inset)?),
        "w" | "min-w" | "max-w" => {
            let property = match root {
                "w" => "width",
                "min-w" => "min-width",
                _ => "max-width",
            };
            if let Val::Named(n) = v
                && n == "none"
                && root == "max-w"
            {
                return one(property, "none".into());
            }
            if let Val::Named(n) = v
                && n == "prose"
            {
                return invalid("max-w-prose (65ch) is not implemented: the engine has no ch unit");
            }
            one(
                property,
                len(Len {
                    fraction: true,
                    container: true,
                    keywords: SIZE_W,
                    ..Default::default()
                })?,
            )
        }
        "h" | "min-h" | "max-h" => {
            let property = match root {
                "h" => "height",
                "min-h" => "min-height",
                _ => "max-height",
            };
            if let Val::Named(n) = v
                && n == "none"
                && root == "max-h"
            {
                return one(property, "none".into());
            }
            one(
                property,
                len(Len {
                    fraction: true,
                    keywords: SIZE_H,
                    ..Default::default()
                })?,
            )
        }
        "size" => {
            let value = len(Len {
                fraction: true,
                keywords: SIZE_BOTH,
                ..Default::default()
            })?;
            Ok(decls(vec![("width", value.clone()), ("height", value)]))
        }
        "basis" => one(
            "flex-basis",
            len(Len {
                fraction: true,
                container: true,
                keywords: &[("auto", "auto"), ("full", "100%"), ("px", "1px")],
                ..Default::default()
            })?,
        ),
        _ => Err(Miss::Unknown),
    }
}
/// Côtés d'un utilitaire d'espacement (`px` → gauche et droite).
pub(super) fn sides(suffix: &str) -> Option<&'static [&'static str]> {
    Some(match suffix {
        "" => &["top", "right", "bottom", "left"],
        "x" => &["left", "right"],
        "y" => &["top", "bottom"],
        "t" => &["top"],
        "r" | "e" => &["right"],
        "b" => &["bottom"],
        "l" | "s" => &["left"],
        _ => return None,
    })
}
