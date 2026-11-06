//! Propriétés CSS : dimensions, limites, proportions et modèle de boîte (les
//! espacements sont dans `spacing`).
use super::super::*;
use super::{Next, one, scalar};

/// Traduit les propriétés de la famille ; passe les autres à `next`.
pub(super) fn property(name: &str, value: &str, next: Next) -> Result<Properties> {
    let key = name.replace('-', "_");
    match name {
        "width" | "height" => one(
            name,
            match value {
                "auto" => format!("{ROOT}::view::AUTO"),
                "fill" => {
                    return Err("Use a percentage or Rust fraction() for fill dimensions".into());
                }
                v if v.ends_with('%') => {
                    let n = number(&v[..v.len() - 1])?;
                    if !(0. ..=100.).contains(&n) {
                        return Err("Percentage outside 0..100%".into());
                    }
                    format!("{ROOT}::view::percent({n:?})")
                }
                v => {
                    if let Ok(n) = px(v, false) {
                        scalar(n)
                    } else {
                        format!(
                            "{ROOT}::element::Length::Computed({})",
                            expression::parse(v)?
                        )
                    }
                }
            },
        ),
        "min-width" | "min-height" | "max-width" | "max-height" => {
            if value == "none" && name.starts_with("max") {
                return one(&key, "::core::primitive::f32::INFINITY".into());
            }
            match px(value, false) {
                Ok(n) => one(&key, scalar(n)),
                // Pourcentage ou expression (`40%`, `calc(100vh - 330px)`, `52ch`) :
                // résolu au layout contre la place offerte par le parent.
                Err(error) => {
                    let index = ["min-width", "min-height", "max-width", "max-height"]
                        .iter()
                        .position(|n| *n == name)
                        .unwrap_or(0);
                    let length = if let Some(n) = value.strip_suffix('%') {
                        let n = number(n)?;
                        if !(0. ..=100.).contains(&n) {
                            return Err("Percentage outside 0..100%".into());
                        }
                        format!("{ROOT}::view::percent({n:?})")
                    } else {
                        format!(
                            "{ROOT}::element::Length::Computed({})",
                            expression::parse(value).map_err(|_| error)?
                        )
                    };
                    one("relative_limit", format!("({index}usize,{length})"))
                }
            }
        }
        "aspect-ratio" => one(
            "aspect_ratio",
            if value == "auto" {
                "None".into()
            } else {
                let ratio = match split(value, '/')?.as_slice() {
                    [w] => number(w)?,
                    [w, h] => number(w)? / number(h)?,
                    _ => return Err("Use aspect-ratio: <width> [/ <height>] or auto".into()),
                };
                if !(ratio.is_finite() && ratio > 0.) {
                    return Err("aspect-ratio must be positive".into());
                }
                format!("Some({ratio:?})")
            },
        ),
        "box-sizing" => one(
            "box_sizing",
            format!(
                "{ROOT}::element::BoxSizing::{}",
                match value {
                    "border-box" => "BorderBox",
                    "content-box" => "ContentBox",
                    _ => return Err("Invalid box-sizing".into()),
                }
            ),
        ),
        _ => next(),
    }
}
