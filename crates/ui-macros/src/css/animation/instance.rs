//! Propriétés `animation` et `animation-*` d'une classe : listes d'animations (une
//! valeur par animation, les listes plus courtes se répètent comme en CSS), validées à
//! la déclaration puis compilées en `Animation` (ou `Animation::group`) du moteur.
use super::{ROOT, Result, easing, parser, time, values};
use std::collections::BTreeMap;

/// Longhands et valeurs initiales, dans l'ordre de la valeur réinitialisée par le
/// raccourci.
const LONGHANDS: [(&str, &str); 8] = [
    ("name", "none"),
    ("duration", "0s"),
    ("delay", "0s"),
    ("timing-function", "ease"),
    ("iteration-count", "1"),
    ("direction", "normal"),
    ("fill-mode", "none"),
    ("play-state", "running"),
];
const DIRECTIONS: [&str; 4] = ["normal", "reverse", "alternate", "alternate-reverse"];
const FILLS: [&str; 4] = ["none", "forwards", "backwards", "both"];
const STATES: [&str; 2] = ["running", "paused"];
/// Animations au plus par élément.
const MAX: usize = 16;

/// Vérifie une valeur d'un longhand (`key` sans le préfixe `animation-`).
fn check(key: &str, value: &str) -> Result<()> {
    match key {
        "name" => {
            if value != "none" {
                parser::class_name(value)
                    .map_err(|_| format!("Invalid animation name '{value}'"))?;
            }
        }
        "duration" => {
            if time(value)? < 0 {
                return Err("Animation duration cannot be negative".into());
            }
        }
        "delay" => {
            time(value)?;
        }
        "timing-function" => {
            easing(value)?;
        }
        "iteration-count" => {
            if value != "infinite" {
                let n = values::number(value).map_err(|_| {
                    "Animation iterations must be a positive integer or infinite".to_owned()
                })?;
                if n == 0. {
                    return Err("Zero animation iterations not implemented".into());
                }
                if n < 0. || n.fract() != 0. || n > u32::MAX as f32 {
                    return Err(format!(
                        "Animation iterations must be a positive integer or infinite (fractional counts are not supported): '{value}'"
                    ));
                }
            }
        }
        "direction" if !DIRECTIONS.contains(&value) => {
            return Err(format!(
                "Invalid animation-direction '{value}' (normal, reverse, alternate, alternate-reverse)"
            ));
        }
        "fill-mode" if !FILLS.contains(&value) => {
            return Err(format!(
                "Invalid animation-fill-mode '{value}' (none, forwards, backwards, both)"
            ));
        }
        "play-state" if !STATES.contains(&value) => {
            return Err(format!(
                "Invalid animation-play-state '{value}' (running, paused)"
            ));
        }
        _ => {}
    }
    Ok(())
}
/// Valeurs d'une liste séparée par des virgules.
fn list(value: &str) -> Result<Vec<String>> {
    let items = parser::split(value, ',')?
        .into_iter()
        .map(|v| v.trim().to_owned())
        .collect::<Vec<_>>();
    if items.iter().any(String::is_empty) {
        return Err("Empty item in an animation list".into());
    }
    if items.len() > MAX {
        return Err(format!("At most {MAX} animations per element"));
    }
    Ok(items)
}
/// Composants d'une animation du raccourci (`pulse 1s ease-in 200ms infinite`).
fn shorthand(item: &str) -> Result<BTreeMap<&'static str, String>> {
    let mut out = BTreeMap::new();
    let mut times = 0;
    for token in values::words(item)? {
        let key = if (token.ends_with("ms") || token.ends_with('s')) && time(&token).is_ok() {
            times += 1;
            if times > 2 {
                return Err("Too many animation times".into());
            }
            if times == 1 { "duration" } else { "delay" }
        } else if easing(&token).is_ok() || token.starts_with("cubic-bezier(") {
            "timing-function"
        } else if DIRECTIONS.contains(&token.as_str()) {
            "direction"
        } else if FILLS.contains(&token.as_str())
            && !(token == "none" && out.contains_key("fill-mode"))
        {
            "fill-mode"
        } else if STATES.contains(&token.as_str()) {
            "play-state"
        } else if token == "infinite" || values::number(&token).is_ok() {
            "iteration-count"
        } else {
            "name"
        };
        check(key, &token)?;
        if out.contains_key(key) {
            return Err(format!("Duplicate animation {key} in '{item}'"));
        }
        out.insert(key, token);
    }
    Ok(out)
}
pub(in crate::css) fn declaration(
    name: &str,
    value: &str,
    out: &mut BTreeMap<String, String>,
) -> Result<()> {
    if name != "animation" {
        let key = name
            .strip_prefix("animation-")
            .filter(|k| LONGHANDS.iter().any(|(n, _)| n == k))
            .ok_or_else(|| format!("Unsupported {name}"))?;
        let items = list(value)?;
        for item in &items {
            check(key, item)?;
        }
        out.insert(name.into(), items.join(", "));
        return Ok(());
    }
    let parsed = list(value)?
        .iter()
        .map(|item| shorthand(item))
        .collect::<Result<Vec<_>>>()?;
    for (key, default) in LONGHANDS {
        let values = parsed
            .iter()
            .map(|p| p.get(key).map_or(default, String::as_str))
            .collect::<Vec<_>>();
        out.insert(format!("animation-{key}"), values.join(", "));
    }
    Ok(())
}
/// Noms d'animation d'une déclaration `animation` ou `animation-name` (sans `none`),
/// pour signaler un `@keyframes` inconnu à la position de la déclaration.
pub(in crate::css) fn names(name: &str, value: &str) -> Vec<String> {
    let mut out = BTreeMap::new();
    if !matches!(name, "animation" | "animation-name")
        || declaration(name, value, &mut out).is_err()
    {
        return Vec::new();
    }
    out.get("animation-name")
        .map(|v| {
            v.split(", ")
                .filter(|n| *n != "none")
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}
/// Longhand d'animation d'une règle d'état ou de media, en champ d'`AnimationOverride`
/// (`("animation_override", "delay:Some(-1400),")`) ; le nom, le raccourci et les
/// autres longhands restent refusés.
pub(in crate::css) fn state_override(name: &str, value: &str) -> Result<(String, String)> {
    let value = value.trim();
    let key = name.strip_prefix("animation-").unwrap_or("");
    if matches!(name, "animation" | "animation-name") && value == "none" {
        return Ok(("animation_override".into(), "none:true,".into()));
    }
    if value.contains(',') {
        return Err(format!(
            "{name} in a state or media rule takes a single value"
        ));
    }
    if !matches!(key, "delay" | "duration" | "direction" | "play-state") {
        return Err(format!(
            "State-dependent keyframe replacement is not implemented: {name} cannot change in a state or media rule (animation-delay, animation-duration, animation-direction and animation-play-state can)"
        ));
    }
    check(key, value)?;
    let field = match key {
        "delay" => format!("delay:Some({}),", time(value)?),
        "duration" => format!("duration:Some({}),", time(value)?),
        "direction" => format!(
            "direction:Some(({},{})),",
            value.starts_with("alternate"),
            value.ends_with("reverse")
        ),
        _ => format!("paused:Some({}),", value == "paused"),
    };
    Ok(("animation_override".into(), field))
}
/// Code d'une animation de la liste.
fn member(frames: &str, value: impl Fn(&str) -> String) -> Result<String> {
    let duration = time(&value("duration"))?;
    let delay = time(&value("delay"))?;
    let ease = easing(&value("timing-function"))?;
    let mut code = format!(
        "{ROOT}::Animation::keyframes(const {{ {frames} }}).duration({duration}).delay({}).easing({ease})",
        delay.max(0)
    );
    if delay < 0 {
        // Délai négatif : l'animation démarre déjà avancée.
        code += &format!(".advance({})", -delay);
    }
    let count = value("iteration-count");
    if count == "infinite" {
        code += ".forever()";
    } else {
        // Entier vérifié à la déclaration (`2`, `2.0`).
        let n = values::number(&count)? as u32;
        if n == 0 {
            return Err("Zero animation iterations not implemented".into());
        }
        code += &format!(".repeat({n})");
    }
    code += match value("direction").as_str() {
        "normal" => "",
        "reverse" => ".reverse()",
        "alternate" => ".alternate()",
        "alternate-reverse" => ".alternate().reverse()",
        other => return Err(format!("Invalid animation-direction '{other}'")),
    };
    let fill = match value("fill-mode").as_str() {
        "none" => "None",
        "both" => "Both",
        "forwards" => "Forwards",
        "backwards" => "Backwards",
        _ => return Err("Invalid animation-fill-mode".into()),
    };
    code += &format!(".fill_mode({ROOT}::FillMode::{fill})");
    if value("play-state") == "paused" {
        code += ".paused(true)";
    }
    Ok(code)
}
pub(in crate::css) fn instance(
    props: &BTreeMap<String, String>,
    frames: &BTreeMap<String, String>,
) -> Result<String> {
    let lists = LONGHANDS
        .iter()
        .map(|(key, default)| {
            let value = props
                .get(&format!("animation-{key}"))
                .map_or(*default, String::as_str);
            list(value).map(|l| (*key, l))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    let mut members = Vec::new();
    for (index, name) in lists["name"].iter().enumerate() {
        if name == "none" {
            continue;
        }
        let frames = frames
            .get(name)
            .ok_or_else(|| format!("Unknown @keyframes '{name}'"))?;
        // Une liste plus courte que celle des noms se répète.
        let value = |key: &str| {
            let l = &lists[key];
            l[index % l.len()].clone()
        };
        members.push(member(frames, value)?);
    }
    Ok(match members.len() {
        0 => format!(
            "{ROOT}::Animation::from_to({ROOT}::Effect::NONE,{ROOT}::Effect::NONE).duration(0)"
        ),
        1 => members.pop().unwrap(),
        _ => format!(
            "{ROOT}::Animation::group(const {{ &[{}] }})",
            members.join(",")
        ),
    })
}
