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
        if d.name == "animation" || d.name.starts_with("animation-") {
            return Err(format!(
                "{} needs @keyframes: declare it in a stylesheet class and use class=\"…\"",
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
