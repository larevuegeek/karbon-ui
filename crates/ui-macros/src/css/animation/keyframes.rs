//! `@keyframes` compilées en étapes du moteur : chaque déclaration devient une piste de
//! sa propriété ; les étapes partielles, `from`/`to`, les sélecteurs multiples
//! (`0%, 100%`), l'ordre quelconque et les doublons (la dernière déclaration gagne)
//! sont acceptés.
use super::{ROOT, Result, easing, parser, values};
use std::collections::BTreeMap;

/// Erreur d'une étape : position de la déclaration ou du bloc fautif (`None` : celle du
/// `@keyframes`) et message.
pub(in crate::css) type Located = (Option<usize>, String);

/// Comme [`frames_at`], sans la position de l'erreur.
pub(in crate::css) fn frames(
    items: &[parser::Item],
    vars: &BTreeMap<String, String>,
) -> Result<String> {
    frames_at(items, vars).map_err(|(_, m)| m)
}
/// Position d'un sélecteur d'étape (`from`, `to`, `40%`) entre 0 et 1.
fn offset(selector: &str) -> Result<f32> {
    let offset = match selector.trim() {
        "from" => 0.,
        "to" => 1.,
        s => {
            let percent = s.strip_suffix('%').ok_or_else(|| {
                format!("Keyframe selector '{s}' must be from, to or a percentage")
            })?;
            values::number(percent)? / 100.
        }
    };
    if !(0. ..=1.).contains(&offset) {
        return Err("Keyframe offset outside 0..100%".into());
    }
    Ok(offset)
}
/// Propriété animable d'une étape : son nom canonique et l'appel du constructeur
/// `Keyframe` qui la déclare.
fn property(name: &str, value: &str) -> Result<(&'static str, String)> {
    let color = |what: &str| {
        values::color(value).map_err(|e| format!("Keyframe {what} expects a single colour: {e}"))
    };
    Ok(match name {
        "opacity" => {
            let n = values::number(value)?;
            if !(0. ..=1.).contains(&n) {
                return Err("Invalid keyframe opacity".into());
            }
            ("opacity", format!(".opacity({n:?})"))
        }
        "transform" => (
            "transform",
            format!(".transform({})", values::transform(value)?),
        ),
        "color" => ("color", format!(".color({})", color("color")?)),
        "background-color" | "background" => (
            "background-color",
            format!(".background_color({})", color(name)?),
        ),
        "border-color" => (
            "border-color",
            format!(".border_color({})", color("border-color")?),
        ),
        "width" => ("width", format!(".width({})", length(name, value)?)),
        "height" => ("height", format!(".height({})", length(name, value)?)),
        _ => {
            return Err(format!(
                "Keyframe property '{name}' is not animatable yet (opacity, transform, color, background-color, border-color, width, height)"
            ));
        }
    })
}
/// Dimension d'une étape : pixels, pourcentage du bloc conteneur (positif, au-delà de
/// 100 % compris) ou `auto` (animation discrète).
fn length(name: &str, value: &str) -> Result<String> {
    let value = value.trim();
    let root = "__ui_root::ui::element::Length";
    if value == "auto" {
        return Ok(format!("{root}::Auto"));
    }
    if let Some(n) = value.strip_suffix('%') {
        let n = values::number(n)?;
        if n < 0. {
            return Err(format!("Keyframe {name} cannot be negative"));
        }
        return Ok(format!("{root}::Percent({n:?})"));
    }
    let n = values::px(value, true).map_err(|_| {
        format!("Keyframe {name} expects pixels, a percentage or auto (calc(), em and other units are not animatable yet): '{value}'")
    })?;
    if n < 0. {
        return Err(format!("Keyframe {name} cannot be negative"));
    }
    Ok(format!("{root}::Px({n:?})"))
}
/// Déclaration retenue d'une propriété à un offset : code et courbe du bloc qui l'a
/// déclarée (elle s'applique jusqu'à l'étape suivante qui déclare la propriété).
struct Step {
    offset: f32,
    property: &'static str,
    code: String,
    easing: Option<String>,
}
/// Code `&[Keyframe]` des blocs d'un `@keyframes`.
pub(in crate::css) fn frames_at(
    items: &[parser::Item],
    vars: &BTreeMap<String, String>,
) -> std::result::Result<String, Located> {
    let mut steps: Vec<Step> = Vec::new();
    for item in items {
        let parser::Item::Rule {
            selectors,
            declarations,
            at,
        } = item
        else {
            return Err((None, "Keyframes contain percentage blocks only".into()));
        };
        let offsets = selectors
            .iter()
            .map(|s| offset(s))
            .collect::<Result<Vec<_>>>()
            .map_err(|m| (Some(*at), m))?;
        let resolve =
            |d: &parser::Declaration| values::resolve(&d.value, vars).map_err(|m| (Some(d.at), m));
        let curve = match declarations
            .iter()
            .rev()
            .find(|d| d.name == "animation-timing-function")
        {
            Some(d) => Some(easing(&resolve(d)?).map_err(|m| (Some(d.at), m))?),
            None => None,
        };
        for d in declarations {
            if d.name == "animation-timing-function" {
                continue;
            }
            let (name, code) = property(&d.name, &resolve(d)?).map_err(|m| (Some(d.at), m))?;
            for &offset in &offsets {
                let step = Step {
                    offset,
                    property: name,
                    code: code.clone(),
                    easing: curve.clone(),
                };
                match steps
                    .iter_mut()
                    .find(|s| s.offset == offset && s.property == name)
                {
                    Some(old) => *old = step,
                    None => steps.push(step),
                }
            }
        }
    }
    steps.sort_by(|a, b| a.offset.total_cmp(&b.offset));
    // Une étape du moteur par offset et par courbe : deux propriétés d'un même offset
    // peuvent venir de blocs à courbes différentes.
    let mut out: Vec<(f32, Option<String>, String)> = Vec::new();
    for step in steps {
        match out
            .iter_mut()
            .find(|(o, e, _)| *o == step.offset && *e == step.easing)
        {
            Some((_, _, code)) => *code += &step.code,
            None => out.push((step.offset, step.easing, step.code)),
        }
    }
    if out.len() > 128 {
        return Err((None, "Keyframes allow at most 128 steps".into()));
    }
    let frames = out
        .into_iter()
        .map(|(offset, curve, code)| {
            let curve = curve.map(|e| format!(".easing({e})")).unwrap_or_default();
            format!("{ROOT}::Keyframe::at({offset:?}){code}{curve}")
        })
        .collect::<Vec<_>>();
    Ok(format!("&[{}]", frames.join(",")))
}
