//! Étapes d'une animation (`@keyframes`) et échantillonnage par propriété : chaque
//! propriété interpole entre ses propres étapes voisines qui la déclarent ; une
//! propriété absente de `0%` ou de `100%` y prend la valeur de l'élément.
use super::{Easing, Effect};
use crate::ui::{animation::timeline::Interpolate, element::Length, theme::Color};

/// Bits des propriétés déclarées par une étape ou animées par une animation.
pub(crate) mod property {
    pub const OPACITY: u8 = 1;
    pub const TRANSFORM: u8 = 2;
    /// Couleur du texte (`color`).
    pub const COLOR: u8 = 4;
    pub const BACKGROUND: u8 = 8;
    pub const BORDER: u8 = 16;
    /// Largeur (`width`).
    pub const WIDTH: u8 = 32;
    /// Hauteur (`height`).
    pub const HEIGHT: u8 = 64;
    pub const EFFECTS: u8 = OPACITY | TRANSFORM;
    pub const COLORS: u8 = COLOR | BACKGROUND | BORDER;
    pub const SIZES: u8 = WIDTH | HEIGHT;
    /// Bit de la couleur d'indice `index` (`0` texte, `1` fond, `2` bordure).
    pub const fn color(index: usize) -> u8 {
        COLOR << index
    }
    /// Bit de la dimension d'indice `index` (`0` largeur, `1` hauteur).
    pub const fn size(index: usize) -> u8 {
        WIDTH << index
    }
}

/// One step of a keyframed [`Animation`](super::Animation), like a CSS `@keyframes` block.
///
/// A step declares some properties only: each property interpolates between the
/// neighbouring steps that declare it, and a property missing at `0.0` or `1.0` takes
/// the element's own value there (its resting effect or computed colour).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Keyframe {
    /// Position in the animation cycle, from `0.0` (start) to `1.0` (end).
    pub offset: f32,
    /// Opacity and transform reached at this offset (only the declared parts count).
    pub effect: Effect,
    /// Easing from this keyframe to the next one declaring the same property; `None`
    /// uses the animation's easing.
    pub easing: Option<Easing>,
    /// Propriétés déclarées (bits de [`property`]).
    pub(crate) declared: u8,
    /// Couleurs déclarées : texte, fond, bordure.
    pub(crate) colors: [Color; 3],
    /// Dimensions déclarées : largeur, hauteur.
    pub(crate) sizes: [Length; 2],
}
impl Keyframe {
    /// Creates a keyframe at `offset` (`0.0..=1.0`) declaring both the opacity and the
    /// transform of `effect`, without a segment-specific easing.
    pub const fn new(offset: f32, effect: Effect) -> Self {
        Self {
            offset,
            effect,
            easing: None,
            declared: property::EFFECTS,
            colors: [Color::rgba(0, 0, 0, 0); 3],
            sizes: [Length::Auto; 2],
        }
    }
    /// Creates a keyframe at `offset` (`0.0..=1.0`) that declares no property yet; add
    /// them with [`opacity`](Self::opacity), [`transform`](Self::transform) and the
    /// colour builders.
    pub const fn at(offset: f32) -> Self {
        let mut value = Self::new(offset, Effect::NONE);
        value.declared = 0;
        value
    }
    /// Declares the opacity (`0.0..=1.0`) at this step.
    pub const fn opacity(mut self, value: f32) -> Self {
        self.effect.opacity = value;
        self.declared |= property::OPACITY;
        self
    }
    /// Declares the transform at this step: the offsets, rotation and scales of
    /// `effect` (its opacity is ignored).
    pub const fn transform(mut self, effect: Effect) -> Self {
        self.effect = Effect {
            opacity: self.effect.opacity,
            ..effect
        };
        self.declared |= property::TRANSFORM;
        self
    }
    /// Declares the text colour (CSS `color`) at this step.
    pub const fn color(mut self, value: Color) -> Self {
        self.colors[0] = value;
        self.declared |= property::COLOR;
        self
    }
    /// Declares the background colour (CSS `background-color`) at this step.
    pub const fn background_color(mut self, value: Color) -> Self {
        self.colors[1] = value;
        self.declared |= property::BACKGROUND;
        self
    }
    /// Declares the border colour (CSS `border-color`) at this step.
    pub const fn border_color(mut self, value: Color) -> Self {
        self.colors[2] = value;
        self.declared |= property::BORDER;
        self
    }
    /// Declares the width (CSS `width`) at this step, in pixels or as a percentage of
    /// the containing block. Two pixel widths, or two percentages, interpolate; other
    /// pairs switch halfway, like CSS discrete animation. Each frame lays the tree out
    /// again.
    pub const fn width(mut self, value: Length) -> Self {
        self.sizes[0] = value;
        self.declared |= property::WIDTH;
        self
    }
    /// Declares the height (CSS `height`) at this step; see [`width`](Self::width).
    pub const fn height(mut self, value: Length) -> Self {
        self.sizes[1] = value;
        self.declared |= property::HEIGHT;
        self
    }
    /// Sets the easing from this keyframe to the next one declaring the same property.
    pub const fn easing(mut self, value: Easing) -> Self {
        self.easing = Some(value);
        self
    }
    pub(crate) fn valid(self) -> bool {
        self.offset.is_finite()
            && (0. ..=1.).contains(&self.offset)
            && self.effect.valid()
            && self.easing.is_none_or(Easing::valid)
            && self.sizes.iter().enumerate().all(|(i, size)| {
                self.declared & property::size(i) == 0
                    || match *size {
                        Length::Px(n) | Length::Percent(n) => n.is_finite() && n >= 0.,
                        Length::Auto | Length::MaxContent => true,
                        Length::Computed(_) | Length::Fill(_) => false,
                    }
            })
    }
}
/// Dimension animée : deux longueurs en pixels, ou deux pourcentages, s'interpolent ;
/// les autres paires basculent à mi-parcours (animation discrète de CSS).
impl Interpolate for Length {
    fn lerp(self, to: Self, t: f32) -> Self {
        match (self, to) {
            (Length::Px(a), Length::Px(b)) => Length::Px(a.lerp(b, t).max(0.)),
            (Length::Percent(a), Length::Percent(b)) => Length::Percent(a.lerp(b, t).max(0.)),
            _ if t < 0.5 => self,
            _ => to,
        }
    }
}
impl Interpolate for Effect {
    fn lerp(self, to: Self, t: f32) -> Self {
        self.mix(to, t)
    }
}
/// Union des propriétés déclarées par `frames` (fonction constante : calculée une fois
/// à la construction de l'animation).
pub(crate) const fn declared(frames: &[Keyframe]) -> u8 {
    let mut bits = 0;
    let mut i = 0;
    while i < frames.len() {
        bits |= frames[i].declared;
        i += 1;
    }
    bits
}
/// `true` quand les offsets croissent (au sens large) et qu'aucune propriété n'est
/// déclarée deux fois au même offset.
pub(crate) fn ordered(frames: &[Keyframe]) -> bool {
    frames.iter().enumerate().all(|(i, a)| {
        frames[i + 1..]
            .iter()
            .take_while(|b| b.offset == a.offset)
            .all(|b| a.declared & b.declared == 0)
            && frames.get(i + 1).is_none_or(|b| a.offset <= b.offset)
    })
}
/// Valeur d'une propriété à la progression `t` : interpolation entre la dernière étape
/// qui la déclare à `t` ou avant et la première qui la déclare après, avec la courbe de
/// la première. Sans étape avant (après), la valeur de l'élément `underlying` vaut à 0
/// (à 1), avec la courbe de l'animation.
pub(crate) fn sample<V: Interpolate>(
    frames: &[Keyframe],
    bit: u8,
    t: f32,
    easing: Easing,
    underlying: V,
    value: impl Fn(&Keyframe) -> V,
) -> V {
    let mut previous = None;
    let mut next = None;
    for frame in frames.iter().filter(|f| f.declared & bit != 0) {
        if frame.offset <= t {
            previous = Some(frame);
        } else {
            next = Some(frame);
            break;
        }
    }
    let (start, from, curve) = match previous {
        Some(f) if f.offset == t => return value(f),
        Some(f) => (f.offset, value(f), f.easing.unwrap_or(easing)),
        None => (0., underlying, easing),
    };
    let Some(next) = next else {
        // Étape 100 % implicite : valeur exacte de l'élément à la fin.
        if t >= 1. {
            return underlying;
        }
        return interpolate(start, from, curve, 1., underlying, t);
    };
    interpolate(start, from, curve, next.offset, value(next), t)
}
fn interpolate<V: Interpolate>(start: f32, from: V, curve: Easing, end: f32, to: V, t: f32) -> V {
    if end <= start {
        return from;
    }
    let local = ((t - start) / (end - start)).clamp(0., 1.);
    from.lerp(to, curve.sample(local))
}
/// Opacité et transformation des étapes à la progression `t`, à partir de l'effet
/// neutre (celui de l'élément, avec lequel le moteur compose l'animation).
pub(crate) fn effect(frames: &[Keyframe], declared: u8, t: f32, easing: Easing) -> Effect {
    let track = |bit| {
        if declared & bit == 0 {
            Effect::NONE
        } else {
            sample(frames, bit, t, easing, Effect::NONE, |f| f.effect)
        }
    };
    let mut value = track(property::TRANSFORM);
    value.opacity = track(property::OPACITY).opacity;
    value
}
/// Couleurs déclarées par les étapes à la progression `t` : `underlying` (couleurs
/// calculées de l'élément, transparent sans valeur) sert aux étapes extrêmes absentes.
pub(crate) fn colors(
    frames: &[Keyframe],
    declared: u8,
    t: f32,
    easing: Easing,
    underlying: [Option<Color>; 3],
    out: &mut [Option<Color>; 3],
) {
    for (index, slot) in out.iter_mut().enumerate() {
        let bit = property::color(index);
        if declared & bit != 0 {
            let base = underlying[index].unwrap_or(Color::rgba(0, 0, 0, 0));
            *slot = Some(sample(frames, bit, t, easing, base, |f| f.colors[index]));
        }
    }
}
/// Dimensions déclarées par les étapes à la progression `t`, écrites dans `out` :
/// `underlying` (largeur et hauteur de l'élément) sert aux étapes extrêmes absentes.
pub(crate) fn sizes(
    frames: &[Keyframe],
    declared: u8,
    t: f32,
    easing: Easing,
    underlying: [Length; 2],
    out: &mut [Option<Length>; 2],
) {
    for (index, slot) in out.iter_mut().enumerate() {
        let bit = property::size(index);
        if declared & bit != 0 {
            *slot = Some(sample(frames, bit, t, easing, underlying[index], |f| {
                f.sizes[index]
            }));
        }
    }
}
