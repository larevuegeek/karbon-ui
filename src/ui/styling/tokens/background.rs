//! Couches d'images de fond des feuilles de frameworks : `background-image`,
//! `background-position`, `background-size`, `background-repeat` et le raccourci
//! `background` (une couche).
use super::{
    reader::{Evaluation, Quantity, Reader},
    token::{Token, layers},
};
use crate::ui::{
    rendering::svg::SvgImage,
    styling::{
        BACKGROUND_LAYERS, BackgroundImages, BackgroundLength, BackgroundPosition, BackgroundSize,
    },
    theme::Color,
};

type Layers<T> = [T; BACKGROUND_LAYERS];

/// Valeurs initiales de toutes les couches (`background: none`).
pub(super) fn reset() -> BackgroundImages {
    BackgroundImages {
        layers: Some([None; BACKGROUND_LAYERS]),
        position: Some([BackgroundPosition::default(); BACKGROUND_LAYERS]),
        size: Some([BackgroundSize::Auto; BACKGROUND_LAYERS]),
        repeat: Some([true; BACKGROUND_LAYERS]),
    }
}
/// Liste d'une valeur par couche, répétée cycliquement comme en CSS ; les couches au-delà
/// de [`BACKGROUND_LAYERS`] sont ignorées.
pub(super) fn list<T: Copy>(
    tokens: &[Token],
    mut item: impl FnMut(&[Token]) -> Option<T>,
) -> Option<Layers<T>> {
    let values = layers(tokens)
        .into_iter()
        .take(BACKGROUND_LAYERS)
        .map(&mut item)
        .collect::<Option<Vec<_>>>()?;
    let first = *values.first()?;
    let mut out = [first; BACKGROUND_LAYERS];
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = values[i % values.len()];
    }
    Some(out)
}
/// `background-image` : images SVG ou `none` ; un dégradé laisse la propriété inchangée
/// (rendu par les déclarations statiques).
pub(super) fn images(tokens: &[Token]) -> Option<Layers<Option<&'static SvgImage>>> {
    let mut out = [None; BACKGROUND_LAYERS];
    for (i, layer) in layers(tokens).into_iter().enumerate() {
        let image = match layer {
            [Token::Image(image)] => Some(*image),
            [Token::Ident("none")] => None,
            _ => return None,
        };
        if let Some(slot) = out.get_mut(i) {
            *slot = image;
        }
    }
    Some(out)
}
/// Élément d'une position : mot-clé, ou longueur (fraction, px).
#[derive(Clone, Copy)]
enum Place {
    Key(&'static str),
    Length(f32, f32),
}
/// `Some(true)` : mot-clé horizontal, `Some(false)` : vertical, `None` : `center`.
fn axis(key: &str) -> Option<bool> {
    match key {
        "left" | "right" => Some(true),
        "top" | "bottom" => Some(false),
        _ => None,
    }
}
fn keyword(key: &str, offset: f32) -> (f32, f32) {
    match key {
        "left" | "top" => (0., offset),
        "right" | "bottom" => (1., -offset),
        _ => (0.5, 0.),
    }
}
fn place(r: &mut Reader) -> Option<Place> {
    match r.peek()? {
        Token::Ident(k @ ("left" | "right" | "top" | "bottom" | "center")) => {
            r.at += 1;
            Some(Place::Key(k))
        }
        Token::Dimension(..) | Token::Percentage(_) | Token::Number(_) | Token::Function(_) => {
            let at = r.at;
            match r.quantity() {
                Some(Quantity::Length(px, p)) => Some(Place::Length(p / 100., px)),
                Some(Quantity::Number(0.)) => Some(Place::Length(0., 0.)),
                _ => {
                    r.at = at;
                    None
                }
            }
        }
        _ => None,
    }
}
/// Position à partir de ses éléments (une à quatre valeurs).
fn resolve(items: &[Place]) -> Option<BackgroundPosition> {
    let value = |p: Place| match p {
        Place::Key(k) => keyword(k, 0.),
        Place::Length(f, px) => (f, px),
    };
    let center = (0.5, 0.);
    Some(match *items {
        [Place::Key(k)] => match axis(k) {
            Some(true) => BackgroundPosition {
                x: keyword(k, 0.),
                y: center,
            },
            Some(false) => BackgroundPosition {
                x: center,
                y: keyword(k, 0.),
            },
            None => BackgroundPosition {
                x: center,
                y: center,
            },
        },
        [a @ Place::Length(..)] => BackgroundPosition {
            x: value(a),
            y: center,
        },
        [a, b] => {
            let vertical = |p: Place| matches!(p, Place::Key(k) if axis(k) == Some(false));
            let horizontal = |p: Place| matches!(p, Place::Key(k) if axis(k) == Some(true));
            let (x, y) = if vertical(a) || horizontal(b) {
                (b, a)
            } else {
                (a, b)
            };
            if vertical(x) || horizontal(y) {
                return None;
            }
            BackgroundPosition {
                x: value(x),
                y: value(y),
            }
        }
        _ if (3..=4).contains(&items.len()) => {
            // Mots-clés suivis d'un décalage éventuel.
            let mut groups: Vec<(&str, f32)> = Vec::new();
            let mut i = 0;
            while i < items.len() {
                let Place::Key(k) = items[i] else {
                    return None;
                };
                let offset = match items.get(i + 1) {
                    Some(Place::Length(0., px)) if k != "center" => {
                        i += 1;
                        *px
                    }
                    Some(Place::Length(..)) => return None,
                    _ => 0.,
                };
                groups.push((k, offset));
                i += 1;
            }
            let [(a, oa), (b, ob)] = groups[..] else {
                return None;
            };
            let (x, y) = if axis(a) == Some(false) || axis(b) == Some(true) {
                ((b, ob), (a, oa))
            } else {
                ((a, oa), (b, ob))
            };
            if axis(x.0) == Some(false) || axis(y.0) == Some(true) {
                return None;
            }
            BackgroundPosition {
                x: keyword(x.0, x.1),
                y: keyword(y.0, y.1),
            }
        }
        _ => return None,
    })
}
/// `background-position` d'une couche.
pub(super) fn position(
    tokens: &[Token],
    eval: &Evaluation,
    font: f32,
) -> Option<BackgroundPosition> {
    let mut r = Reader {
        tokens,
        at: 0,
        eval,
        font,
    };
    let mut items = Vec::new();
    while !r.done() {
        items.push(place(&mut r)?);
    }
    resolve(&items)
}
fn size_length(r: &mut Reader) -> Option<BackgroundLength> {
    Some(match r.length()? {
        crate::ui::element::Length::Auto => BackgroundLength::Auto,
        crate::ui::element::Length::Px(px) => BackgroundLength::Px(px.max(0.)),
        crate::ui::element::Length::Percent(p) => BackgroundLength::Percent(p.max(0.)),
        _ => return None,
    })
}
fn read_size(r: &mut Reader) -> Option<BackgroundSize> {
    match r.peek()? {
        Token::Ident("contain") => {
            r.at += 1;
            return Some(BackgroundSize::Contain);
        }
        Token::Ident("cover") => {
            r.at += 1;
            return Some(BackgroundSize::Cover);
        }
        _ => {}
    }
    let width = size_length(r)?;
    let at = r.at;
    let height = match r.peek() {
        Some(Token::Ident("auto") | Token::Dimension(..) | Token::Percentage(_))
        | Some(Token::Number(_) | Token::Function(_)) => size_length(r).or_else(|| {
            r.at = at;
            None
        }),
        _ => None,
    };
    Some(match (width, height) {
        (BackgroundLength::Auto, None | Some(BackgroundLength::Auto)) => BackgroundSize::Auto,
        (w, h) => BackgroundSize::Explicit(w, h.unwrap_or(BackgroundLength::Auto)),
    })
}
/// `background-size` d'une couche.
pub(super) fn size(tokens: &[Token], eval: &Evaluation, font: f32) -> Option<BackgroundSize> {
    let mut r = Reader {
        tokens,
        at: 0,
        eval,
        font,
    };
    let size = read_size(&mut r)?;
    r.done().then_some(size)
}
/// `background-repeat` d'une couche : `false` pour `no-repeat`.
pub(super) fn repeat(tokens: &[Token]) -> Option<bool> {
    let mut repeat = None;
    for t in tokens {
        match t {
            Token::Ident("no-repeat") => repeat = Some(repeat.unwrap_or(false)),
            Token::Ident("repeat" | "repeat-x" | "repeat-y" | "space" | "round") => {
                repeat = Some(true)
            }
            _ => return None,
        }
    }
    repeat
}
/// Raccourci `background` d'une seule couche : couleur et images posées ensemble (les
/// valeurs absentes reprennent leur valeur initiale). `None` pour un dégradé ou une
/// valeur invalide.
pub(super) fn shorthand(
    tokens: &[Token],
    eval: &Evaluation,
    font: f32,
) -> Option<(Color, BackgroundImages)> {
    if layers(tokens).len() != 1 {
        return None;
    }
    let mut r = Reader {
        tokens,
        at: 0,
        eval,
        font,
    };
    let (mut color, mut image, mut repeat) = (None, None, None);
    let mut places = Vec::new();
    let mut size = None;
    while !r.done() {
        match r.peek()? {
            Token::Image(i) => {
                r.at += 1;
                image = Some(i);
            }
            Token::Ident("none") => r.at += 1,
            Token::Ident(
                k @ ("no-repeat" | "repeat" | "repeat-x" | "repeat-y" | "space" | "round"),
            ) => {
                r.at += 1;
                repeat = Some(repeat.unwrap_or(true) && k != "no-repeat");
            }
            Token::Ident(
                "scroll" | "fixed" | "local" | "border-box" | "padding-box" | "content-box",
            ) => r.at += 1,
            Token::Slash => {
                r.at += 1;
                size = Some(read_size(&mut r)?);
            }
            Token::Function(name) if name.ends_with("gradient") => return None,
            _ => match place(&mut r) {
                Some(p) => places.push(p),
                None => color = Some(r.color()?),
            },
        }
    }
    let position = if places.is_empty() {
        BackgroundPosition::default()
    } else {
        resolve(&places)?
    };
    let mut images = reset();
    images.layers = Some({
        let mut l = [None; BACKGROUND_LAYERS];
        l[0] = image;
        l
    });
    images.position = Some([position; BACKGROUND_LAYERS]);
    images.size = Some([size.unwrap_or_default(); BACKGROUND_LAYERS]);
    images.repeat = Some([repeat.unwrap_or(true); BACKGROUND_LAYERS]);
    Some((color.unwrap_or(Color::rgba(0, 0, 0, 0)), images))
}
