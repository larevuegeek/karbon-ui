//! Raccourcis de bordure (`largeur style couleur`) et répartition sur les côtés.
use super::{
    reader::{Evaluation, Reader},
    token::Token,
};
use crate::ui::theme::Color;

/// Un à quatre éléments répartis sur les côtés (haut, droite, bas, gauche).
pub(super) fn sides<T: Copy>(values: &[T]) -> Option<[T; 4]> {
    Some(match values {
        [a] => [*a; 4],
        [a, b] => [*a, *b, *a, *b],
        [a, b, c] => [*a, *b, *c, *b],
        [a, b, c, d] => [*a, *b, *c, *d],
        _ => return None,
    })
}
/// `largeur style couleur` dans n'importe quel ordre ; `none` donne une largeur nulle.
pub(super) fn border(
    tokens: &[Token],
    eval: &Evaluation,
    font: f32,
) -> Option<(Option<f32>, Option<Color>)> {
    let mut r = Reader {
        tokens,
        at: 0,
        eval,
        font,
    };
    let (mut width, mut color, mut none) = (None, None, false);
    while !r.done() {
        match r.peek()? {
            Token::Ident(
                "solid" | "dashed" | "dotted" | "double" | "groove" | "ridge" | "inset" | "outset",
            ) => r.at += 1,
            Token::Ident("none" | "hidden") => {
                r.at += 1;
                none = true;
            }
            Token::Ident("thin") => {
                r.at += 1;
                width = Some(1.);
            }
            Token::Ident("medium") => {
                r.at += 1;
                width = Some(3.);
            }
            Token::Ident("thick") => {
                r.at += 1;
                width = Some(5.);
            }
            _ => {
                let at = r.at;
                if let Some(px) = r.px() {
                    width = Some(px.max(0.));
                } else {
                    r.at = at;
                    color = Some(r.color()?);
                }
            }
        }
    }
    if none {
        width = Some(0.);
    }
    // Largeur initiale `medium` quand seul le style ou la couleur est donné.
    Some((width.or(Some(3.)), color))
}
