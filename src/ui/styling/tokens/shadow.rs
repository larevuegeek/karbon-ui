//! Ombres portées (`box-shadow`) calculées à la résolution des styles.
use super::{
    reader::{Evaluation, Reader},
    token::{Token, layers},
};
use crate::ui::rendering::component_shadow::{BoxShadow, Shadows};

pub(super) fn shadows(tokens: &[Token], eval: &Evaluation, font: f32) -> Option<Shadows> {
    if tokens == [Token::Ident("none")] {
        return Some(Shadows::new(&[]));
    }
    let mut list = Vec::new();
    for layer in layers(tokens) {
        let mut r = Reader {
            tokens: layer,
            at: 0,
            eval,
            font,
        };
        let mut inset = false;
        let mut lengths = Vec::new();
        let mut color = None;
        while !r.done() {
            if r.peek() == Some(Token::Ident("inset")) {
                r.at += 1;
                inset = true;
                continue;
            }
            let at = r.at;
            if let Some(px) = r.px() {
                lengths.push(px);
                continue;
            }
            r.at = at;
            color = Some(r.color()?);
        }
        if !(2..=4).contains(&lengths.len()) {
            return None;
        }
        let color = color.unwrap_or(eval.current);
        // Les couches entièrement transparentes et nulles (`0 0 #0000` de Tailwind) ne
        // dessinent rien.
        if color.alpha == 0 && lengths.iter().all(|n| *n == 0.) {
            continue;
        }
        list.push(BoxShadow {
            x: lengths[0],
            y: lengths[1],
            blur: lengths.get(2).copied().unwrap_or(0.).max(0.),
            spread: lengths.get(3).copied().unwrap_or(0.),
            color,
            inset,
        });
    }
    if list.len() > 16 {
        return None;
    }
    // Liste partagée, libérée avec son dernier utilisateur : plus de listes gardées à vie
    // ni de limite au-delà de laquelle l'ombre n'était plus appliquée (L10 de la revue).
    Some(Shadows::shared(list))
}
