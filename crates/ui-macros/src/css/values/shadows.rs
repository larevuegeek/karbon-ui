//! `box-shadow` : couches d'ombres (décalages, flou, étalement, couleur, `inset`).
use super::*;
pub(super) fn parse(value: &str) -> Result<String> {
    let mut shadows = Vec::new();
    if value != "none" {
        for layer in split(value, ',')? {
            let mut lengths = Vec::new();
            let mut tint = None;
            let mut inset = false;
            for word in words(layer)? {
                if word == "inset" {
                    if inset {
                        return Err("Duplicate inset in box-shadow".into());
                    }
                    inset = true;
                } else if let Ok(color) = color(&word) {
                    if tint.replace(color).is_some() {
                        return Err("Multiple colors in box-shadow".into());
                    }
                } else {
                    lengths.push(px(&word, true)?);
                }
            }
            if !(2..=4).contains(&lengths.len()) {
                return Err("box-shadow requires X Y [blur] [spread] and a color".into());
            }
            let x = lengths[0];
            let y = lengths[1];
            let blur = *lengths.get(2).unwrap_or(&0.);
            let spread = *lengths.get(3).unwrap_or(&0.);
            if !(0. ..=512.).contains(&blur) {
                return Err("box-shadow blur must be within 0..512px".into());
            }
            let color = tint.ok_or("Specify an explicit box-shadow color")?;
            shadows.push(format!("{ROOT}::rendering::component_shadow::BoxShadow{{x:{x:?},y:{y:?},blur:{blur:?},spread:{spread:?},color:{color},inset:{inset}}}"));
            if shadows.len() > 16 {
                return Err("At most 16 component shadows".into());
            }
        }
    }
    Ok(format!(
        "{ROOT}::rendering::component_shadow::Shadows::new(const{{&[{}]}})",
        shadows.join(",")
    ))
}
