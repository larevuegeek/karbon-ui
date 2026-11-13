//! Attributs de présentation hérités (remplissage, contour, opacités, épaisseurs) :
//! lecture des valeurs et code Rust des peintures.
use super::{
    SVG,
    transform::{IDENTITY, Matrix, multiply, transform},
};
use crate::{
    Result,
    template::markup::{Attr, Element, Value},
};
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Paint {
    None,
    Current,
    Color([f32; 4]),
}
/// Attributs de présentation en vigueur (hérités puis surchargés).
#[derive(Clone, Copy)]
pub(super) struct Context {
    pub(super) fill: Paint,
    pub(super) stroke: Paint,
    pub(super) stroke_width: f64,
    pub(super) cap: &'static str,
    pub(super) join: &'static str,
    pub(super) miter: f64,
    pub(super) even_odd: bool,
    pub(super) fill_opacity: f64,
    pub(super) stroke_opacity: f64,
    /// Produit des `opacity` des ancêtres (approximation : appliqué à chaque forme).
    pub(super) opacity: f64,
    pub(super) transform: Matrix,
}
impl Default for Context {
    fn default() -> Self {
        Self {
            fill: Paint::Color([0., 0., 0., 1.]),
            stroke: Paint::None,
            stroke_width: 1.,
            cap: "Butt",
            join: "Miter",
            miter: 4.,
            even_odd: false,
            fill_opacity: 1.,
            stroke_opacity: 1.,
            opacity: 1.,
            transform: IDENTITY,
        }
    }
}
pub(super) fn literal(a: &Attr) -> Result<&str> {
    match &a.value {
        Value::Text(t) => Ok(t.trim()),
        _ => Err(a.loc.error(format!(
            "SVG attribute '{}' must be a literal value; SVG geometry is compiled at build time",
            a.name
        ))),
    }
}
pub(super) fn number(a: &Attr) -> Result<f64> {
    let text = literal(a)?;
    let value = text.strip_suffix("px").unwrap_or(text).trim();
    value
        .parse::<f64>()
        .ok()
        .filter(|n| n.is_finite())
        .ok_or_else(|| {
            a.loc.error(format!(
                "Invalid number {}=\"{text}\" (SVG lengths are user units or px)",
                a.name
            ))
        })
}
pub(super) fn opacity(a: &Attr) -> Result<f64> {
    let text = literal(a)?;
    let value = match text.strip_suffix('%') {
        Some(p) => p.trim().parse::<f64>().map(|n| n / 100.),
        None => text.parse::<f64>(),
    };
    value
        .ok()
        .filter(|n| n.is_finite())
        .map(|n| n.clamp(0., 1.))
        .ok_or_else(|| {
            a.loc
                .error(format!("Invalid opacity {}=\"{text}\"", a.name))
        })
}
pub(super) fn paint(a: &Attr) -> Result<Paint> {
    let text = literal(a)?;
    if text == "none" || text == "transparent" {
        return Ok(Paint::None);
    }
    if text.eq_ignore_ascii_case("currentColor") {
        return Ok(Paint::Current);
    }
    if text.starts_with("url(") {
        return Err(a.loc.error(format!(
            "{}=\"{text}\": SVG gradients and patterns are not supported; use a colour or currentColor",
            a.name
        )));
    }
    match crate::css::svg_color(text) {
        Ok(Some(c)) => Ok(Paint::Color(c)),
        Ok(None) => Ok(Paint::Current),
        Err(m) => Err(a
            .loc
            .error(format!("Invalid SVG colour {}=\"{text}\": {m}", a.name))),
    }
}
/// Lit les attributs de présentation de `e` par-dessus `context`. Les attributs non
/// reconnus (hors `extra`) sont des erreurs.
pub(super) fn presentation(e: &Element, mut context: Context, extra: &[&str]) -> Result<Context> {
    let root = e.name == "svg";
    for a in &e.attrs {
        let name = a.name.as_str();
        // Attributs de la racine traités par le générateur commun (data-*, class:x…).
        if root
            && (name.starts_with("data-")
                || name.starts_with("class:")
                || name.starts_with("style:")
                || crate::template::html::EVENTS.contains(&name)
                || crate::template::html::COMMON.contains(&name))
        {
            continue;
        }
        match name {
            "fill" => context.fill = paint(a)?,
            "stroke" => context.stroke = paint(a)?,
            "stroke-width" => {
                let n = number(a)?;
                if n < 0. {
                    return Err(a.loc.error("stroke-width cannot be negative"));
                }
                context.stroke_width = n;
            }
            "stroke-linecap" => {
                context.cap = match literal(a)? {
                    "butt" => "Butt",
                    "round" => "Round",
                    "square" => "Square",
                    v => {
                        return Err(a.loc.error(format!(
                            "stroke-linecap=\"{v}\": expected butt, round or square"
                        )));
                    }
                }
            }
            "stroke-linejoin" => {
                context.join = match literal(a)? {
                    "miter" | "miter-clip" => "Miter",
                    "round" => "Round",
                    "bevel" => "Bevel",
                    v => {
                        return Err(a.loc.error(format!(
                            "stroke-linejoin=\"{v}\": expected miter, round or bevel"
                        )));
                    }
                }
            }
            "stroke-miterlimit" => {
                let n = number(a)?;
                if n < 1. {
                    return Err(a.loc.error("stroke-miterlimit must be at least 1"));
                }
                context.miter = n;
            }
            "fill-rule" => {
                context.even_odd = match literal(a)? {
                    "nonzero" => false,
                    "evenodd" => true,
                    v => {
                        return Err(a
                            .loc
                            .error(format!("fill-rule=\"{v}\": expected nonzero or evenodd")));
                    }
                }
            }
            // Sans effet hors d'un clipPath (non pris en charge) : accepté.
            "clip-rule" => {
                literal(a)?;
            }
            "fill-opacity" => context.fill_opacity = opacity(a)?,
            "stroke-opacity" => context.stroke_opacity = opacity(a)?,
            "opacity" => context.opacity *= opacity(a)?,
            "transform" => {
                let m = transform(literal(a)?).map_err(|m| a.loc.error(m))?;
                context.transform = multiply(context.transform, m);
            }
            _ if extra.contains(&name) => {}
            _ => {
                let hint = match name {
                    "class" | "id" | "style" => {
                        " CSS classes, ids and styles apply to the <svg> element only; use presentation attributes (fill, stroke…) on its shapes."
                    }
                    "stroke-dasharray" | "stroke-dashoffset" => {
                        " Dashed strokes are not supported."
                    }
                    "clip-path" | "mask" | "filter" => {
                        " Clipping, masks and filters inside SVG are not supported."
                    }
                    _ => "",
                };
                return Err(a.loc.error(format!(
                    "Unsupported attribute '{name}' on <{}> in inline SVG.{hint}",
                    e.name
                )));
            }
        }
    }
    Ok(context)
}
fn color_code(c: [f32; 4]) -> String {
    let q = |v: f32| (v.clamp(0., 1.) * 255.).round() as u8;
    format!(
        "{SVG}::SvgPaint::Color(__ui_root::ui::theme::Color::rgba({},{},{},{}))",
        q(c[0]),
        q(c[1]),
        q(c[2]),
        q(c[3])
    )
}
pub(super) fn paint_code(p: Paint) -> String {
    match p {
        Paint::None => format!("{SVG}::SvgPaint::None"),
        Paint::Current => format!("{SVG}::SvgPaint::CurrentColor"),
        Paint::Color(c) => color_code(c),
    }
}
/// Littéral `f32` du code généré ; erreur pour une valeur finie en `f64` mais hors de
/// la plage de `f32` (`1e39` deviendrait `inff32`, illisible pour rustc).
pub(super) fn f(n: f64) -> std::result::Result<String, String> {
    let rounded = ((n * 10_000.).round() / 10_000.) as f32;
    if !rounded.is_finite() {
        return Err(format!(
            "SVG number {n:e} is out of range (at most {:e})",
            f32::MAX
        ));
    }
    let rounded = if rounded == 0. { 0. } else { rounded };
    Ok(format!("{rounded:?}f32"))
}
