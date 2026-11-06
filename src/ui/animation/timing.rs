//! Minutage d'une animation : progression dans les étapes à un instant donné
//! (délai, répétitions, direction, remplissage), valeurs et validation.
use super::{
    Animation, Effect, FillMode,
    keyframes::{self, property},
};
use crate::ui::theme::Color;

/// Prochain changement d'une animation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Due {
    /// Plus aucun changement (fin, ou état final).
    Never,
    /// En cours : une image par trame.
    Frame,
    /// Rien ne change avant ce temps écoulé (ms, avance comprise) : fin du délai.
    At(f64),
}
impl Animation {
    /// Animations jouées : les membres d'un groupe, ou l'animation elle-même.
    pub(crate) fn members(&self) -> &[Animation] {
        if self.group.is_empty() {
            std::slice::from_ref(self)
        } else {
            self.group
        }
    }
    /// Décalage (ms) entre le temps écoulé depuis le début de l'élément et le temps du
    /// membre : son avance, moins le délai du groupe (`data-stagger`).
    pub(crate) fn offset(&self, member: &Animation) -> f64 {
        let outer = if self.group.is_empty() {
            0.
        } else {
            self.timing.delay as f64
        };
        member.advance as f64 - outer
    }
    /// `true` quand l'itération d'indice `index` se joue à rebours.
    fn reversed(self, index: u64) -> bool {
        self.reverse ^ (self.alternate && index % 2 == 1)
    }
    /// Progression de l'état final (remplissage avant), `None` sans remplissage.
    fn end_progress(self) -> Option<f32> {
        matches!(self.fill, FillMode::Forwards | FillMode::Both).then(|| {
            let last = self.iterations.map_or(0, |n| n.saturating_sub(1) as u64);
            if self.reversed(last) { 0. } else { 1. }
        })
    }
    /// Progression dans les étapes (`0` à `1`) après `elapsed` ms, `None` quand
    /// l'animation n'a pas d'effet (délai sans remplissage arrière, fin sans remplissage
    /// avant) ; `elapsed` à `None` demande l'état final (mouvements réduits).
    pub(crate) fn progress(self, elapsed: Option<f64>) -> (Option<f32>, Due) {
        let Some(elapsed) = elapsed else {
            return (self.end_progress(), Due::Never);
        };
        let delay = self.timing.delay as f64;
        if elapsed < delay {
            let start = matches!(self.fill, FillMode::Backwards | FillMode::Both)
                .then_some(if self.reverse { 1. } else { 0. });
            return (start, Due::At(delay));
        }
        if self.timing.duration == 0 {
            return (self.end_progress(), Due::Never);
        }
        let cycles = (elapsed - delay) / self.timing.duration as f64;
        if self.iterations.is_some_and(|n| cycles >= n as f64) {
            return (self.end_progress(), Due::Never);
        }
        let mut t = cycles.fract() as f32;
        if self.reversed(cycles.floor() as u64) {
            t = 1. - t;
        }
        (Some(t), Due::Frame)
    }
    /// Opacité et transformation à la progression `t`, à partir de l'effet neutre.
    pub(crate) fn effect_at(self, t: f32) -> Effect {
        if self.frames.is_empty() {
            if t >= 1. {
                return self.to;
            }
            return self.from.mix(self.to, self.timing.easing.sample(t));
        }
        let mut value = keyframes::effect(self.frames, self.properties, t, self.timing.easing);
        value.x *= self.distance_scale;
        value.y *= self.distance_scale;
        value
    }
    /// Couleurs animées à la progression `t`, écrites dans `out` ; `underlying` donne les
    /// couleurs calculées de l'élément.
    pub(crate) fn colors_at(
        self,
        t: f32,
        underlying: [Option<Color>; 3],
        out: &mut [Option<Color>; 3],
    ) {
        keyframes::colors(
            self.frames,
            self.properties,
            t,
            self.timing.easing,
            underlying,
            out,
        );
    }
    /// Effet constant sur toute l'animation (aucune image à demander), s'il l'est.
    pub(crate) fn constant_effect(self) -> Option<Effect> {
        if self.properties & property::EFFECTS == 0 {
            return Some(Effect::NONE);
        }
        if self.frames.is_empty() {
            return (self.from == self.to).then_some(self.to);
        }
        let start = self.effect_at(0.);
        let same = |f: &keyframes::Keyframe| {
            let mut e = f.effect;
            e.x *= self.distance_scale;
            e.y *= self.distance_scale;
            (f.declared & property::OPACITY == 0 || e.opacity == start.opacity)
                && (f.declared & property::TRANSFORM == 0
                    || Effect {
                        opacity: start.opacity,
                        ..e
                    } == start)
        };
        (self.effect_at(1.) == start && self.frames.iter().all(same)).then_some(start)
    }
    /// Une étape ou un état déplace, tourne ou met à l'échelle l'élément.
    pub(crate) fn spatial(self) -> bool {
        self.members().iter().any(|m| {
            m.from.spatial()
                || m.to.spatial()
                || m.frames
                    .iter()
                    .any(|f| f.declared & property::TRANSFORM != 0 && f.effect.spatial())
        })
    }
    /// Même animation à `animation-play-state` près : basculer la pause d'un membre ne
    /// redémarre pas l'animation.
    pub(crate) fn same_run(self, other: Self) -> bool {
        let plain = |a: Animation| Animation {
            paused: false,
            group: &[],
            ..a
        };
        plain(self) == plain(other)
            && self.group.len() == other.group.len()
            && self
                .group
                .iter()
                .zip(other.group)
                .all(|(a, b)| plain(*a) == plain(*b))
    }
    /// L'animation anime une couleur (échantillonnée pendant la résolution des styles).
    pub(crate) fn animates_colors(self) -> bool {
        self.properties & property::COLORS != 0
    }
    /// Durée (ms) au bout de laquelle une animation de sortie est terminée.
    pub(crate) fn exit_length(self) -> f64 {
        self.members()
            .iter()
            .map(|m| m.timing.delay as f64 + m.timing.duration as f64 - self.offset(m))
            .fold(0., f64::max)
    }
    fn valid_member(self) -> bool {
        self.iterations != Some(0)
            && self.timing.easing.valid()
            && self.from.valid()
            && self.to.valid()
            && self.distance_scale.is_finite()
            && self.frames.len() <= 128
            && self.frames.iter().all(|f| f.valid())
            && keyframes::ordered(self.frames)
    }
    pub(crate) fn valid(self) -> bool {
        if self.group.is_empty() {
            return self.valid_member();
        }
        self.group.len() <= 16
            && self.distance_scale.is_finite()
            && self
                .group
                .iter()
                .all(|m| m.group.is_empty() && m.valid_member())
    }
}
