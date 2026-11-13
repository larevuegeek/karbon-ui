//! `--dump layout.json`: every laid out element of the page with its bounds, classes,
//! text and computed typography, in the shape of the Tauri reference captures
//! (`x`, `y`, `w`, `h` in logical px, `path`, `text`, `font`, `ls`, `color`, `bg`,
//! `radius`). Compared with `tools/compare.py`.
use karbon_ui::ui::{
    element::Element,
    styling::{LetterSpacing, LineHeight},
};
use std::fmt::Write;

/// JSON array of the visible elements of `root`, in document order.
pub fn layout_json(root: &Element) -> String {
    let mut out = String::from("[\n");
    let mut first = true;
    walk(root, &mut Vec::new(), &mut out, &mut first);
    out.push_str("\n]\n");
    out
}

fn walk(node: &Element, path: &mut Vec<String>, out: &mut String, first: &mut bool) {
    let r = node.bounds();
    let (w, h) = (r.right - r.left, r.bottom - r.top);
    let classes = node.classes();
    let pushed = !classes.is_empty();
    if pushed {
        path.push(classes.join("."));
    }
    // Les boîtes vides (display:none, éléments non placés) ne sont pas visibles.
    if w > 0. && h > 0. {
        if !*first {
            out.push_str(",\n");
        }
        *first = false;
        // Texte propre d'une feuille ; un conteneur (bouton riche…) porte celui de ses
        // descendants dans `deep`, comme le texte d'un <button> de la référence.
        let text = if node.children.is_empty() {
            leaf_text(node)
        } else {
            String::new()
        };
        let mut deep = String::new();
        if !node.children.is_empty() {
            collect(node, &mut deep);
        }
        let shown = path.len().saturating_sub(4);
        let _ = write!(
            out,
            "{{\"id\": {}, \"path\": {}, \"cls\": {}, \"text\": {}, \"deep\": {}, \"x\": {}, \"y\": {}, \"w\": {}, \"h\": {}",
            string(&format!("{:?}", node.id)),
            string(&path[shown..].join(" > ")),
            string(&classes.join(" ")),
            string(&text),
            string(deep.trim()),
            round(r.left),
            round(r.top),
            round(w),
            round(h),
        );
        if let Some(style) = node.computed_style() {
            let t = style.text;
            if !text.is_empty() || !deep.is_empty() {
                let size = t.size.unwrap_or(16.);
                let line = match t.line_height {
                    Some(LineHeight::Px(px)) => format!("{}px", round(px)),
                    Some(LineHeight::Factor(f)) => format!("{}px", round(f * size)),
                    _ => "normal".into(),
                };
                let _ = write!(
                    out,
                    ", \"font\": {}",
                    string(&format!(
                        "{} {}px/{} \"{}\"",
                        t.weight.unwrap_or(400),
                        round(size),
                        line,
                        t.family.unwrap_or("")
                    ))
                );
                if let Some(ls) = t.letter_spacing {
                    let px = match ls {
                        LetterSpacing::Px(px) => px,
                        LetterSpacing::Em(_) => ls.resolve(size),
                    };
                    let _ = write!(out, ", \"ls\": \"{}px\"", round(px));
                }
                if let Some(c) = t.color {
                    let _ = write!(out, ", \"color\": {}", string(&rgba(c)));
                }
            }
            let local = &style.local;
            if let Some(c) = local.background.or(node.style.background) {
                let _ = write!(out, ", \"bg\": {}", string(&rgba(c)));
            }
            if let Some([a, b, c, d]) = local.corner_radii {
                let _ = write!(
                    out,
                    ", \"radius\": \"{}px {}px {}px {}px\"",
                    round(a),
                    round(b),
                    round(c),
                    round(d)
                );
            } else if let Some(radius) = local.radius.filter(|r| *r > 0.) {
                let _ = write!(out, ", \"radius\": \"{}px\"", round(radius));
            }
            if let (Some(width), Some(c)) = (local.border_width, local.border) {
                let _ = write!(
                    out,
                    ", \"border\": {}",
                    string(&format!("{}px {}", round(width), rgba(c)))
                );
            }
        }
        out.push('}');
    }
    for child in &node.children {
        walk(child, path, out, first);
    }
    if pushed {
        path.pop();
    }
}

fn leaf_text(node: &Element) -> String {
    node.widget
        .as_ref()
        .map(|widget| widget.semantics().name.to_owned())
        .unwrap_or_default()
}

/// Textes des feuilles visibles du sous-arbre, séparés par une espace (120 octets au plus).
fn collect(node: &Element, out: &mut String) {
    for child in &node.children {
        let r = child.bounds();
        if r.right - r.left <= 0. || r.bottom - r.top <= 0. || out.len() > 120 {
            continue;
        }
        if child.children.is_empty() {
            let text = leaf_text(child);
            if !text.is_empty() {
                out.push(' ');
                out.push_str(&text);
            }
        } else {
            collect(child, out);
        }
    }
}

fn round(v: f32) -> f32 {
    (v * 10.).round() / 10.
}

fn rgba(c: karbon_ui::ui::prelude::Color) -> String {
    if c.alpha == 255 {
        format!("rgb({}, {}, {})", c.red, c.green, c.blue)
    } else {
        format!(
            "rgba({}, {}, {}, {})",
            c.red,
            c.green,
            c.blue,
            round(f32::from(c.alpha) / 255.)
        )
    }
}

fn string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
