//! Déclaration complète : variables `var()` résolues, ou déclaration évaluée à la
//! résolution des styles quand elle dépend de variables d'élément.
use super::*;
fn keys(raw: &str, vars: &BTreeMap<String, String>) -> Result<Vec<String>> {
    let mut raw = raw.trim();
    let mut result = Vec::new();
    while let Some(inner) = raw.strip_prefix("var(").and_then(|s| s.strip_suffix(')')) {
        let args = split(inner, ',')?;
        let name = args[0].trim();
        if result.iter().any(|s| s == name) || result.len() > 32 {
            return Err("Circular CSS variable".into());
        }
        result.push(name.to_string());
        raw = vars
            .get(name)
            .map(String::as_str)
            .or_else(|| args.get(1).copied())
            .ok_or("Missing variable fallback")?
            .trim();
    }
    Ok(result)
}
pub fn declaration(name: &str, raw: &str, vars: &BTreeMap<String, String>) -> Result<Properties> {
    // Couleurs avec var() : évaluées à la résolution des styles, pour suivre les
    // propriétés héritées des ancêtres et les variables de l'application.
    if raw.contains("var(") {
        let field = match name {
            "color" => Some(("color", "text")),
            "background-color" => Some(("background_color", "background")),
            "border-color" => Some(("border_color", "border")),
            "accent-color" => Some(("accent_color", "accent")),
            "background" if !raw.contains("gradient(") && split(raw, ',')?.len() == 1 => {
                Some(("background", "background"))
            }
            _ => None,
        };
        if let Some((field, target)) = field {
            return super::properties::color_prop(field, target, raw.trim(), Some(vars));
        }
        if name == "border" {
            let parts = words(raw)?;
            if let [w, kind, c] = parts.as_slice()
                && kind == "solid"
                && !w.contains("var(")
            {
                let mut out = vec![("border_width".into(), format!("{:?}", px(w, false)?))];
                out.extend(super::properties::color_prop(
                    "border_color",
                    "border",
                    c,
                    Some(vars),
                )?);
                return Ok(out);
            }
        }
    }
    let resolved = resolve(raw, vars)?;
    // Dégradés et ombres : couleurs lues à la construction de la vue.
    let read_at_view = [
        "background",
        "background-image",
        "box-shadow",
        "border-top",
        "border-right",
        "border-bottom",
        "border-left",
        "border-top-color",
        "border-right-color",
        "border-bottom-color",
        "border-left-color",
    ]
    .contains(&name)
        && raw.contains("var(");
    let mut props = if read_at_view {
        view::with_vars(vars, || property(name, raw)).or_else(|_| property(name, &resolved))?
    } else {
        property(name, &resolved)?
    };
    if !raw.contains("var(") {
        return Ok(props);
    }
    // Espacement dont un côté est une expression : chaque mot en `var()` devient une
    // expression qui lit la variable à la résolution des styles.
    if props.iter().any(|(p, _)| p == "spacing_expression") {
        return super::properties::spacing_with_vars(name, raw, vars);
    }
    if name == "font-size" {
        return Ok(vec![(
            "font_size_expression".into(),
            expression::parse_with_vars(raw, vars)?,
        )]);
    }
    // Dimensions avec var() : expression évaluée au layout, dans la portée de l'élément
    // (propriétés héritées des ancêtres, puis variables de l'application).
    if [
        "width",
        "height",
        "flex-basis",
        "top",
        "right",
        "bottom",
        "left",
    ]
    .contains(&name)
        && let Ok(code) = expression::parse_with_vars(raw, vars)
    {
        props[0].1 = format!("{ROOT}::element::Length::Computed({code})");
        return Ok(props);
    }
    let keys = keys(raw, vars)?;
    if keys.is_empty() {
        if [
            "width",
            "height",
            "flex-basis",
            "top",
            "right",
            "bottom",
            "left",
        ]
        .contains(&name)
        {
            props[0].1 = format!(
                "{ROOT}::element::Length::Computed({})",
                expression::parse_with_vars(raw, vars)?
            );
        }
        return Ok(props);
    }
    for (property, value) in &mut props {
        let method = match property.as_str() {
            "padding" => "variable_insets",
            "css_margin" => "variable_margins",
            "css_margin_top" | "css_margin_right" | "css_margin_bottom" | "css_margin_left" => {
                "variable_margin"
            }
            "color" | "background" | "background_color" | "border_color" | "accent_color"
                if !value.contains("Gradient::") =>
            {
                "variable_color"
            }
            "width" | "height" | "flex_basis" | "offset_top" | "offset_right" | "offset_bottom"
            | "offset_left" => "variable_length",
            "font_family" => "variable_family",
            "opacity" | "font_size" | "min_width" | "min_height" | "max_width" | "max_height"
            | "border_width" | "radius" | "row_gap" | "column_gap" | "gap" | "flex_grow"
            | "flex_shrink" | "padding_top" | "padding_right" | "padding_bottom"
            | "padding_left" => "variable_number",
            "weight" => "variable_number",
            _ => continue,
        };
        if property == "weight" {
            *value = format!("({value} as f32)");
        }
        for name in keys.iter().rev() {
            *value = format!("cx.{method}({name:?},{value})");
        }
        if property == "weight" {
            *value = format!("({value} as u16)");
        }
    }
    Ok(props)
}
