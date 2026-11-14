//! Propriétés conditionnelles (états, requêtes, enfants) : structure `Properties` du
//! moteur, résolue à l'exécution avec le contexte de style.
use super::render::{color_target, side_index};
pub(super) fn state_properties(props: &[(String, String)]) -> std::result::Result<String, String> {
    let mut fields = String::new();
    let mut text = String::new();
    let mut layout = String::new();
    let mut expressions: [Option<&str>; 4] = [None; 4];
    let mut custom = Vec::new();
    // Réglages d'animation d'une règle d'état : un champ pour tous les longhands.
    let mut animation = Vec::new();
    let (mut transform, mut opacity) = (None, None);
    let mut edges: [Option<String>; 4] = Default::default();
    let mut edge_colors: [Option<String>; 4] = Default::default();
    let mut corners: [Option<String>; 4] = Default::default();
    // Limites relatives (`max-w-[40%]`) : un seul champ pour les quatre.
    let mut relative = Vec::new();
    // Espacements en expression (`padding: 1em`) : un champ pour tous les côtés.
    let mut spacing = Vec::new();
    for (name, value) in props {
        if let Some(side) = name.strip_prefix("border_edge_") {
            edges[side_index(side)] = Some(format!("({value} as f32)*cx.scale"));
            continue;
        }
        if let Some(side) = name.strip_prefix("border_color_") {
            edge_colors[side_index(side)] = Some(value.clone());
            continue;
        }
        if let Some(corner) = name.strip_prefix("corner_") {
            let index = match corner {
                "top_left" => 0,
                "top_right" => 1,
                "bottom_right" => 2,
                _ => 3,
            };
            corners[index] = Some(format!("({value} as f32)*cx.scale"));
            continue;
        }
        if name == "transform" {
            transform = Some(value.as_str());
            continue;
        }
        if name == "opacity" {
            opacity = Some(value.as_str());
            continue;
        }
        if name == "custom_property" {
            custom.push(value.as_str());
            continue;
        }
        if name == "animation_override" {
            animation.push(value.as_str());
            continue;
        }
        if name == "dynamic_declarations" {
            fields += &format!("dynamic:Some(const{{{value}}}),");
            continue;
        }
        if name.starts_with("color_expression_") {
            let index = match color_target(name) {
                "Text" => 0,
                "Background" => 1,
                "Border" => 2,
                _ => 3,
            };
            expressions[index] = Some(value);
            continue;
        }
        // Dégradés et ombres teintés par des variables : évalués à la résolution des
        // styles, donc possibles dans une règle d'état.
        if name == "background_expressions" {
            fields += &format!("background_expressions:Some({value}),");
            continue;
        }
        if name == "shadow_expressions" {
            fields += &format!("shadow_expressions:Some(cx.shadow_expressions({value})),");
            continue;
        }
        if name == "relative_limit" {
            relative.push(value.as_str());
            continue;
        }
        if name == "spacing_expression" {
            spacing.push(value.as_str());
            continue;
        }
        if let Some(value) = state_layout(name, value)? {
            layout += &value;
            continue;
        }
        if ["background", "background_color"].contains(&name.as_str())
            && value.contains("Gradient::")
        {
            return Err(
                "Gradient backgrounds in interactive states are not implemented yet".into(),
            );
        }
        let (field, value, is_text) = match name.as_str() {
            "filters" => ("filters", format!("cx.filters({value})"), false),
            "box_shadows" => ("shadows", format!("cx.shadows({value})"), false),
            "corner_radii" => (
                "corner_radii",
                format!("({value}).map(|n:f32|n*cx.scale)"),
                false,
            ),
            "image_fit" | "image_position" => (name.as_str(), value.clone(), false),
            "font_size_expression" => (
                "font_size_expression",
                format!("cx.computed_length({value})"),
                false,
            ),
            "font_size" => ("size", format!("({value} as f32)*cx.scale"), true),
            "weight" => ("weight", value.clone(), true),
            "font_family" => ("family", value.clone(), true),
            "letter_spacing" => (
                "letter_spacing",
                format!("({value}).scaled(cx.scale)"),
                true,
            ),
            "text_transform" => ("transform", value.clone(), true),
            "white_space" => ("white_space", value.clone(), true),
            "text_overflow" => ("overflow", value.clone(), true),
            "direction_rtl" => ("rtl", value.clone(), true),
            "line_clamp" => ("line_clamp", value.clone(), true),
            "font_italic" => ("italic", value.clone(), true),
            "tabular_numbers" => ("tabular_numbers", value.clone(), true),
            "color" => ("color", value.clone(), true),
            "text_align" => ("align", value.clone(), true),
            "line_height" => ("line_height", format!("cx.line_height({value})"), true),
            "text_decoration" => ("decoration", value.clone(), true),
            "decoration_color" => ("decoration_color", value.clone(), true),
            "underline_offset" | "decoration_thickness" => {
                (name.as_str(), format!("({value} as f32)*cx.scale"), true)
            }
            "background" | "background_color" => ("background", value.clone(), false),
            "border_color" => ("border", value.clone(), false),
            "border_width" => ("border_width", format!("({value} as f32)*cx.scale"), false),
            "radius" | "border_radius" => ("radius", format!("({value} as f32)*cx.scale"), false),
            "accent_color" => ("accent", value.clone(), false),
            "outline_width" => ("outline_width", format!("({value} as f32)*cx.scale"), false),
            "outline_offset" => (
                "outline_offset",
                format!("({value} as f32)*cx.scale"),
                false,
            ),
            "outline_color" => ("outline_color", value.clone(), false),
            "placeholder_color" => ("placeholder_color", value.clone(), false),
            "backdrop" => ("backdrop", format!("cx.filters({value})"), false),
            "text_clip" => ("text_clip", value.clone(), false),
            "cursor" => ("cursor", value.clone(), false),
            "padding" => ("padding", format!("cx.insets({value})"), false),
            // `motion-reduce:transition-none`, `.x:hover { transition: … }` : remplace la
            // transition de l'élément tant que la règle s'applique.
            "transition" => ("transition", value.clone(), false),
            _ => {
                return Err(format!(
                    "State property '{name}' has no native state resolver yet"
                ));
            }
        };
        let target = if is_text { &mut text } else { &mut fields };
        *target += &format!("{field}:Some({value}),");
    }
    if !custom.is_empty() {
        fields += &format!("custom:Some(const{{&[{}]}}),", custom.join(","));
    }
    if !animation.is_empty() {
        fields += &format!(
            "animation:__ui_root::ui::animation::AnimationOverride{{{}..Default::default()}},",
            animation.concat()
        );
    }
    let array = |values: &[Option<String>; 4]| {
        values
            .iter()
            .map(|v| v.as_ref().map_or("None".into(), |v| format!("Some({v})")))
            .collect::<Vec<_>>()
            .join(",")
    };
    if edges.iter().any(Option::is_some) {
        fields += &format!("border_edges:[{}],", array(&edges));
    }
    if !relative.is_empty() {
        layout += &format!(
            "relative_limits:{{let mut a=[None;4];{}a}},",
            relative
                .iter()
                .map(|v| format!(
                    "{{let (i,l):(usize,__ui_root::ui::element::Length)={v};a[i]=Some(cx.length(l));}}"
                ))
                .collect::<String>()
        );
    }
    if !spacing.is_empty() {
        layout += &format!(
            "spacing:{{let mut s=__ui_root::ui::styling::SpacingExpressions::default();{}s}},",
            spacing
                .iter()
                .map(|v| format!(
                    "{{let (i,l):(usize,__ui_root::ui::layout::ComputedLength)={v};s.set(i,cx.computed_length(l));}}"
                ))
                .collect::<String>()
        );
    }
    if edge_colors.iter().any(Option::is_some) {
        fields += &format!("border_colors:[{}],", array(&edge_colors));
    }
    if corners.iter().any(Option::is_some) {
        fields += &format!("corner_edges:[{}],", array(&corners));
    }
    if transform.is_some() || opacity.is_some() {
        fields += &format!(
            "effect:Some(__ui_root::ui::styling::StateEffect{{effect:cx.effect(({}).opacity({})),opacity:{},transform:{}}}),",
            transform.unwrap_or("__ui_root::ui::animation::Effect::NONE"),
            opacity.unwrap_or("1.0"),
            opacity.is_some(),
            transform.is_some()
        );
    }
    if expressions.iter().any(Option::is_some) {
        fields += &format!(
            "color_expressions:[{}],",
            expressions
                .iter()
                .map(|e| e.map_or("None".into(), |e| format!("Some({e})")))
                .collect::<Vec<_>>()
                .join(",")
        );
    }
    Ok(format!(
        "__ui_root::ui::styling::Properties{{{fields}layout:__ui_root::ui::styling::LayoutProperties{{{layout}..Default::default()}},text:__ui_root::ui::styling::Typography{{{text}..Default::default()}},..Default::default()}}"
    ))
}
fn state_layout(name: &str, value: &str) -> std::result::Result<Option<String>, String> {
    let prop = match name {
        "width" | "height" | "flex_basis" => format!("{name}:Some(cx.length({value})),"),
        "min_width" | "min_height" | "max_width" | "max_height" | "gap" | "row_gap"
        | "column_gap" => format!("{name}:Some(({value} as f32)*cx.scale),"),
        "css_margin" => format!("margin:Some(({value}).map(|v:Option<f32>|v.map(|n|n*cx.scale))),"),
        "padding_top" | "padding_right" | "padding_bottom" | "padding_left" => {
            let i = match name {
                "padding_top" => 0,
                "padding_right" => 1,
                "padding_bottom" => 2,
                _ => 3,
            };
            format!(
                "padding_edges:[{}],",
                (0..4)
                    .map(|j| if i == j {
                        format!("Some(({value} as f32)*cx.scale)")
                    } else {
                        "None".into()
                    })
                    .collect::<Vec<_>>()
                    .join(",")
            )
        }
        "css_margin_top" | "css_margin_right" | "css_margin_bottom" | "css_margin_left" => {
            let i = match name {
                "css_margin_top" => 0,
                "css_margin_right" => 1,
                "css_margin_bottom" => 2,
                _ => 3,
            };
            format!(
                "margin_edges:[{}],",
                (0..4)
                    .map(|j| if i == j {
                        format!("{{let v:Option<f32>={value};Some(v.map(|n|n*cx.scale))}}")
                    } else {
                        "None".into()
                    })
                    .collect::<Vec<_>>()
                    .join(",")
            )
        }
        "flex_direction" => format!(
            "axis:Some(__ui_root::ui::element::Axis::{}),",
            if value == "\"row\"" {
                "Horizontal"
            } else if value == "\"column\"" {
                "Vertical"
            } else {
                return Err("Invalid conditional flex direction".into());
            }
        ),
        "z_index" => format!("z_index:Some({value}),"),
        "align" | "justify" => {
            let variant = match value.trim_matches('"') {
                "start" | "flex-start" => "Start",
                "end" | "flex-end" => "End",
                "center" => "Center",
                "stretch" => "Stretch",
                "baseline" if name == "align" => "Baseline",
                "space-between" => "SpaceBetween",
                _ => return Err("Unsupported conditional alignment".into()),
            };
            format!(
                "{name}:Some(__ui_root::ui::element::{}::{variant}),",
                if name == "align" { "Align" } else { "Justify" }
            )
        }
        "flex_grow" | "flex_shrink" | "flex_wrap" | "flex_reverse" | "box_sizing" => {
            format!("{name}:Some({value}),")
        }
        "css_flex" => String::new(),
        "hidden" => format!("hidden:Some({value}),"),
        "display_block" => format!("block:Some({value}),"),
        "display_grid" => format!("display_grid:Some({value}),"),
        "order" => format!("order:Some({value}),"),
        "row" | "column" | "span" => format!("{name}:Some({value}),"),
        "grid_auto_repeat" => format!("grid_auto_repeat:Some({value}),"),
        "justify_self" => format!("justify_self:Some({value}),"),
        "aspect_ratio" => format!("aspect_ratio:Some({value}),"),
        "grid_tracks" | "grid_rows" => {
            // Pistes statiques : `Arc::from([...cx.px(n)...])` devient `const{&[...(n)...]}`.
            let inner = value
                .strip_prefix("::std::sync::Arc::from(")
                .and_then(|v| v.strip_suffix(')'))
                .ok_or("Invalid conditional grid tracks")?;
            if inner.contains("cx.") && !inner.contains("cx.px(") {
                return Err("Invalid conditional grid tracks".into());
            }
            format!(
                "{}:Some(const{{&{}}}),",
                if name == "grid_tracks" {
                    "grid_columns"
                } else {
                    "grid_rows"
                },
                inner.replace("cx.px(", "(")
            )
        }
        _ => return Ok(None),
    };
    Ok(Some(prop))
}
