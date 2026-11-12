//! Propriétés CSS : affichage, positionnement, débordement, images remplacées et interaction.
use super::super::*;
use super::{Next, SIDES, one};

/// Traduit les propriétés de la famille ; passe les autres à `next`.
pub(super) fn property(name: &str, value: &str, next: Next) -> Result<Properties> {
    let key = name.replace('-', "_");
    match name {
        "display" => {
            // Règle conditionnelle : un modèle visible annule aussi `display: none`.
            let shown = ("hidden".to_string(), "false".to_string());
            match value {
                "flex" | "inline-flex" => {
                    Ok(vec![("flex_direction".into(), "\"row\"".into()), shown])
                }
                "grid" | "inline-grid" => Ok(vec![("display_grid".into(), "true".into()), shown]),
                "block" => Ok(vec![("display_block".into(), "true".into()), shown]),
                // Boîte en ligne d'un élément du moteur : traitée en bloc (voir
                // `base_display`), seulement réaffichée.
                "inline" | "inline-block" => Ok(vec![shown]),
                "none" => one("hidden", "true".into()),
                _ => Err(format!(
                    "display: {value} is not implemented; use block, flex, inline-flex, grid, inline-grid or none"
                )),
            }
        }
        "position" => one(
            "css_position",
            format!(
                "{ROOT}::element::Position::{}",
                match value {
                    "static" => "Static",
                    "relative" => "Relative",
                    "absolute" => "Absolute",
                    "fixed" => "Fixed",
                    "sticky" => "Sticky",
                    _ => return Err("Unsupported position".into()),
                }
            ),
        ),
        "top" | "right" | "bottom" | "left" => one(
            &format!("offset_{name}"),
            if value == "auto" {
                format!("{ROOT}::element::Length::Auto")
            } else {
                format!(
                    "{ROOT}::element::Length::Computed({})",
                    expression::parse(value)?
                )
            },
        ),
        "z-index" => one(
            "z_index",
            value
                .parse::<i32>()
                .map_err(|_| "z-index requires an integer")?
                .to_string(),
        ),
        "overflow" | "overflow-x" | "overflow-y" => one(
            &key,
            format!(
                "{ROOT}::element::Overflow::{}",
                match value {
                    "visible" => "Visible",
                    "hidden" => "Hidden",
                    "clip" => "Clip",
                    "auto" => "Auto",
                    "scroll" => "Scroll",
                    _ => return Err("Use visible, hidden, clip, auto or scroll".into()),
                }
            ),
        ),
        "object-fit" => one(
            "image_fit",
            format!(
                "{ROOT}::styling::ImageFit::{}",
                match value {
                    "fill" => "Fill",
                    "contain" => "Contain",
                    "cover" => "Cover",
                    "none" => "None",
                    "scale-down" => "ScaleDown",
                    _ => return Err("Invalid object-fit".into()),
                }
            ),
        ),
        "object-position" => {
            let mut parts = words(value)?;
            if parts.len() == 1 {
                if ["top", "bottom"].contains(&parts[0].as_str()) {
                    parts.insert(0, "center".into());
                } else {
                    parts.push("center".into());
                }
            }
            if parts.len() != 2 {
                return Err(
                    "object-position requires one or two percentage/keyword coordinates".into(),
                );
            }
            if ["top", "bottom"].contains(&parts[0].as_str())
                || ["left", "right"].contains(&parts[1].as_str())
            {
                parts.swap(0, 1);
            }
            let coordinate = |v: &str, horizontal| -> Result<f32> {
                Ok(match v {
                    "center" => 0.5,
                    "left" if horizontal => 0.,
                    "right" if horizontal => 1.,
                    "top" if !horizontal => 0.,
                    "bottom" if !horizontal => 1.,
                    _ => {
                        number(
                            v.strip_suffix('%')
                                .ok_or("Use percentages or edge keywords for object-position")?,
                        )? / 100.
                    }
                })
            };
            one(
                "image_position",
                format!(
                    "[{:?},{:?}]",
                    coordinate(&parts[0], true)?,
                    coordinate(&parts[1], false)?
                ),
            )
        }
        "inset" => {
            let parts = words(value)?;
            let parts = match parts.as_slice() {
                [a] => [a, a, a, a],
                [a, b] => [a, b, a, b],
                [a, b, c] => [a, b, c, b],
                [a, b, c, d] => [a, b, c, d],
                _ => return Err("inset needs 1..4 values".into()),
            };
            let mut out = Vec::new();
            for (side, v) in SIDES.iter().zip(parts) {
                out.extend(super::property(side, v)?);
            }
            Ok(out)
        }
        "cursor" => {
            let shape = match value {
                "auto" | "default" => "Arrow",
                "pointer" => "Hand",
                "text" | "vertical-text" => "Text",
                "not-allowed" | "no-drop" => "NotAllowed",
                "wait" => "Wait",
                "progress" => "Progress",
                "help" | "context-menu" => "Help",
                "move" | "grab" | "grabbing" | "all-scroll" => "Move",
                "crosshair" | "cell" => "Crosshair",
                "col-resize" | "ew-resize" | "e-resize" | "w-resize" => "ResizeHorizontal",
                "row-resize" | "ns-resize" | "n-resize" | "s-resize" => "ResizeVertical",
                "nwse-resize" | "nw-resize" | "se-resize" => "ResizeDiagonal",
                "nesw-resize" | "ne-resize" | "sw-resize" => "ResizeAntiDiagonal",
                _ => {
                    return Err(format!(
                        "cursor: {value} is not implemented; use auto, default, pointer, text, not-allowed, wait, progress, help, move, grab, crosshair or a resize cursor"
                    ));
                }
            };
            one("cursor", format!("{ROOT}::runtime::Cursor::{shape}"))
        }
        "pointer-events" => match value {
            "none" => one("pointer_events", "false".into()),
            "auto" => one("pointer_events", "true".into()),
            _ => Err("pointer-events supports none and auto".into()),
        },
        _ => next(),
    }
}
