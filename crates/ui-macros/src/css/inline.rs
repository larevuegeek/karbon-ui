//! Attribut `style="…"` des templates : mêmes déclarations qu'une classe de
//! `css_file!`, compilées en appels sur le nœud (sans `@keyframes` ni variables).
use super::class::{base_display, display, emit};
use super::{parser, values};
use crate::codegen::Output;
use proc_macro::Span;
use std::collections::BTreeMap;
/// Compile les déclarations d'un attribut `style="…"` en appels sur `node`, avec la
/// même couverture qu'une classe de `css_file!` (sans `@keyframes` ni variables).
pub fn inline(
    text: &str,
    out: &mut Output,
    cx: &str,
    node: &str,
) -> std::result::Result<String, String> {
    let parsed = parser::parse(&format!(".inline{{{text}}}"))
        .map_err(|e| format!("Invalid style=\"…\" declarations: {}", e.message))?;
    let declarations = match parsed.into_iter().next() {
        Some(parser::Item::Rule { declarations, .. }) => declarations,
        _ => return Ok(String::new()),
    };
    let vars = BTreeMap::new();
    let mut props = Vec::new();
    let mut layout = None;
    let mut code = String::new();
    for d in &declarations {
        if d.name.starts_with("--") {
            // Propriété personnalisée de l'élément, héritée par ses descendants.
            let value = out
                .finish(&values::custom_value(&d.value)?)
                .map_err(|e| e.message)?;
            let value = out.value(&value);
            code += &format!("let {node}={node}.custom_property({:?},{value});", d.name);
            continue;
        }
        // Minutage en ligne d'une animation posée par les classes de l'élément
        // (`style="animation-delay: -1.4s"`).
        if let Some(call) = animation_longhand(&d.name, &d.value)? {
            code += &format!("let {node}={node}.{call};");
            continue;
        }
        if d.name == "animation" || d.name.starts_with("animation-") {
            return Err(format!(
                "{} needs @keyframes: declare it in a stylesheet class and use class=\"…\" (inline style accepts animation-delay, animation-duration and animation-play-state)",
                d.name
            ));
        }
        if d.name == "display" {
            let value = values::resolve(&d.value, &vars)?;
            match base_display(&value)? {
                Some(model) => {
                    layout = Some(model);
                    props.push(("hidden".into(), "false".into()));
                }
                None if value == "none" => props.push(("hidden".into(), "true".into())),
                // Un `display` visible annule le `display: none` des classes.
                None => props.push(("hidden".into(), "false".into())),
            }
            continue;
        }
        props.extend(values::declaration(&d.name, &d.value, &vars)?);
    }
    if let Some(value) = layout {
        display(&mut props, &value);
    }
    for (name, value) in props {
        let tokens = out.finish(&value).map_err(|e| e.message)?;
        code += &emit(name, tokens, out, cx, node, Span::call_site()).map_err(|e| e.message)?;
    }
    Ok(code)
}
/// Appel du nœud pour `animation-delay`, `animation-duration` ou `animation-play-state`
/// en ligne (une seule valeur), `None` pour une autre propriété.
fn animation_longhand(name: &str, value: &str) -> std::result::Result<Option<String>, String> {
    let value = value.trim();
    let time = |value: &str| -> std::result::Result<i64, String> {
        let ms = if let Some(n) = value.strip_suffix("ms") {
            values::number(n)?
        } else if let Some(n) = value.strip_suffix('s') {
            values::number(n)? * 1000.
        } else {
            return Err(format!("{name} needs ms or s: '{value}'"));
        };
        // `u32::MAX as f32` vaut 2^32 (arrondi) : la borne elle-même ne tient pas en `u32`.
        if ms.abs() >= u32::MAX as f32 {
            return Err(format!("Invalid {name} '{value}'"));
        }
        Ok(ms.round() as i64)
    };
    Ok(Some(match name {
        "animation-delay" => format!("animation_delay({}i64)", time(value)?),
        "animation-duration" => {
            let ms = time(value)?;
            if ms < 0 {
                return Err("animation-duration cannot be negative".into());
            }
            format!("animation_duration({ms}u32)")
        }
        "animation-play-state" => match value {
            "paused" => "animation_paused(true)".into(),
            "running" => "animation_paused(false)".into(),
            _ => {
                return Err(format!(
                    "Invalid animation-play-state '{value}' (running, paused)"
                ));
            }
        },
        _ => return Ok(None),
    }))
}

#[cfg(test)]
mod tests {
    /// 2^32 ms passait la borne (`u32::MAX as f32` vaut 2^32) et devenait un littéral
    /// `u32` hors plage dans le code généré.
    #[test]
    fn an_animation_duration_beyond_u32_is_refused() {
        let call = |value| super::animation_longhand("animation-duration", value);
        assert_eq!(
            call("4294967040ms").unwrap().as_deref(),
            Some("animation_duration(4294967040u32)")
        );
        assert!(call("4294967296ms").is_err());
    }
}
