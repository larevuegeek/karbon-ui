//! Traduction des propriétés de style (`nom: valeur`) en appels de nœud, partagée par
//! les templates (`style:nom={…}`), `styles!` et `css_file!`.
use super::Output;
use crate::{Error, Result, input::files};
use proc_macro::{Delimiter, Span, TokenStream, TokenTree};

/// Propriété de style à traduire en appel de nœud (nom canonique et valeur Rust).
#[derive(Clone)]
pub struct StyleProp {
    pub name: String,
    pub value: TokenStream,
    pub span: Span,
}
pub fn canonical(name: &str) -> &str {
    match name {
        "border_radius" => "radius",
        "position" => "css_position",
        "font_weight" => "weight",
        "align_items" => "align",
        "justify_content" => "justify",
        "transform_origin" => "origin",
        other => other,
    }
}
pub fn property(a: &StyleProp, out: &mut Output, cx: &str, node: &str) -> Result<String> {
    let expr = out.value(&a.value);
    let name = canonical(&a.name);
    let call = match name {
        "width" | "height" => {
            let value = keyword(&a.value)
                .and_then(|s| match s.as_str() {
                    "fill" => Some("__ui_root::ui::view::FILL"),
                    "auto" => Some("__ui_root::ui::view::AUTO"),
                    _ => None,
                })
                .map_or_else(|| format!("{cx}.length({expr})"), str::to_owned);
            format!(".{name}({value})")
        }
        "padding" | "margin" => format!(".{name}({cx}.insets({expr}))"),
        "css_margin" => format!(".css_margin(({expr}).map(|v:Option<f32>|v.map(|n|n*{cx}.scale)))"),
        "css_margin_top" | "css_margin_right" | "css_margin_bottom" | "css_margin_left" => {
            let edge = match name {
                "css_margin_top" => 0,
                "css_margin_right" => 1,
                "css_margin_bottom" => 2,
                _ => 3,
            };
            // `as` ne type pas un littéral dans `Some(10.0)` (f64 par défaut) : on
            // annote la valeur, comme les autres propriétés optionnelles.
            format!(
                ".css_margin_edge({edge},::core::convert::identity::<Option<f32>>({expr}).map(|n|n*{cx}.scale))"
            )
        }
        "flex_basis" => format!(".flex_basis({cx}.length({expr}))"),
        "css_position" => format!(".position({expr})"),
        "z_index" => format!(".z_index({expr})"),
        "offset_top" | "offset_right" | "offset_bottom" | "offset_left" => {
            let edge = match name {
                "offset_top" => 0,
                "offset_right" => 1,
                "offset_bottom" => 2,
                _ => 3,
            };
            format!(".offset({edge},{cx}.length({expr}))")
        }
        "overflow" => format!(".overflow({expr},{expr})"),
        "overflow_x" | "overflow_y" => format!(".overflow_axis({}, {expr})", name == "overflow_x"),
        "flex_grow" | "flex_shrink" | "box_sizing" | "css_flex" | "flex_wrap" | "flex_reverse"
        | "align_self" => format!(".{name}({expr})"),
        "padding_top" | "padding_right" | "padding_bottom" | "padding_left" | "margin_top"
        | "margin_right" | "margin_bottom" | "margin_left" => {
            let (kind, edge) = name.split_once('_').unwrap();
            format!(
                ".inset_edge({},{edge:?},(({expr}) as f32)*{cx}.scale)",
                kind == "margin"
            )
        }
        "columns" | "grid_tracks" => format!(".{name}({expr})"),
        "corner_radii" => format!(".corner_radii(({expr}).map(|n:f32|n*{cx}.scale))"),
        "corner_top_left" | "corner_top_right" | "corner_bottom_right" | "corner_bottom_left" => {
            let index = match name {
                "corner_top_left" => 0,
                "corner_top_right" => 1,
                "corner_bottom_right" => 2,
                _ => 3,
            };
            format!(".corner_radius({index},(({expr}) as f32)*{cx}.scale)")
        }
        "flex_direction" => format!(".flex_direction({})", axis(Some(a), out)?),
        "gap" | "radius" | "min_width" | "min_height" | "max_width" | "max_height"
        | "font_size" | "border_width" | "row_gap" | "column_gap" => {
            format!(".{name}((({expr}) as f32)*{cx}.scale)")
        }
        "align" | "justify" => {
            let value = if let Some(k) = keyword(&a.value) {
                let variant = match (name, k.as_str()) {
                    ("align", "start" | "flex-start") => "Start",
                    ("align", "center") => "Center",
                    ("align", "end" | "flex-end") => "End",
                    ("align", "stretch") => "Stretch",
                    ("align", "baseline" | "first baseline") => "Baseline",
                    ("justify", "start" | "flex-start") => "Start",
                    ("justify", "center") => "Center",
                    ("justify", "end" | "flex-end") => "End",
                    ("justify", "between" | "space_between" | "space-between") => "SpaceBetween",
                    _ => return Err(Error::new(format!("Unknown {name} value '{k}'"), a.span)),
                };
                let ty = if name == "align" { "Align" } else { "Justify" };
                format!("__ui_root::ui::element::{ty}::{variant}")
            } else {
                expr
            };
            format!(".{name}({value})")
        }
        "grow" => {
            return Ok(format!(
                "let {node}=if {expr}{{{node}.width(__ui_root::ui::view::FILL).height(__ui_root::ui::view::FILL)}}else{{{node}}};"
            ));
        }
        "weight" => format!(".weight({expr})"),
        "span" | "row" | "column" => format!(".{name}({expr})"),
        "opacity" | "rotate" | "scale" => format!(".{name}(({expr}) as f32)"),
        "translate" => format!(".translate({cx}.translation({expr}))"),
        "effect" | "hover" | "pressed" | "transform" => format!(".{name}({cx}.effect({expr}))"),
        "animate" => format!(".animate({cx}.animation({expr}))"),
        "color" | "background" | "background_color" | "border_color" | "accent_color" => {
            let value = if let Some(value) = keyword(&a.value) {
                if value.starts_with('#') {
                    let color = color(&value, a.span)?;
                    format!("__ui_root::ui::theme::Color::hex({color})")
                } else if value == "transparent" {
                    "__ui_root::ui::theme::Color::rgba(0,0,0,0)".into()
                } else {
                    expr
                }
            } else {
                expr
            };
            format!(".{name}({value})")
        }
        "scroll_initial_target" => format!(".scroll_initial_target({expr})"),
        "line_height" => format!(".line_height({cx}.line_height({expr}))"),
        "box_shadows" => format!(".box_shadows({cx}.shadows({expr}))"),
        "filters" => format!(".filters({cx}.filters({expr}))"),
        "font_size_expression" => format!(".font_size_expression({cx}.computed_length({expr}))"),
        "text_align" | "text_decoration" | "image_fit" | "image_position" => {
            format!(".{name}({expr})")
        }
        "font" | "font_family" | "background_image" | "background_layers" | "image_layers"
        | "transition" | "origin" | "tooltip" | "painter" | "style" | "on_click" => {
            format!(".{name}({expr})")
        }
        "extends" => {
            return Ok(classes(&a.value)
                .iter()
                .map(|class| {
                    let class = out.value(class);
                    format!("let {node}=__ui_root::ui::template::style({node},{cx},{class});")
                })
                .collect());
        }
        _ => {
            return Err(Error::new(
                format!("Unknown style property '{name}'"),
                a.span,
            ));
        }
    };
    Ok(format!("let {node}={node}{call};"))
}
fn color(value: &str, span: Span) -> Result<u32> {
    let hex = &value[1..];
    if hex.len() != 6 {
        return Err(Error::new("Use #RRGGBB or a Rust Color expression", span));
    }
    u32::from_str_radix(hex, 16).map_err(|_| Error::new("Invalid hexadecimal color", span))
}
fn keyword(value: &TokenStream) -> Option<String> {
    let tokens = value.clone().into_iter().collect::<Vec<_>>();
    match tokens.as_slice() {
        [TokenTree::Literal(l)] => files::string(&l.to_string()).ok(),
        [TokenTree::Ident(i)] if !["true", "false"].contains(&i.to_string().as_str()) => {
            Some(i.to_string())
        }
        _ => None,
    }
}
fn axis(a: Option<&StyleProp>, out: &mut Output) -> Result<String> {
    match a {
        None => Ok("__ui_root::ui::element::Axis::Vertical".into()),
        Some(a) => {
            if let Some(value) = keyword(&a.value) {
                let v = match value.as_str() {
                    "vertical" | "column" => "Vertical",
                    "horizontal" | "row" => "Horizontal",
                    _ => {
                        return Err(Error::new(
                            "Axis must be horizontal, vertical, or a Rust expression",
                            a.span,
                        ));
                    }
                };
                Ok(format!("__ui_root::ui::element::Axis::{v}"))
            } else {
                Ok(out.value(&a.value))
            }
        }
    }
}
pub fn classes(value: &TokenStream) -> Vec<TokenStream> {
    let tokens = value.clone().into_iter().collect::<Vec<_>>();
    if let [TokenTree::Group(g)] = tokens.as_slice()
        && g.delimiter() == Delimiter::Parenthesis
    {
        return classes(&g.stream());
    }
    if let [TokenTree::Group(g)] = tokens.as_slice()
        && g.delimiter() == Delimiter::Bracket
    {
        let mut result = Vec::new();
        let mut current = TokenStream::new();
        for token in g.stream() {
            if matches!(&token,TokenTree::Punct(p) if p.as_char()==',') {
                if !current.is_empty() {
                    result.push(std::mem::take(&mut current));
                }
            } else {
                current.extend([token]);
            }
        }
        if !current.is_empty() {
            result.push(current);
        }
        result
    } else {
        vec![value.clone()]
    }
}
