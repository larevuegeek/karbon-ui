//! Pièces d'un curseur dessiné en CSS (`::-webkit-slider-thumb`,
//! `::-webkit-slider-runnable-track`).
use super::super::{
    border::border,
    reader::{Evaluation, Reader},
    shadow::shadows,
    token::Token,
};
use super::{DynamicProperty, part};
use crate::ui::styling::Properties;

/// Applique une propriété d'une pièce de curseur.
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
        P::ThumbBackground | P::TrackBackground => {
            if let Some(c) = r.color().filter(|_| r.done()) {
                part(out, property).background = Some(c);
            }
        }
        P::ThumbWidth | P::ThumbHeight | P::TrackHeight => {
            if let Some(px) = r.px().filter(|_| r.done()) {
                let p = part(out, property);
                if property == P::ThumbWidth {
                    p.width = Some(px.max(0.));
                } else {
                    p.height = Some(px.max(0.));
                }
            }
        }
        P::ThumbRadius | P::TrackRadius => {
            // Premier rayon (coins égaux sur une pièce de contrôle).
            if let Some(px) = r.px() {
                part(out, property).radius = Some(px.max(0.));
            }
        }
        P::ThumbBorder | P::TrackBorder => {
            if let Some((width, color)) = border(tokens, eval, font) {
                let p = part(out, property);
                p.border_width = width;
                p.border = Some(color.unwrap_or(eval.current));
            }
        }
        P::ThumbShadow => {
            if let Some(s) = shadows(tokens, eval, font) {
                out.slider_thumb.shadows = Some(s);
            }
        }
        _ => {}
    }
}
