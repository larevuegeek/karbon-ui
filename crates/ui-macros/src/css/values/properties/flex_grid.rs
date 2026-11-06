//! Propriétés CSS : alignement, flexbox et grille.
use super::super::*;
use super::{Next, one, scalar};

/// Traduit les propriétés de la famille ; passe les autres à `next`.
pub(super) fn property(name: &str, value: &str, next: Next) -> Result<Properties> {
    let key = name.replace('-', "_");
    match name {
        "order" => one(
            "order",
            value
                .parse::<i32>()
                .map_err(|_| "order requires an integer")?
                .to_string(),
        ),
        "grid-template-rows" => one("grid_rows", grid::tracks(value)?),
        "justify-self" | "justify-items" => one(
            if name == "justify-self" {
                "justify_self"
            } else {
                "justify_items"
            },
            if value == "auto" || value == "normal" {
                "None".into()
            } else {
                format!(
                    "Some({ROOT}::element::Align::{})",
                    match value {
                        "start" | "flex-start" | "left" => "Start",
                        "end" | "flex-end" | "right" => "End",
                        "center" => "Center",
                        "stretch" => "Stretch",
                        _ => return Err(format!("Invalid {name}")),
                    }
                )
            },
        ),
        "align-items" => {
            if [
                "start",
                "end",
                "flex-start",
                "flex-end",
                "center",
                "stretch",
                "baseline",
            ]
            .contains(&value)
            {
                one("align", format!("{value:?}"))
            } else {
                Err("Unsupported align-items value".into())
            }
        }
        "justify-content" => {
            if [
                "start",
                "end",
                "flex-start",
                "flex-end",
                "center",
                "space-between",
            ]
            .contains(&value)
            {
                one("justify", format!("{value:?}"))
            } else {
                Err("Unsupported justify-content value".into())
            }
        }
        "flex-direction" => {
            if ["row", "column", "row-reverse", "column-reverse"].contains(&value) {
                Ok(vec![
                    (
                        "flex_direction".into(),
                        format!("{:?}", value.trim_end_matches("-reverse")),
                    ),
                    ("css_flex".into(), "true".into()),
                    (
                        "flex_reverse".into(),
                        value.ends_with("-reverse").to_string(),
                    ),
                ])
            } else {
                Err("Only row and column flex directions are implemented".into())
            }
        }
        "flex-wrap" => one(
            "flex_wrap",
            format!(
                "{ROOT}::element::FlexWrap::{}",
                match value {
                    "nowrap" => "NoWrap",
                    "wrap" => "Wrap",
                    "wrap-reverse" => "WrapReverse",
                    _ => return Err("Invalid flex-wrap".into()),
                }
            ),
        ),
        "flex-grow" | "flex-shrink" => {
            let n = number(value)?;
            if n < 0. {
                return Err("Flex factors must be nonnegative".into());
            }
            one(&key, scalar(n))
        }
        "flex-basis" => {
            let mut p = super::property("width", value)?;
            p[0].0 = "flex_basis".into();
            Ok(p)
        }
        "flex" => {
            let parts = words(value)?;
            let (grow, shrink, basis) = match parts.as_slice() {
                [v] if v == "none" => ("0", "0", "auto"),
                [v] if v == "auto" => ("1", "1", "auto"),
                [v] => (v.as_str(), "1", "0px"),
                [g, s, b] => (g.as_str(), s.as_str(), b.as_str()),
                _ => return Err("Use flex: grow shrink basis, auto, none or a grow factor".into()),
            };
            let mut out = super::property("flex-grow", grow)?;
            out.extend(super::property("flex-shrink", shrink)?);
            out.extend(super::property("flex-basis", basis)?);
            Ok(out)
        }
        "align-self" => one(
            "align_self",
            if value == "auto" {
                "None".into()
            } else {
                format!(
                    "Some({ROOT}::element::Align::{})",
                    match value {
                        "start" | "flex-start" => "Start",
                        "end" | "flex-end" => "End",
                        "center" => "Center",
                        "stretch" => "Stretch",
                        _ => return Err("Invalid align-self".into()),
                    }
                )
            },
        ),
        "grid-template-columns" => {
            // `repeat(auto-fill|auto-fit, piste)` : une piste déclarée, répétée au layout
            // autant que la largeur le permet.
            if let Some(inner) = value
                .strip_prefix("repeat(")
                .and_then(|v| v.strip_suffix(')'))
                && let Some((count, track)) = inner.split_once(',')
                && matches!(count.trim(), "auto-fill" | "auto-fit")
            {
                let tracks = grid::tracks(track.trim())?;
                if tracks.matches("GridTrack::").count() != 1 {
                    return Err("repeat(auto-fill|auto-fit, …) supports one track".into());
                }
                return Ok(vec![
                    ("grid_tracks".into(), tracks),
                    (
                        "grid_auto_repeat".into(),
                        (count.trim() == "auto-fit").to_string(),
                    ),
                ]);
            }
            one("grid_tracks", grid::tracks(value)?)
        }
        "grid-column" if value.contains('/') => {
            // `a / b` : colonne a jusqu'à la ligne b ; `1 / -1` va jusqu'à la dernière
            // ligne (`col-span-full`) ; `span n / span n` vaut `span n`.
            let (start, end) = value.split_once('/').unwrap();
            let (start, end) = (start.trim(), end.trim());
            if start == end && start.starts_with("span ") {
                return super::property("grid-column", start);
            }
            let start = start
                .parse::<usize>()
                .ok()
                .filter(|n| *n > 0)
                .ok_or("Use grid-column: N / M, N / -1 or span N")?;
            let span = match end {
                "-1" => "usize::MAX".to_owned(),
                e => {
                    let end = e
                        .parse::<usize>()
                        .ok()
                        .filter(|n| *n > start)
                        .ok_or("Use grid-column: N / M (M > N), N / -1 or span N")?;
                    (end - start).to_string()
                }
            };
            // `1 / -1` : toutes les colonnes, qui ne tiennent qu'à partir de la première ;
            // sans colonne imposée, un `span` conditionnel (`@3xl:col-span-1`) laisse
            // l'élément se placer seul.
            if start == 1 && span == "usize::MAX" {
                return one("span", span);
            }
            Ok(vec![
                ("column".into(), start.to_string()),
                ("span".into(), span),
            ])
        }
        "grid-column" => {
            if let Some(span) = value.strip_prefix("span ") {
                let n = span
                    .trim()
                    .parse::<usize>()
                    .map_err(|_| "Invalid grid span")?;
                if n == 0 {
                    return Err("Grid span starts at 1".into());
                }
                one("span", n.to_string())
            } else {
                let n = value
                    .parse::<usize>()
                    .map_err(|_| "Use grid-column: N or span N")?;
                if n == 0 {
                    return Err("Grid columns start at 1".into());
                }
                one("column", n.to_string())
            }
        }
        "grid-row" | "grid-row-start" | "grid-column-start" => {
            let n = value
                .parse::<usize>()
                .map_err(|_| "Grid position must be an integer")?;
            if n == 0 {
                return Err("Grid positions start at 1".into());
            }
            one(
                if name.starts_with("grid-row") {
                    "row"
                } else {
                    "column"
                },
                n.to_string(),
            )
        }
        _ => next(),
    }
}
