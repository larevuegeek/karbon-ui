//! Couleurs, fond, contour, apparence et ombres des déclarations dynamiques.
use super::super::{
    background,
    border::border,
    reader::{Evaluation, Reader},
    shadow::shadows,
    token::Token,
};
use super::DynamicProperty;
use crate::ui::{styling::Properties, theme::Color};

/// Applique une propriété de peinture (couleur, fond, contour, ombre).
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
        P::Color => {
            if let Some(c) = r.color().filter(|_| r.done()) {
                out.text.color = Some(c);
                out.color_expressions[0] = None;
            }
        }
        P::BackgroundColor | P::Background => {
            if tokens == [Token::Ident("none")] {
                out.background = Some(Color::rgba(0, 0, 0, 0));
                out.color_expressions[1] = None;
                if property == P::Background {
                    out.background_images = background::reset();
                }
            } else if let Some(c) = r.color().filter(|_| r.done()) {
                out.background = Some(c);
                out.color_expressions[1] = None;
            } else if property == P::Background
                && let Some((color, images)) = background::shorthand(tokens, eval, font)
            {
                out.background = Some(color);
                out.color_expressions[1] = None;
                out.background_images = images;
            }
        }
        P::BackgroundImage => {
            if let Some(layers) = background::images(tokens) {
                out.background_images.layers = Some(layers);
            }
        }
        P::BackgroundPosition => {
            if let Some(p) = background::list(tokens, |t| background::position(t, eval, font)) {
                out.background_images.position = Some(p);
            }
        }
        P::BackgroundSize => {
            if let Some(s) = background::list(tokens, |t| background::size(t, eval, font)) {
                out.background_images.size = Some(s);
            }
        }
        P::BackgroundRepeat => {
            if let Some(s) = background::list(tokens, background::repeat) {
                out.background_images.repeat = Some(s);
            }
        }
        P::Appearance => {
            out.appearance_none = Some(tokens == [Token::Ident("none")]);
        }
        P::AccentColor => {
            if let Some(c) = r.color().filter(|_| r.done()) {
                out.accent = Some(c);
                out.color_expressions[3] = None;
            }
        }
        P::OutlineColor => {
            if let Some(c) = r.color().filter(|_| r.done()) {
                out.outline_color = Some(c);
            }
        }
        P::Outline => {
            if let Some((width, color)) = border(tokens, eval, font) {
                out.outline_width = width;
                if color.is_some() {
                    out.outline_color = color;
                }
            }
        }
        P::OutlineWidth => {
            if let Some(px) = r.px().filter(|_| r.done()) {
                out.outline_width = Some(px.max(0.));
            }
        }
        P::OutlineOffset => {
            if let Some(px) = r.px().filter(|_| r.done()) {
                out.outline_offset = Some(px);
            }
        }
        P::BoxShadow => {
            if let Some(s) = shadows(tokens, eval, font) {
                out.shadows = Some(s);
            }
        }
        _ => {}
    }
}
