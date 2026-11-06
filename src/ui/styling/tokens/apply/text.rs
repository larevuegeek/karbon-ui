//! Typographie : taille, graisse, interligne et espacement des lettres.
use super::super::{reader::Reader, token::Token};
use super::DynamicProperty;
use crate::ui::styling::{LetterSpacing, LineHeight, Properties};

/// Applique une propriété de typographie.
pub(super) fn apply(property: DynamicProperty, r: &mut Reader, out: &mut Properties) {
    use DynamicProperty as P;
    match property {
        P::FontSize => {
            if let Some(px) = r.px().filter(|n| *n > 0. && r.done()) {
                out.text.size = Some(px);
                out.font_size_expression = None;
            }
        }
        P::FontWeight => {
            let weight = match r.peek() {
                Some(Token::Ident("normal")) => Some(400.),
                Some(Token::Ident("bold")) => Some(700.),
                Some(Token::Ident("bolder")) => Some(700.),
                Some(Token::Ident("lighter")) => Some(300.),
                _ => None,
            };
            let weight = match weight {
                Some(w) => {
                    r.at += 1;
                    Some(w)
                }
                None => r.number(),
            };
            if let Some(w) = weight.filter(|w| (1. ..=999.).contains(w) && r.done()) {
                out.text.weight = Some(w as u16);
            }
        }
        P::LineHeight => {
            let value = match r.peek() {
                Some(Token::Ident("normal")) => {
                    r.at += 1;
                    Some(LineHeight::Normal)
                }
                Some(Token::Number(n)) if n > 0. => {
                    r.at += 1;
                    Some(LineHeight::Factor(n))
                }
                _ => r.px().filter(|n| *n > 0.).map(LineHeight::Px),
            };
            if let Some(v) = value.filter(|_| r.done()) {
                out.text.line_height = Some(v);
            }
        }
        P::LetterSpacing => {
            let value = if r.peek() == Some(Token::Ident("normal")) {
                r.at += 1;
                Some(0.)
            } else {
                r.px()
            };
            if let Some(px) = value.filter(|_| r.done()) {
                out.text.letter_spacing = Some(LetterSpacing::Px(px));
            }
        }
        _ => {}
    }
}
