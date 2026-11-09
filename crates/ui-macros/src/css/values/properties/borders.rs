//! Propriétés CSS : bordures, contours et rayons des coins.
use super::super::*;
use super::{Next, SIDES, color_prop, one, scalar};

/// Traduit les propriétés de la famille ; passe les autres à `next`.
pub(super) fn property(name: &str, value: &str, next: Next) -> Result<Properties> {
    match name {
        "border-width" => {
            let values = words(value)?
                .iter()
                .map(|v| px(v, false))
                .collect::<Result<Vec<_>>>()?;
            let edges = match values.as_slice() {
                [a] => return one("border_width", scalar(*a)),
                [a, b] => [*a, *b, *a, *b],
                [a, b, c] => [*a, *b, *c, *b],
                [a, b, c, d] => [*a, *b, *c, *d],
                _ => return Err("border-width needs 1..4 pixel widths".into()),
            };
            Ok(SIDES
                .iter()
                .zip(edges)
                .map(|(side, n)| (format!("border_edge_{side}"), scalar(n)))
                .collect())
        }
        "border-top" | "border-right" | "border-bottom" | "border-left" => {
            let side = &name[7..];
            if value == "none" || value == "0" {
                return one(&format!("border_edge_{side}"), "0.0".into());
            }
            let args = words(value)?;
            let (width, paint) = match args.as_slice() {
                [w, s] if s == "solid" => (w.as_str(), None),
                [w, s, c] if s == "solid" => (w.as_str(), Some(c.as_str())),
                [s, c] if s == "solid" => ("1px", Some(c.as_str())),
                _ => return Err(format!("Use {name}: <width> solid [<color>], or none")),
            };
            let mut out = vec![(format!("border_edge_{side}"), scalar(px(width, false)?))];
            if let Some(c) = paint
                && !c.eq_ignore_ascii_case("currentcolor")
            {
                out.push((format!("border_color_{side}"), view::paint_color(c)?));
            }
            Ok(out)
        }
        "border-top-width" | "border-right-width" | "border-bottom-width" | "border-left-width" => {
            one(
                &format!("border_edge_{}", name[7..].trim_end_matches("-width")),
                scalar(px(value, false)?),
            )
        }
        "border-top-color" | "border-right-color" | "border-bottom-color" | "border-left-color" => {
            if value.eq_ignore_ascii_case("currentcolor") {
                return Ok(Vec::new());
            }
            one(
                &format!("border_color_{}", name[7..].trim_end_matches("-color")),
                view::paint_color(value)?,
            )
        }
        "border-top-style" | "border-right-style" | "border-bottom-style" | "border-left-style" => {
            match value {
                "solid" => Ok(Vec::new()),
                "none" => one(
                    &format!("border_edge_{}", name[7..].trim_end_matches("-style")),
                    "0.0".into(),
                ),
                _ => Err("Only solid borders are implemented".into()),
            }
        }
        "outline" => {
            if value == "none" || value == "0" {
                return one("outline_width", "0.0".into());
            }
            let args = words(value)?;
            let (width, c) = match args.as_slice() {
                [w, s] if s == "solid" => (w.as_str(), None),
                [w, s, c] if s == "solid" => (w.as_str(), Some(c.as_str())),
                _ => return Err("Use outline: <width> solid [<color>], or none".into()),
            };
            let mut out = vec![("outline_width".into(), scalar(px(width, false)?))];
            if let Some(c) = c
                && !c.eq_ignore_ascii_case("currentcolor")
            {
                out.push(("outline_color".into(), color(c)?));
            }
            Ok(out)
        }
        "outline-width" => one("outline_width", scalar(px(value, false)?)),
        "outline-offset" => one("outline_offset", scalar(px(value, true)?)),
        "outline-color" => {
            if value.eq_ignore_ascii_case("currentcolor") {
                return Ok(Vec::new());
            }
            one("outline_color", color(value)?)
        }
        "outline-style" => match value {
            "solid" => Ok(Vec::new()),
            "none" => one("outline_width", "0.0".into()),
            _ => Err("Only solid outlines are implemented".into()),
        },
        "border-radius" => {
            let values = words(value)?
                .iter()
                .map(|v| px(v, false))
                .collect::<Result<Vec<_>>>()?;
            let a = match values.as_slice() {
                [a] => return one("radius", scalar(*a)),
                [a, b] => [*a, *b, *a, *b],
                [a, b, c] => [*a, *b, *c, *b],
                [a, b, c, d] => [*a, *b, *c, *d],
                _ => return Err("border-radius requires 1..4 circular pixel radii".into()),
            };
            one("corner_radii", format!("{a:?}"))
        }
        "border-top-left-radius"
        | "border-top-right-radius"
        | "border-bottom-right-radius"
        | "border-bottom-left-radius" => one(
            &format!(
                "corner_{}",
                name.trim_start_matches("border-")
                    .trim_end_matches("-radius")
                    .replace('-', "_")
            ),
            scalar(px(value, false)?),
        ),
        "border-color" => color_prop("border_color", "border", value, None),
        "border" => {
            if value == "none" {
                return one("border_width", "0.".into());
            }
            let args = words(value)?;
            match args.as_slice() {
                [w, kind, c] if kind == "solid" => {
                    let mut out = vec![("border_width".into(), scalar(px(w, false)?))];
                    out.extend(color_prop("border_color", "border", c, None)?);
                    Ok(out)
                }
                _ => Err("Use border: <width> solid <color>, or none".into()),
            }
        }
        "border-style" => {
            if value == "solid" {
                Ok(Vec::new())
            } else if value == "none" {
                one("border_width", "0.".into())
            } else {
                Err("Only solid borders are implemented".into())
            }
        }
        _ => next(),
    }
}
