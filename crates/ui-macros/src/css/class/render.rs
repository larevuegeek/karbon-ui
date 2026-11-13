//! Rendu d'une classe compilée en fonction de style : appels de nœud des propriétés,
//! effets de survol et d'appui, règles d'état, d'enfants, pseudo-éléments et animation.
use super::{Class, display, state_properties};
use crate::css::animation;
use crate::{
    Error, Result,
    codegen::{self, Output, StyleProp},
};
use proc_macro::{Span, TokenStream};
use std::collections::{BTreeMap, BTreeSet};
/// Propriétés propres au CSS compilé, traduites ici en appels de nœud ; les autres
/// passent par la traduction commune des templates (`codegen::property`).
pub(in crate::css) fn emit(
    name: String,
    value: TokenStream,
    out: &mut Output,
    cx: &str,
    node: &str,
    span: Span,
) -> Result<String> {
    let direct = match name.as_str() {
        "letter_spacing" => Some(format!(".letter_spacing(({{}}).scaled({cx}.scale))")),
        "text_transform" | "white_space" | "text_overflow" | "line_clamp" | "font_italic"
        | "direction_rtl" | "tabular_numbers" => Some(format!(".{name}({{}})")),
        "custom_property" => Some(".custom_properties(const{&[{}]})".into()),
        "cursor"
        | "pointer_events"
        | "hidden"
        | "grid_rows"
        | "order"
        | "aspect_ratio"
        | "inline_level"
        | "justify_self"
        | "justify_items"
        | "outline_color"
        | "mask_image"
        | "text_clip"
        | "container_type"
        | "container_name"
        | "scrollbar_width"
        | "grid_auto_repeat"
        | "overscroll_contain"
        | "scroll_initial_target"
        | "content_visibility"
        | "scrollbar_color"
        | "scrollbar_gutter"
        | "reveal" => Some(format!(".{name}({{}})")),
        "backdrop" => Some(format!(".backdrop_filter({cx}.filters({{}}))")),
        "contain_intrinsic_size" => Some(format!(
            ".contain_intrinsic_size({{let (i,a,n):(usize,bool,Option<f32>)={{}};(i,__ui_root::ui::element::ContainIntrinsicLength{{remembered:a,length:n.map(|n|n*{cx}.scale)}})}})"
        )),
        "decoration_color" => Some(".decoration_color({})".into()),
        "scroll_margin_edge" | "scroll_padding_edge" => Some(format!(
            ".{name}({{let (i,n):(usize,f32)={{}};(i,n*{cx}.scale)}})"
        )),
        "underline_offset" | "decoration_thickness" => {
            Some(format!(".{name}(({{}} as f32)*{cx}.scale)"))
        }
        "relative_limit" => Some(format!(
            ".relative_limit({{let (i,l):(usize,__ui_root::ui::element::Length)={{}};(i,{cx}.length(l))}})"
        )),
        "spacing_expression" => Some(format!(
            ".spacing_expression({{let (i,l):(usize,__ui_root::ui::layout::ComputedLength)={{}};(i,{cx}.computed_length(l))}})"
        )),
        "outline_width" | "outline_offset" | "scrollbar_size" => {
            Some(format!(".{name}(({{}} as f32)*{cx}.scale)"))
        }
        "border_edge_top" | "border_edge_right" | "border_edge_bottom" | "border_edge_left" => {
            Some(format!(
                ".border_edge({},({{}} as f32)*{cx}.scale)",
                side_index(&name)
            ))
        }
        "border_color_top" | "border_color_right" | "border_color_bottom" | "border_color_left" => {
            Some(format!(".border_edge_color({},{{}})", side_index(&name)))
        }
        "color_expression_text"
        | "color_expression_background"
        | "color_expression_border"
        | "color_expression_accent" => Some(format!(
            ".color_expression(__ui_root::ui::styling::ColorTarget::{},{{}})",
            color_target(&name)
        )),
        _ => None,
    };
    if let Some(call) = direct {
        let expr = out.value(&value);
        return Ok(format!("let {node}={node}{};", call.replace("{}", &expr)));
    }
    codegen::property(
        &StyleProp {
            name,
            value,
            span,
            expression: false,
        },
        out,
        cx,
        node,
    )
}
pub(super) fn side_index(name: &str) -> usize {
    match name.rsplit('_').next() {
        Some("top") => 0,
        Some("right") => 1,
        Some("bottom") => 2,
        _ => 3,
    }
}
pub(super) fn color_target(name: &str) -> &'static str {
    match name.trim_start_matches("color_expression_") {
        "text" => "Text",
        "background" => "Background",
        "border" => "Border",
        _ => "Accent",
    }
}
/// Fonction de style `name` de la cascade d'un élément de framework.
pub(crate) fn render_framework_class(
    name: &str,
    class: Class,
    animations: &BTreeMap<String, String>,
    out: &mut Output,
    literals: &Output,
    span: Span,
) -> Result<String> {
    render_class(name, class, animations, out, literals, span)
}
/// Corps de la fonction de style `name` d'une classe compilée.
pub(in crate::css) fn render_class(
    name: &str,
    mut class: Class,
    animations: &BTreeMap<String, String>,
    out: &mut Output,
    literals: &Output,
    span: Span,
) -> Result<String> {
    if let Some(value) = class.display.take() {
        display(&mut class.props, &value);
    }
    let mut bounds = BTreeSet::new();
    if class.state_props.iter().any(|(_, props, _)| {
        props
            .iter()
            .any(|(name, _)| ["flex_direction", "columns", "grid_tracks"].contains(&name.as_str()))
    }) {
        bounds.insert("__ui_root::ui::template::ContainerStyle");
    }
    let mut properties = String::new();
    for name in std::iter::once(&class.original)
        .chain(&class.markers)
        .filter(|n| !n.is_empty())
    {
        properties += &format!("let __ui_node=__ui_node.css_class({name:?});");
    }
    properties += "let __ui_node=__ui_node.style_context(cx);";
    properties += &class.prelude;
    for (name, value) in class.props {
        if matches!(
            codegen::canonical(&name),
            "flex_direction" | "columns" | "grid_tracks"
        ) {
            bounds.insert("__ui_root::ui::template::ContainerStyle");
        }
        let tokens = literals.finish(&value)?;
        properties += &emit(name, tokens, out, "cx", "__ui_node", span)?;
    }
    if !class.view_rules.is_empty() {
        properties += "let __ui_scheme=__ui_root::ui::styling::color_scheme();";
    }
    for (test, props) in class.view_rules {
        let mut inner = String::new();
        for (name, value) in props {
            let tokens = literals.finish(&value)?;
            inner += &emit(name, tokens, out, "cx", "__ui_node", span)?;
        }
        properties += &format!("let __ui_node=if {test}{{{inner}__ui_node}}else{{__ui_node}};");
    }
    for (name, states) in [("hover", class.hover), ("pressed", class.active)] {
        if states.is_empty() {
            continue;
        }
        let mut effect = states
            .get("transform")
            .cloned()
            .unwrap_or_else(|| "__ui_root::ui::animation::Effect::NONE".into());
        if let Some(opacity) = states.get("opacity") {
            effect += &format!(".opacity({opacity})");
        }
        let value = literals.finish(&effect)?;
        let value = out.value(&value);
        properties += &format!(
            "let __ui_node=__ui_node.css_{name}(cx.effect({value}),{},{});",
            states.contains_key("opacity"),
            states.contains_key("transform")
        );
    }
    for (state, props, order) in class.state_props {
        let value = state_properties(&props).map_err(|m| Error::new(m, span))?;
        let value = literals.finish(&value)?;
        let value = out.value(&value);
        properties += &format!("let __ui_node=__ui_node.state_rule({state},{value},{order});");
    }
    // Règles des enfants (`space-y-*`, `divide-y`).
    for (state, props, order) in class.child_props {
        let value = state_properties(&props).map_err(|m| Error::new(m, span))?;
        let value = literals.finish(&value)?;
        let value = out.value(&value);
        properties += &format!("let __ui_node=__ui_node.child_rule({state},{value},{order});");
    }
    // Pseudo-éléments : boîte générée, premier ou dernier enfant de l'hôte.
    for (kind, mut pseudo) in std::mem::take(&mut class.pseudos) {
        let Some(content) = pseudo.content else {
            continue;
        };
        if let Some(model) = &pseudo.display {
            display(&mut pseudo.props, model);
        }
        let children = if content.is_empty() {
            "::std::vec::Vec::new()".to_string()
        } else {
            format!(
                "::std::vec![__ui_root::ui::element::Element::custom(__ui_pid.child(\"content\").0,__ui_root::ui::widgets::Text::new({content:?}))]"
            )
        };
        let mut inner = format!(
            "let __ui_pid=__ui_node.element_id().child(\"::{kind}\");let __ui_pseudo={{let __ui_node=__ui_root::ui::template::Node::element(__ui_root::ui::element::Element::column(__ui_pid.0,{children})).style_context(cx);"
        );
        for (name, value) in pseudo.props {
            let tokens = literals.finish(&value)?;
            inner += &emit(name, tokens, out, "cx", "__ui_node", span)?;
        }
        for (state, props, order) in pseudo.state_props {
            let value = state_properties(&props).map_err(|m| Error::new(m, span))?;
            let value = literals.finish(&value)?;
            let value = out.value(&value);
            inner += &format!("let __ui_node=__ui_node.state_rule({state},{value},{order});");
        }
        properties += &format!(
            "{inner}__ui_node.build()}};let __ui_node=__ui_node.pseudo_element({},__ui_pseudo);",
            kind == "after"
        );
    }
    if !class.animation.is_empty() {
        let code =
            animation::instance(&class.animation, animations).map_err(|m| Error::new(m, span))?;
        let tokens = literals.finish(&code)?;
        let expr = out.value(&tokens);
        properties += &format!("let __ui_node=__ui_node.animate(cx.animation({expr}));");
    }
    let bounds = bounds.iter().map(|b| format!("+{b}")).collect::<String>();
    Ok(format!(
        "#[allow(non_snake_case,unused_variables)] pub fn {name}<R,W:__ui_root::ui::element::Widget+'static {bounds}>(__ui_node:__ui_root::ui::template::Node<W>,cx:&__ui_root::ui::template::TemplateContext<'_,R>)->__ui_root::ui::template::Node<W>{{{properties}__ui_node}}"
    ))
}
