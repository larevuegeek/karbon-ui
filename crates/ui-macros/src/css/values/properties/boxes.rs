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
                // Hauteur `max-content` : celle du contenu, comme `auto`.
                "max-content" if name == "height" => format!("{ROOT}::view::AUTO"),
                "max-content" => format!("{ROOT}::element::Length::MaxContent"),
                "min-content" | "fit-content" => {
                    return Err(format!(
                        "{name}: {value} is not implemented (use max-content, auto, a length or a percentage)"
                    ));
                }
                v if v.ends_with('%') => {
                    let n = number(&v[..v.len() - 1])?;
                    if n < 0. {
                        return Err("A width or height percentage cannot be negative".into());
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
                        if n < 0. {
                            return Err("A size limit percentage cannot be negative".into());
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
        // Contenu sauté loin de la vue (`auto`) ou toujours (`hidden`), et sa taille
        // pendant ce temps.
        "content-visibility" => one(
            "content_visibility",
            format!(
                "{ROOT}::element::ContentVisibility::{}",
                match value {
                    "visible" => "Visible",
                    "auto" => "Auto",
                    "hidden" => "Hidden",
                    _ => return Err("Use content-visibility: visible, auto or hidden".into()),
                }
            ),
        ),
        "contain-intrinsic-size" | "contain-intrinsic-width" | "contain-intrinsic-height" => {
            let lengths = intrinsic_lengths(value)?;
            let axes: &[usize] = match (name, lengths.len()) {
                ("contain-intrinsic-width", 1) => &[0],
                ("contain-intrinsic-height", 1) => &[1],
                ("contain-intrinsic-size", 1 | 2) => &[0, 1],
                _ => return Err(format!("Invalid {name}: '{value}'")),
            };
            Ok(axes
                .iter()
                .map(|&axis| {
                    let (remembered, length) = lengths[axis.min(lengths.len() - 1)];
                    let length = length.map_or("None".into(), |n| format!("Some({n:?})"));
                    (
                        "contain_intrinsic_size".into(),
                        format!("({axis}usize,{remembered},{length})"),
                    )
                })
                .collect())
        }
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

/// Valeurs de `contain-intrinsic-*` : `[auto] none | <longueur>`, une ou deux fois.
fn intrinsic_lengths(value: &str) -> Result<Vec<(bool, Option<f32>)>> {
    let mut lengths = Vec::new();
    let mut remembered = false;
    for word in words(value)? {
        match word.as_str() {
            "auto" if !remembered => remembered = true,
            "none" => lengths.push((std::mem::take(&mut remembered), None)),
            _ => {
                let n = px(&word, false)?;
                lengths.push((std::mem::take(&mut remembered), Some(n)));
            }
        }
    }
    if remembered || lengths.is_empty() || lengths.len() > 2 {
        return Err(format!(
            "Use contain-intrinsic-size: [auto] <length> or none (one or two values), not '{value}'"
        ));
    }
    Ok(lengths)
}
