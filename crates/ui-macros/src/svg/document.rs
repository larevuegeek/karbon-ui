pub(super) use super::builder::{Builder, css_length};
pub(super) use super::paint::{Context, f, literal, presentation};
pub(super) use super::scanner::numbers;
pub(super) use crate::{Result, template::markup::Element};

pub(super) const SVG: &str = "__ui_root::ui::rendering::svg";
/// Attributs propres à la racine `<svg>` (en plus des attributs communs et de
/// présentation).
pub const ROOT_ATTRIBUTES: &[&str] = &[
    "viewBox",
    "width",
    "height",
    "preserveAspectRatio",
    "xmlns",
    "xmlns:xlink",
    "version",
    "role",
    "focusable",
    "src",
];
/// Attributs de présentation, hérités par les descendants.
pub const PRESENTATION: &[&str] = &[
    "fill",
    "stroke",
    "stroke-width",
    "stroke-linecap",
    "stroke-linejoin",
    "stroke-miterlimit",
    "fill-rule",
    "clip-rule",
    "fill-opacity",
    "stroke-opacity",
    "opacity",
    "transform",
];

/// Dessin compilé : code de l'expression `&'static SvgImage`, taille naturelle et nom
/// accessible tiré de `<title>`.
pub struct Compiled {
    pub image: String,
    pub width: String,
    pub height: String,
    pub intrinsic: (f32, f32),
    pub title: Option<String>,
}

/// Compile la racine `<svg>` (attributs déjà validés par l'appelant).
pub fn compile(e: &Element) -> Result<Compiled> {
    let view_box = match e.attr("viewBox") {
        Some(a) => {
            let v = numbers(literal(a)?).map_err(|m| a.loc.error(m))?;
            if v.len() != 4 || v[2] <= 0. || v[3] <= 0. {
                return Err(a.loc.error(
                    "viewBox needs four numbers \"min-x min-y width height\" with a positive size",
                ));
            }
            Some([v[0], v[1], v[2], v[3]])
        }
        None => None,
    };
    let width = e.attr("width").map(css_length).transpose()?;
    let height = e.attr("height").map(css_length).transpose()?;
    // Sans viewBox, le repère est celui de la taille déclarée (en px).
    let view_box = match view_box {
        Some(v) => v,
        None => match (
            width.as_ref().and_then(|w| w.1),
            height.as_ref().and_then(|h| h.1),
        ) {
            (Some(w), Some(h)) if w > 0. && h > 0. => [0., 0., w as f64, h as f64],
            _ => {
                return Err(e.loc.error(
                    "<svg> needs a viewBox (or numeric width and height) to size its drawing",
                ));
            }
        },
    };
    let preserve = match e.attr("preserveAspectRatio") {
        None => true,
        Some(a) => match literal(a)? {
            "xMidYMid" | "xMidYMid meet" => true,
            "none" => false,
            v => {
                return Err(a.loc.error(format!(
                    "preserveAspectRatio=\"{v}\" is not supported; use the default (xMidYMid meet) or none"
                )));
            }
        },
    };
    if let Some(a) = e.attr("role")
        && !matches!(literal(a)?, "img" | "presentation" | "none")
    {
        return Err(a
            .loc
            .error("role on <svg> must be img, presentation or none"));
    }
    if let Some(a) = e.attr("transform") {
        return Err(a.loc.error(
            "transform on <svg> is not supported; use the CSS transform property or a <g transform>",
        ));
    }
    let context = presentation(e, Context::default(), ROOT_ATTRIBUTES)?;
    let mut builder = Builder {
        shapes: Vec::new(),
        title: None,
    };
    builder.children(&e.children, context, true)?;
    let [x, y, w, h] = view_box;
    let number = |n| f(n).map_err(|m| e.attr("viewBox").map_or(e.loc, |a| a.loc).error(m));
    // Les coordonnées d'un dessin peuvent ressembler à π ou e (3.14, 2.718) : ce ne sont
    // pas des constantes approchées, Clippy ne doit pas les signaler chez l'application.
    let image = format!(
        "{{#[allow(clippy::approx_constant,clippy::excessive_precision)] static __UI_SVG:{SVG}::SvgImage={SVG}::SvgImage{{view_box:[{},{},{},{}],preserve_aspect_ratio:{preserve},shapes:&[{}]}};&__UI_SVG}}",
        number(x)?,
        number(y)?,
        number(w)?,
        number(h)?,
        builder.shapes.join(",")
    );
    let (width_css, intrinsic_w) = width.unwrap_or_else(|| (format!("{w}px"), Some(w as f32)));
    let (height_css, intrinsic_h) = height.unwrap_or_else(|| (format!("{h}px"), Some(h as f32)));
    Ok(Compiled {
        image,
        width: width_css,
        height: height_css,
        intrinsic: (
            intrinsic_w.unwrap_or(w as f32),
            intrinsic_h.unwrap_or(h as f32),
        ),
        title: builder.title,
    })
}
/// Retire le prologue XML, le doctype et les commentaires de tête d'un fichier `.svg`.
pub fn strip_prolog(text: &str) -> &str {
    let mut rest = text.trim_start_matches('\u{feff}').trim_start();
    loop {
        if rest.starts_with("<?") {
            match rest.find("?>") {
                Some(end) => rest = rest[end + 2..].trim_start(),
                None => return rest,
            }
        } else if rest.starts_with("<!DOCTYPE") || rest.starts_with("<!doctype") {
            match rest.find('>') {
                Some(end) => rest = rest[end + 1..].trim_start(),
                None => return rest,
            }
        } else {
            return rest;
        }
    }
}
