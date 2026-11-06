//! Bordures : largeurs, couleurs et rayons des coins.
use super::super::{
    border::{border, sides},
    reader::{Evaluation, Reader},
    token::Token,
};
use super::{DynamicProperty, side};
use crate::ui::styling::Properties;

/// Applique une propriété de bordure ou de rayon.
pub(super) fn apply(
    property: DynamicProperty,
    tokens: &[Token],
    eval: &Evaluation,
    font: f32,
    r: &mut Reader,
    out: &mut Properties,
) {
    use DynamicProperty as P;
    match property {
        P::BorderTopLeftRadius
        | P::BorderTopRightRadius
        | P::BorderBottomRightRadius
        | P::BorderBottomLeftRadius => {
            let i = match property {
                P::BorderTopLeftRadius => 0,
                P::BorderTopRightRadius => 1,
                P::BorderBottomRightRadius => 2,
                _ => 3,
            };
            // `inherit` : le coin du parent (dernier élément d'une liste dans une carte).
            if tokens == [Token::Ident("inherit")] {
                out.corner_edges[i] = Some(eval.parent_corners[i]);
            // Rayon horizontal seulement (`a / b` elliptique).
            } else if let Some(px) = r
                .px()
                .filter(|_| r.done() || r.peek() == Some(Token::Slash))
            {
                out.corner_edges[i] = Some(px.max(0.));
            }
        }
        P::BorderColor => {
            let mut colors = Vec::new();
            while !r.done() {
                match r.color() {
                    Some(c) => colors.push(c),
                    None => return,
                }
            }
            if let Some(c) = sides(&colors) {
                if c.iter().all(|x| *x == c[0]) {
                    out.border = Some(c[0]);
                    out.border_colors = [None; 4];
                    out.color_expressions[2] = None;
                } else {
                    out.border_colors = c.map(Some);
                }
            }
        }
        P::BorderTopColor | P::BorderRightColor | P::BorderBottomColor | P::BorderLeftColor => {
            if let Some(c) = r.color().filter(|_| r.done()) {
                out.border_colors[side(property)] = Some(c);
            }
        }
        P::Border => {
            if let Some((width, color)) = border(tokens, eval, font) {
                out.border_width = width;
                out.border_edges = [None; 4];
                if let Some(c) = color {
                    out.border = Some(c);
                    out.border_colors = [None; 4];
                    out.color_expressions[2] = None;
                } else {
                    out.border = Some(eval.current);
                }
            }
        }
        P::BorderTop | P::BorderRight | P::BorderBottom | P::BorderLeft => {
            if let Some((width, color)) = border(tokens, eval, font) {
                let i = side(property);
                out.border_edges[i] = width;
                out.border_colors[i] = Some(color.unwrap_or(eval.current));
            }
        }
        P::BorderWidth => {
            let mut values = Vec::new();
            while !r.done() {
                match r.px() {
                    Some(px) => values.push(px.max(0.)),
                    None => return,
                }
            }
            if let Some(w) = sides(&values) {
                if w.iter().all(|x| *x == w[0]) {
                    out.border_width = Some(w[0]);
                    out.border_edges = [None; 4];
                } else {
                    out.border_edges = w.map(Some);
                }
            }
        }
        P::BorderTopWidth | P::BorderRightWidth | P::BorderBottomWidth | P::BorderLeftWidth => {
            if let Some(px) = r.px().filter(|_| r.done()) {
                out.border_edges[side(property)] = Some(px.max(0.));
            }
        }
        P::BorderRadius if tokens == [Token::Ident("inherit")] => {
            let c = eval.parent_corners;
            if c.iter().all(|x| *x == c[0]) {
                out.radius = Some(c[0]);
                out.corner_radii = None;
            } else {
                out.corner_radii = Some(c);
            }
        }
        P::BorderRadius => {
            let mut values = Vec::new();
            while !r.done() {
                if r.peek() == Some(Token::Slash) {
                    // Rayons elliptiques : la partie horizontale seulement.
                    break;
                }
                match r.px() {
                    Some(px) => values.push(px.max(0.)),
                    None => return,
                }
            }
            // Ordre CSS : haut-gauche, haut-droit, bas-droit, bas-gauche.
            if let Some(c) = sides(&values) {
                if c.iter().all(|x| *x == c[0]) {
                    out.radius = Some(c[0]);
                    out.corner_radii = None;
                } else {
                    out.corner_radii = Some(c);
                }
            }
        }
        _ => {}
    }
}
