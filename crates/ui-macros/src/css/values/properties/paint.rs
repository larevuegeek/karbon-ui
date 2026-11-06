//! Propriétés CSS : fonds, couleurs, filtres, masques, ombres et opacité.
use super::super::*;
use super::{Next, color_prop, one, scalar};

/// Traduit les propriétés de la famille ; passe les autres à `next`.
pub(super) fn property(name: &str, value: &str, next: Next) -> Result<Properties> {
    match name {
        "filter" => one("filters", filters::parse(value)?),
        "backdrop-filter" | "-webkit-backdrop-filter" => one("backdrop", filters::parse(value)?),
        "mask-image" | "-webkit-mask-image" => one(
            "mask_image",
            if value == "none" {
                "None".into()
            } else if value.starts_with("linear-gradient(") {
                format!("Some(::std::sync::Arc::new({}))", background(value)?)
            } else {
                return Err("mask-image supports one linear-gradient() or none".into());
            },
        ),
        "background-clip" | "-webkit-background-clip" => one(
            "text_clip",
            match value {
                "text" => "true",
                "border-box" => "false",
                _ => return Err("background-clip supports text and border-box".into()),
            }
            .into(),
        ),
        "-webkit-text-fill-color" => {
            if value == "transparent" {
                // Avec background-clip: text, les glyphes prennent déjà le fond.
                Ok(Vec::new())
            } else {
                Err(
                    "-webkit-text-fill-color supports transparent (with background-clip: text)"
                        .into(),
                )
            }
        }
        "box-shadow" => one("box_shadows", shadows::parse(value)?),
        "background" => {
            let layers = split(value, ',')?;
            if layers.len() == 1 && !value.contains("gradient(") {
                return color_prop("background", "background", value, None);
            }
            if layers.len() == 1 {
                return one("background", background(value)?);
            }
            one(
                "background_layers",
                format!(
                    "::std::vec![{}]",
                    layers
                        .iter()
                        .map(|s| background(s)
                            .map(|s| format!("{ROOT}::rendering::brush::Background::from({s})")))
                        .collect::<Result<Vec<_>>>()?
                        .join(",")
                ),
            )
        }
        "background-image" => {
            let layers = if value == "none" {
                Vec::new()
            } else {
                split(value, ',')?
                    .iter()
                    .map(|s| {
                        if !s.starts_with("linear-gradient(") && !s.starts_with("radial-gradient(")
                        {
                            return Err(
                                "background-image accepts linear/radial gradients or none".into()
                            );
                        }
                        background(s)
                            .map(|s| format!("{ROOT}::rendering::brush::Background::from({s})"))
                    })
                    .collect::<Result<Vec<_>>>()?
            };
            one("image_layers", format!("::std::vec![{}]", layers.join(",")))
        }
        "background-color" => color_prop("background_color", "background", value, None),
        "color" => {
            if value.eq_ignore_ascii_case("currentcolor") {
                // `color: currentColor` vaut la couleur héritée.
                return Ok(Vec::new());
            }
            color_prop("color", "text", value, None)
        }
        "accent-color" => color_prop("accent_color", "accent", value, None),
        "opacity" => {
            let n = number(value)?;
            if !(0. ..=1.).contains(&n) {
                return Err("Opacity outside 0..1".into());
            }
            one("opacity", scalar(n))
        }
        _ => next(),
    }
}
