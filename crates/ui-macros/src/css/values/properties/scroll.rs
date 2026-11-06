//! Propriétés CSS : défilement (marges d’accroche, barres, débordement) et conteneurs de requêtes.
use super::super::*;
use super::{Next, container_names, container_type, one};

/// Traduit les propriétés de la famille ; passe les autres à `next`.
pub(super) fn property(name: &str, value: &str, next: Next) -> Result<Properties> {
    let key = name.replace('-', "_");
    match name {
        // Marges et retraits de défilement : zone gardée visible quand un élément est
        // amené dans la vue (focus), et retrait de la vue du conteneur.
        "scroll-margin" | "scroll-padding" => {
            let values = words(value)?
                .iter()
                .map(|v| px(v, name == "scroll-margin"))
                .collect::<Result<Vec<_>>>()?;
            let v = match values.as_slice() {
                [a] => [*a, *a, *a, *a],
                [a, b] => [*a, *b, *a, *b],
                [a, b, c] => [*a, *b, *c, *b],
                [a, b, c, d] => [*a, *b, *c, *d],
                _ => return Err(format!("{name} needs 1..4 values")),
            };
            let key = format!("{key}_edge");
            Ok((0..4)
                .map(|i| (key.clone(), format!("({i}usize,{:?}f32)", v[i])))
                .collect())
        }
        n if n.starts_with("scroll-margin-") || n.starts_with("scroll-padding-") => {
            let (property, side) = n.rsplit_once('-').unwrap();
            let index = match side {
                "top" => 0,
                "right" => 1,
                "bottom" => 2,
                "left" => 3,
                _ => {
                    return Err(format!(
                        "{n} is not implemented; use top, right, bottom or left"
                    ));
                }
            };
            one(
                &format!("{}_edge", property.replace('-', "_")),
                format!(
                    "({index}usize,{:?}f32)",
                    px(value, property == "scroll-margin")?
                ),
            )
        }
        "overscroll-behavior" | "overscroll-behavior-x" | "overscroll-behavior-y" => {
            let parts = words(value)?;
            let contain = |v: &str| match v {
                "contain" | "none" => Ok(true),
                "auto" => Ok(false),
                _ => Err(format!("Invalid overscroll-behavior '{v}'")),
            };
            let values = parts
                .iter()
                .map(|v| contain(v))
                .collect::<Result<Vec<_>>>()?;
            if values.is_empty() || values.len() > 2 || values.iter().any(|v| *v != values[0]) {
                return Err("overscroll-behavior: one value for both axes is implemented".into());
            }
            one("overscroll_contain", values[0].to_string())
        }
        "scrollbar-width" => one(
            "scrollbar_width",
            format!(
                "{ROOT}::element::ScrollbarWidth::{}",
                match value {
                    "auto" => "Auto",
                    "thin" => "Thin",
                    "none" => "None",
                    _ => return Err("Use scrollbar-width: auto, thin or none".into()),
                }
            ),
        ),
        "scrollbar-color" => one(
            "scrollbar_color",
            if value == "auto" {
                "(None,None)".into()
            } else {
                match words(value)?.as_slice() {
                    [thumb, track] => format!("(Some({}),Some({}))", color(thumb)?, color(track)?),
                    _ => {
                        return Err(
                            "Use scrollbar-color: <thumb colour> <track colour> or auto".into()
                        );
                    }
                }
            },
        ),
        "scrollbar-gutter" => one(
            "scrollbar_gutter",
            match value {
                "auto" => "false",
                "stable" => "true",
                "stable both-edges" => {
                    return Err(
                        "scrollbar-gutter: stable both-edges is not supported; use stable".into(),
                    );
                }
                _ => return Err("Use scrollbar-gutter: auto or stable".into()),
            }
            .into(),
        ),
        "container-type" => one("container_type", container_type(value)?),
        "container-name" => one("container_name", container_names(value)?),
        "container" => {
            // container: <noms> [/ <type>]
            let (names, kind) = match split(value, '/')?.as_slice() {
                [names] => (*names, "normal"),
                [names, kind] => (*names, *kind),
                _ => return Err("Use container: <name> [/ <type>]".into()),
            };
            Ok(vec![
                ("container_name".into(), container_names(names)?),
                ("container_type".into(), container_type(kind)?),
            ])
        }
        _ => next(),
    }
}
