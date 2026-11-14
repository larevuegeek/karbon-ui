//! Fonds : couleur unie ou dégradé `linear-gradient()` / `radial-gradient()` traduit
//! en `Gradient` du moteur (arrêts, angle ou direction vers un coin), ou en
//! `BackgroundExpression` quand ses couleurs lisent des variables d'élément.
use super::*;

/// Forme du code produit : dégradé constant (couleurs lues à la construction de la vue),
/// ou expression évaluée à la résolution des styles.
enum Target<'a> {
    Brush,
    Expression(&'a BTreeMap<String, String>),
}
impl Target<'_> {
    fn paint(&self, word: &str) -> Result<String> {
        match self {
            Self::Brush => view::paint_color(word),
            Self::Expression(vars) => colors::color_expression(word, vars),
        }
    }
    fn is_color(&self, word: &str) -> bool {
        self.paint(word).is_ok()
    }
    fn stop(&self) -> String {
        match self {
            Self::Brush => format!("{ROOT}::rendering::brush::GradientStop"),
            Self::Expression(_) => format!("{ROOT}::styling::StopExpression"),
        }
    }
}

pub(super) fn background(value: &str) -> Result<String> {
    layer(value, &Target::Brush)
}
/// Couche de fond en `BackgroundExpression` : ses couleurs (`oklch(… var(--h))`) sont
/// évaluées dans la portée de l'élément à la résolution des styles.
pub(super) fn background_expression(
    value: &str,
    vars: &BTreeMap<String, String>,
) -> Result<String> {
    layer(value, &Target::Expression(vars))
}
fn layer(value: &str, target: &Target) -> Result<String> {
    if value.starts_with("radial-gradient(") {
        return radial(value, target);
    }
    let Some(inner) = value
        .strip_prefix("linear-gradient(")
        .and_then(|v| v.strip_suffix(')'))
    else {
        return match target {
            Target::Brush => color(value),
            Target::Expression(vars) => Ok(format!(
                "{ROOT}::styling::BackgroundExpression::Solid({})",
                colors::color_expression(value, vars)?
            )),
        };
    };
    let parts = split(inner, ',')?;
    // `to bottom right` : direction vers un coin, angle calculé sur la boîte peinte.
    if let Some(sides) = parts[0].strip_prefix("to ") {
        let mut corner = (0i8, 0i8);
        for side in sides.split_whitespace() {
            match side {
                "left" if corner.0 == 0 => corner.0 = -1,
                "right" if corner.0 == 0 => corner.0 = 1,
                "top" if corner.1 == 0 => corner.1 = -1,
                "bottom" if corner.1 == 0 => corner.1 = 1,
                _ => return Err(format!("Invalid gradient direction '{}'", parts[0])),
            }
        }
        if corner.0 != 0 && corner.1 != 0 {
            let stops = stops(&parts, 1, target)?;
            return Ok(match target {
                Target::Brush => format!(
                    "{ROOT}::rendering::brush::LinearGradient::to_corner({},{},[{stops}])",
                    corner.0, corner.1
                ),
                Target::Expression(_) => format!(
                    "{ROOT}::styling::BackgroundExpression::Linear{{angle:0.0,corner:Some([{},{}]),stops:&[{stops}]}}",
                    corner.0, corner.1
                ),
            });
        }
    }
    let (angle, start) = if target.is_color(parts[0]) {
        (90., 0)
    } else {
        let direction = match parts[0] {
            "to right" => 0.,
            "to bottom" => 90.,
            "to left" => 180.,
            "to top" => -90.,
            s => angle(s)? - 90.,
        };
        (direction, 1)
    };
    let stops = stops(&parts, start, target)?;
    Ok(match target {
        Target::Brush => {
            format!("{ROOT}::rendering::brush::LinearGradient::new({angle:?},[{stops}])")
        }
        Target::Expression(_) => format!(
            "{ROOT}::styling::BackgroundExpression::Linear{{angle:{angle:?},corner:None,stops:&[{stops}]}}"
        ),
    })
}
fn stops(parts: &[&str], start: usize, target: &Target) -> Result<String> {
    if !(2..=64).contains(&(parts.len() - start)) {
        return Err("A gradient needs 2..64 stops".into());
    }
    let mut colors = Vec::new();
    let mut positions = Vec::new();
    for part in &parts[start..] {
        let tokens = words(part)?;
        if !(1..=2).contains(&tokens.len()) {
            return Err("Use one color and optional percentage per gradient stop".into());
        }
        colors.push(target.paint(&tokens[0])?);
        let position = if tokens.len() == 2 {
            let n = number(
                tokens[1]
                    .strip_suffix('%')
                    .ok_or("Gradient stop positions must be percentages")?,
            )? / 100.;
            if !(0. ..=1.).contains(&n) {
                return Err("Gradient stop outside 0..100%".into());
            }
            Some(n)
        } else {
            None
        };
        positions.push(position);
    }
    let last = positions.len() - 1;
    positions[0].get_or_insert(0.);
    positions[last].get_or_insert(1.);
    let mut start = 0;
    while start < last {
        let end = (start + 1..=last)
            .find(|i| positions[*i].is_some())
            .unwrap();
        let a = positions[start].unwrap();
        let b = positions[end].unwrap();
        if b < a {
            return Err("Gradient stop positions must be sorted".into());
        }
        for (i, position) in positions.iter_mut().enumerate().take(end).skip(start + 1) {
            *position = Some(a + (b - a) * (i - start) as f32 / (end - start) as f32);
        }
        start = end;
    }
    let stop = target.stop();
    let stops = colors
        .iter()
        .zip(positions)
        .map(|(c, p)| format!("{stop}{{position:{:?},color:{c}}}", p.unwrap()))
        .collect::<Vec<_>>()
        .join(",");
    Ok(stops)
}
fn radial(value: &str, target: &Target) -> Result<String> {
    let inner = value
        .strip_prefix("radial-gradient(")
        .and_then(|s| s.strip_suffix(')'))
        .ok_or("Malformed radial-gradient")?;
    let parts = split(inner, ',')?;
    let mut circle = false;
    let mut extent = "FarthestCorner";
    let mut center = "[0.5,0.5]".to_string();
    let mut radius = "[0.5,0.5]".to_string();
    // Rayons explicites : longueurs (px) ou pourcentages de la boîte, 1 ou 2 valeurs.
    let mut sizes: Vec<String> = Vec::new();
    let start = if target.is_color(parts[0]) {
        0
    } else {
        let args = words(parts[0])?;
        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "circle" => circle = true,
                "ellipse" => circle = false,
                "closest-side" => extent = "ClosestSide",
                "farthest-side" => extent = "FarthestSide",
                "closest-corner" => extent = "ClosestCorner",
                "farthest-corner" => extent = "FarthestCorner",
                v if v.ends_with('%') || v.ends_with("px") || v == "0" => sizes.push(v.to_owned()),
                "at" => {
                    let position = args[i + 1..].join(" ");
                    center = super::properties::property("object-position", &position)?[0]
                        .1
                        .clone();
                    break;
                }
                _ => return Err(
                    "Use circle/ellipse, a size keyword and optional at X% Y% in radial-gradient"
                        .into(),
                ),
            }
            i += 1;
        }
        1
    };
    if !sizes.is_empty() {
        let percent = |v: &str| v.strip_suffix('%').map(number).transpose();
        let px_size = |v: &str| px(v, false);
        let pair = match sizes.as_slice() {
            [r] if circle || !r.ends_with('%') => {
                if r.ends_with('%') {
                    return Err("A circle radius must be a length".into());
                }
                let n = px_size(r)?;
                circle = true;
                ("Absolute", [n, n])
            }
            [x, y] if !circle => match (percent(x)?, percent(y)?) {
                (Some(a), Some(b)) => ("Explicit", [a / 100., b / 100.]),
                (None, None) => ("Absolute", [px_size(x)?, px_size(y)?]),
                _ => return Err("Use two percentages or two lengths for ellipse radii".into()),
            },
            _ => {
                return Err(
                    "Use one length (circle) or two radii (ellipse) in radial-gradient".into(),
                );
            }
        };
        if pair.1.iter().any(|n| *n <= 0.) {
            return Err("Radial gradient radii must be positive".into());
        }
        extent = pair.0;
        radius = format!("[{:?},{:?}]", pair.1[0], pair.1[1]);
    }
    let stops = stops(&parts, start, target)?;
    let extent = format!("{ROOT}::rendering::brush::RadialExtent::{extent}");
    Ok(match target {
        Target::Brush => format!(
            "{ROOT}::rendering::brush::RadialGradient::new({center},{radius},[{stops}]).extent({extent},{circle})"
        ),
        Target::Expression(_) => format!(
            "{ROOT}::styling::BackgroundExpression::Radial{{center:{center},radius:{radius},extent:{extent},circle:{circle},stops:&[{stops}]}}"
        ),
    })
}
