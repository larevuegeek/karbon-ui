//! `transform` (suite d'opérations) et `transition` (propriétés animées et minutage).
use super::*;
pub fn transform(value: &str) -> Result<String> {
    let mut code = format!("{ROOT}::animation::Effect::NONE");
    if value == "none" {
        return Ok(code);
    }
    let mut rest = value.trim();
    let mut previous = 0;
    let mut seen = std::collections::BTreeSet::new();
    while !rest.is_empty() {
        let (name, tail) = rest.split_once('(').ok_or("Expected transform function")?;
        let (args, next) = tail.split_once(')').ok_or("Unclosed transform")?;
        let parts = split(args, ',')?;
        let args = if parts.len() == 1 {
            words(args)?
        } else {
            parts.iter().map(|s| s.to_string()).collect()
        };
        let name = name.trim();
        let order = match name {
            "translate" | "translateX" | "translateY" => 1,
            "rotate" => 2,
            "scale" | "scaleX" | "scaleY" => 3,
            _ => return Err(format!("Transform '{name}' is not implemented")),
        };
        if order < previous || !seen.insert(order) {
            return Err("Native transforms currently use translate, then rotate, then scale; each group once".into());
        }
        previous = order;
        // Translation en px ou en pourcentage de la boîte de l'élément.
        let offset = |v: &str| -> Result<(f32, f32)> {
            match v.strip_suffix('%') {
                Some(n) => Ok((0., number(n)?)),
                None => Ok((px(v, true)?, 0.)),
            }
        };
        let translate = |x: &str, y: &str| -> Result<String> {
            let ((x, xp), (y, yp)) = (offset(x)?, offset(y)?);
            let mut code = format!(".translate({x:?},{y:?})");
            if xp != 0. || yp != 0. {
                code += &format!(".translate_percent({xp:?},{yp:?})");
            }
            Ok(code)
        };
        code += &match (name, args.as_slice()) {
            ("translate", [x]) => translate(x, "0")?,
            ("translate", [x, y]) => translate(x, y)?,
            ("translateX", [x]) => translate(x, "0")?,
            ("translateY", [y]) => translate("0", y)?,
            ("rotate", [a]) => format!(".rotate({:?})", angle(a)?),
            ("scale", [s]) => format!(".scale({:?})", number(s)?),
            ("scale", [x, y]) => format!(".scale_xy({:?},{:?})", number(x)?, number(y)?),
            ("scaleX", [x]) => format!(".scale_xy({:?},1.)", number(x)?),
            ("scaleY", [y]) => format!(".scale_xy(1.,{:?})", number(y)?),
            _ => return Err("Invalid transform arguments".into()),
        };
        rest = next.trim();
    }
    Ok(code)
}
pub(super) fn transition(value: &str) -> Result<String> {
    if value == "none" {
        return Ok(format!("{ROOT}::animation::Transition::new().duration(0)"));
    }
    let mut timing = None;
    let (mut effects, mut colors, mut sizes) = (false, false, false);
    for part in split(value, ',')? {
        let args = words(part)?;
        if !(2..=4).contains(&args.len()) {
            return Err("Use transition: <property> 180ms ease-out [delay]".into());
        }
        match args[0].as_str() {
            "opacity" | "transform" => effects = true,
            "color" | "background-color" | "background" | "border-color" | "outline-color" => {
                colors = true
            }
            // Dimensions en pixels : interpolées, avec un layout par image.
            "width" | "height" => sizes = true,
            "all" => (effects, colors, sizes) = (true, true, true),
            _ => {
                return Err(
                    "Transitions animate opacity, transform, color, background-color, border-color, width, height or all"
                        .into(),
                );
            }
        }
        let time = |s: &str| -> Result<u32> {
            let ms = if let Some(v) = s.strip_suffix("ms") {
                number(v)?
            } else if let Some(v) = s.strip_suffix('s') {
                number(v)? * 1000.
            } else {
                return Err("Use ms or s for durations".into());
            };
            if !(0. ..=u32::MAX as f32).contains(&ms) {
                return Err("Invalid transition duration".into());
            }
            Ok(ms.round() as u32)
        };
        let duration = time(&args[1])?;
        let mut easing = "Ease".to_string();
        let mut delay = 0;
        if let Some(e) = args.get(2) {
            easing = match e.as_str() {
                "linear" => "Linear",
                "ease" => "Ease",
                "ease-in" => "EaseIn",
                "ease-out" => "EaseOut",
                "ease-in-out" => "EaseInOut",
                curve if curve.starts_with("cubic-bezier(") => {
                    let inner = curve
                        .strip_prefix("cubic-bezier(")
                        .and_then(|c| c.strip_suffix(')'))
                        .ok_or("Malformed cubic-bezier()")?;
                    let n = split(inner, ',')?
                        .iter()
                        .map(|v| number(v))
                        .collect::<Result<Vec<_>>>()?;
                    if n.len() != 4 || !(0. ..=1.).contains(&n[0]) || !(0. ..=1.).contains(&n[2]) {
                        return Err("cubic-bezier() needs x1, y1, x2, y2 with x in 0..1".into());
                    }
                    // Écrit ci-dessous, une fois `easing` remplacé.
                    ""
                }
                _ => {
                    return Err(
                        "Use linear/ease/ease-in/ease-out/ease-in-out/cubic-bezier() for transition timing"
                            .into(),
                    );
                }
            }
            .into();
            if easing.is_empty() {
                // Courbe cubic-bezier validée ci-dessus.
                let inner = e
                    .strip_prefix("cubic-bezier(")
                    .and_then(|c| c.strip_suffix(')'))
                    .unwrap();
                let n = split(inner, ',')?
                    .iter()
                    .map(|v| number(v))
                    .collect::<Result<Vec<_>>>()?;
                easing = format!("CubicBezier({:?},{:?},{:?},{:?})", n[0], n[1], n[2], n[3]);
            }
        }
        if let Some(d) = args.get(3) {
            delay = time(d)?;
        }
        let code = format!(
            "{ROOT}::animation::Transition::new().duration({duration}).delay({delay}).easing({ROOT}::animation::Easing::{easing})"
        );
        if timing.as_ref().is_some_and(|old| old != &code) {
            return Err("Transitioned properties currently share one timing".into());
        }
        timing = Some(code);
    }
    let sizes = if sizes { ".sizes(true)" } else { "" };
    Ok(format!(
        "{}.properties({effects},{colors}){sizes}",
        timing.unwrap()
    ))
}
