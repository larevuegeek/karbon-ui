//! Parcours de l'arbre `<svg>` : groupes, formes et attributs de présentation
//! accumulés en couches du `SvgImage`.
use super::{
    SVG,
    paint::{Context, Paint, f, literal, paint_code, presentation},
    path::{Op, shape_ops},
    transform::apply,
};
use crate::{
    Result,
    template::markup::{Attr, Element, Loc, Node},
};
pub(super) struct Builder {
    pub(super) shapes: Vec<String>,
    pub(super) title: Option<String>,
}
impl Builder {
    pub(super) fn children(&mut self, nodes: &[Node], context: Context, root: bool) -> Result<()> {
        for node in nodes {
            match node {
                Node::Text(t, loc) => {
                    if !t.trim().is_empty() {
                        return Err(loc.error("Text is not allowed inside inline SVG"));
                    }
                }
                Node::Element(e) => self.element(e, context, root)?,
                other => {
                    let loc = other.loc().unwrap_or(Loc {
                        span: proc_macro::Span::call_site(),
                        line: 0,
                        col: 0,
                        file: 0,
                    });
                    return Err(loc.error(
                        "Inline SVG is compiled at build time: {expressions}, {#if} and {#each} are not allowed inside <svg>",
                    ));
                }
            }
        }
        Ok(())
    }
    fn element(&mut self, e: &Element, context: Context, root: bool) -> Result<()> {
        match e.name.as_str() {
            "g" => {
                let context = presentation(e, context, &[])?;
                self.children(&e.children, context, false)
            }
            "title" if root => {
                let mut text = String::new();
                for child in &e.children {
                    match child {
                        Node::Text(t, _) => text.push_str(t),
                        _ => return Err(e.loc.error("<title> inside <svg> accepts text only")),
                    }
                }
                self.title = Some(text.split_whitespace().collect::<Vec<_>>().join(" "));
                Ok(())
            }
            // Description non rendue.
            "desc" | "metadata" => Ok(()),
            "path" | "circle" | "ellipse" | "rect" | "line" | "polyline" | "polygon" => {
                if let Some(child) = e
                    .children
                    .iter()
                    .find(|c| !matches!(c, Node::Text(t, _) if t.trim().is_empty()))
                {
                    let loc = child.loc().unwrap_or(e.loc);
                    return Err(
                        loc.error(format!("<{}> cannot have children in inline SVG", e.name))
                    );
                }
                let (ops, extra) = shape_ops(e)?;
                let context = presentation(e, context, extra)?;
                self.shape(e, ops, context)
            }
            "svg" => Err(e.loc.error("Nested <svg> elements are not supported")),
            other => {
                let hint = match other {
                    "defs" | "use" | "symbol" => " Inline the shapes instead of referencing them.",
                    "linearGradient" | "radialGradient" | "pattern" => {
                        " SVG paint servers are not supported; use a colour or currentColor."
                    }
                    "text" | "tspan" => " Draw text with HTML elements next to the <svg>.",
                    "clipPath" | "mask" | "filter" => {
                        " Clipping, masks and filters inside SVG are not supported."
                    }
                    _ => "",
                };
                Err(e.loc.error(format!(
                    "<{other}> is not supported inside inline SVG (supported: path, circle, ellipse, rect, line, polyline, polygon, g, title).{hint}"
                )))
            }
        }
    }
    fn shape(&mut self, e: &Element, ops: Vec<Op>, context: Context) -> Result<()> {
        if ops.is_empty() {
            return Ok(());
        }
        let m = context.transform;
        let scale = (m[0] * m[3] - m[1] * m[2]).abs().sqrt();
        let mut code = String::new();
        for op in ops {
            let p = |x, y| {
                let (x, y) = apply(m, (x, y));
                format!("{},{}", f(x), f(y))
            };
            code += &match op {
                Op::Move(x, y) => format!("{SVG}::PathOp::MoveTo({}),", p(x, y)),
                Op::Line(x, y) => format!("{SVG}::PathOp::LineTo({}),", p(x, y)),
                Op::Cubic(a, b, c, d, x, y) => format!(
                    "{SVG}::PathOp::CubicTo({},{},{}),",
                    p(a, b),
                    p(c, d),
                    p(x, y)
                ),
                Op::Close => format!("{SVG}::PathOp::Close,"),
            };
        }
        // Une ligne n'a pas d'intérieur.
        let fill = if e.name == "line" {
            Paint::None
        } else {
            context.fill
        };
        if fill == Paint::None && context.stroke == Paint::None {
            return Ok(());
        }
        if self.shapes.len() >= 1024 {
            return Err(e.loc.error("Inline SVG exceeds 1024 shapes"));
        }
        self.shapes.push(format!(
            "{SVG}::SvgShape{{ops:&[{code}],fill:{},stroke:{},stroke_width:{},line_cap:{SVG}::LineCap::{},line_join:{SVG}::LineJoin::{},miter_limit:{},fill_rule:{SVG}::FillRule::{},fill_opacity:{},stroke_opacity:{}}}",
            paint_code(fill),
            paint_code(context.stroke),
            f(context.stroke_width * scale),
            context.cap,
            context.join,
            f(context.miter),
            if context.even_odd { "EvenOdd" } else { "NonZero" },
            f((context.fill_opacity * context.opacity).clamp(0., 1.)),
            f((context.stroke_opacity * context.opacity).clamp(0., 1.)),
        ));
        Ok(())
    }
}
/// Taille CSS d'un attribut `width`/`height` de la racine : nombre → px, sinon la
/// valeur telle quelle (`1em`, `100%`).
pub(super) fn css_length(a: &Attr) -> Result<(String, Option<f32>)> {
    let text = literal(a)?;
    if let Ok(n) = text
        .strip_suffix("px")
        .unwrap_or(text)
        .trim()
        .parse::<f64>()
    {
        if !n.is_finite() || n < 0. {
            return Err(a.loc.error(format!("Invalid SVG {}=\"{text}\"", a.name)));
        }
        return Ok((format!("{n}px"), Some(n as f32)));
    }
    if text.ends_with("em") || text.ends_with('%') || text.ends_with("rem") {
        return Ok((text.to_owned(), None));
    }
    Err(a.loc.error(format!(
        "Invalid SVG {}=\"{text}\": use a number, px, em, rem or %",
        a.name
    )))
}
