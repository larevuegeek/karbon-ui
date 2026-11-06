//! Opacité, transformations et transition.
use super::super::{
    reader::{Evaluation, Reader},
    token::Token,
    transition::transition,
};
use super::{DynamicProperty, effect};
use crate::ui::styling::Properties;

/// Applique une propriété d'effet (opacité, transformation) ou la transition.
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
        P::Transition => {
            if let Some(t) = transition(tokens) {
                out.transition = Some(t);
            }
        }
        P::Opacity => {
            let value = match r.peek() {
                Some(Token::Percentage(p)) => {
                    r.at += 1;
                    Some(p / 100.)
                }
                _ => r.number(),
            };
            if let Some(o) = value.filter(|_| r.done()) {
                effect(out, |e| {
                    e.effect.opacity = o.clamp(0., 1.);
                    e.opacity = true;
                });
            }
        }
        P::Transform => {
            if tokens == [Token::Ident("none")] {
                effect(out, |e| {
                    let o = e.effect.opacity;
                    e.effect = crate::ui::animation::Effect::NONE;
                    e.effect.opacity = o;
                    e.transform = true;
                });
                return;
            }
            let mut fx = crate::ui::animation::Effect::NONE;
            while !r.done() {
                let Some(Token::Function(name)) = r.next() else {
                    return;
                };
                let args = r.arguments().to_vec();
                let mut a = Reader {
                    tokens: &args,
                    at: 0,
                    eval,
                    font,
                };
                let ok = match name {
                    "translate" => {
                        let x = a.px();
                        let y = if a.peek() == Some(Token::Comma) {
                            a.at += 1;
                            a.px()
                        } else {
                            Some(0.)
                        };
                        x.zip(y).map(|(x, y)| {
                            fx.x += x;
                            fx.y += y;
                        })
                    }
                    "translateX" | "translatex" => a.px().map(|x| fx.x += x),
                    "translateY" | "translatey" => a.px().map(|y| fx.y += y),
                    "rotate" => a.angle().map(|d| fx.rotation += d),
                    "scale" => {
                        let x = a.number();
                        let y = if a.peek() == Some(Token::Comma) {
                            a.at += 1;
                            a.number()
                        } else {
                            x
                        };
                        x.zip(y).map(|(x, y)| {
                            fx.scale_x *= x;
                            fx.scale_y *= y;
                        })
                    }
                    "scaleX" | "scalex" => a.number().map(|x| fx.scale_x *= x),
                    "scaleY" | "scaley" => a.number().map(|y| fx.scale_y *= y),
                    _ => None,
                };
                if ok.is_none() || !a.done() {
                    return;
                }
            }
            effect(out, |e| {
                let o = e.effect.opacity;
                e.effect = fx;
                e.effect.opacity = o;
                e.transform = true;
            });
        }
        P::Translate => {
            if tokens == [Token::Ident("none")] {
                return;
            }
            let x = r.px();
            let y = if r.done() { Some(0.) } else { r.px() };
            if let (Some(x), Some(y)) = (x, y)
                && r.at_end_or_comma()
            {
                effect(out, |e| {
                    e.effect.x = x;
                    e.effect.y = y;
                    e.transform = true;
                });
            }
        }
        P::Scale => {
            if tokens == [Token::Ident("none")] {
                return;
            }
            let number = |r: &mut Reader| match r.peek() {
                Some(Token::Percentage(p)) => {
                    r.at += 1;
                    Some(p / 100.)
                }
                _ => r.number(),
            };
            let x = number(&mut *r);
            let y = if r.done() { x } else { number(&mut *r) };
            if let (Some(x), Some(y)) = (x, y) {
                effect(out, |e| {
                    e.effect.scale_x = x;
                    e.effect.scale_y = y;
                    e.transform = true;
                });
            }
        }
        P::Rotate => {
            if tokens == [Token::Ident("none")] {
                return;
            }
            if let Some(d) = r.angle().filter(|_| r.done()) {
                effect(out, |e| {
                    e.effect.rotation = d;
                    e.transform = true;
                });
            }
        }
        _ => {}
    }
}
