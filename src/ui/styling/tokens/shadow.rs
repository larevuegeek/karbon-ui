//! Ombres portées (`box-shadow`) et leur internement.
use super::{
    reader::{Evaluation, Reader},
    token::{Token, layers},
};
use crate::ui::rendering::component_shadow::{BoxShadow, Shadows};

/// Ombres internées : la liste de `Shadows` est statique ; les listes calculées sont
/// conservées pour la durée du processus (au plus 4096 listes distinctes).
fn intern_shadows(list: Vec<BoxShadow>) -> Option<&'static [BoxShadow]> {
    use std::sync::Mutex;
    static INTERNED: Mutex<Vec<&'static [BoxShadow]>> = Mutex::new(Vec::new());
    let mut interned = INTERNED.lock().ok()?;
    if let Some(found) = interned.iter().find(|s| **s == list.as_slice()) {
        return Some(found);
    }
    if interned.len() >= 4096 {
        return None;
    }
    let leaked: &'static [BoxShadow] = Box::leak(list.into_boxed_slice());
    interned.push(leaked);
    Some(leaked)
}
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
    Some(Shadows::new(intern_shadows(list)?))
}
