//! Transitions de valeurs CSS (couleurs, longueurs) échantillonnées pendant la
//! résolution des styles.
use super::{FRAME, earliest};
use crate::ui::{animation::Transition, element::ElementId, theme::Color};
use std::{collections::HashMap, time::Instant};
/// Valeur CSS interpolable par une transition : couleur ou longueur en pixels.
pub(crate) trait Interpolate: Copy + PartialEq {
    /// Valeur à la progression `t` (déjà passée par la courbe) entre `self` et `to`.
    fn lerp(self, to: Self, t: f32) -> Self;
}
impl Interpolate for Color {
    /// Interpolation en alpha prémultiplié, comme les navigateurs (CSS Color 4) : de
    /// `transparent` (noir transparent) vers une couleur, seule l'opacité change. Sans
    /// prémultiplication, le fond blanc d'un onglet qui devient transparent passait par
    /// un gris semi-opaque visible.
    fn lerp(self, to: Self, t: f32) -> Self {
        let (a0, a1) = (self.alpha as f32 / 255., to.alpha as f32 / 255.);
        let alpha = a0 + (a1 - a0) * t;
        if alpha <= 0. {
            return Color::rgba(0, 0, 0, 0);
        }
        let channel = |a: u8, b: u8| {
            let premultiplied = a as f32 * a0 + (b as f32 * a1 - a as f32 * a0) * t;
            (premultiplied / alpha).round().clamp(0., 255.) as u8
        };
        Color::rgba(
            channel(self.red, to.red),
            channel(self.green, to.green),
            channel(self.blue, to.blue),
            (alpha * 255.).round().clamp(0., 255.) as u8,
        )
    }
}
impl Interpolate for f32 {
    fn lerp(self, to: Self, t: f32) -> Self {
        self + (to - self) * t
    }
}
/// Transition d'une valeur : départ, cible, début et minutage.
#[derive(Clone, Copy)]
struct ValueTween<V> {
    from: V,
    to: V,
    start: Instant,
    timing: Transition,
    seen: u64,
    /// La valeur finale a été rendue (plus rien à échantillonner).
    done: bool,
}
impl<V: Interpolate> ValueTween<V> {
    fn sample(self, now: Instant) -> (V, bool) {
        let elapsed = now.saturating_duration_since(self.start).as_secs_f64() * 1000.
            - self.timing.delay as f64;
        if self.from == self.to || elapsed >= self.timing.duration as f64 {
            return (self.to, false);
        }
        if elapsed <= 0. {
            return (self.from, true);
        }
        let t = self
            .timing
            .easing
            .sample((elapsed / self.timing.duration as f64) as f32);
        (self.from.lerp(self.to, t), true)
    }
}
/// Valeurs en transition, par élément et propriété : couleurs (`transition:
/// background-color …`, propriétés `0` texte, `1` fond, `2` bordure) ou dimensions
/// (`transition: width …`, `0` largeur, `1` hauteur).
pub(crate) struct ValueTracks<V> {
    tracks: HashMap<(ElementId, u8), ValueTween<V>>,
    epoch: u64,
    pub(super) next: Option<Instant>,
    active: bool,
}
impl<V> Default for ValueTracks<V> {
    fn default() -> Self {
        Self {
            tracks: HashMap::new(),
            epoch: 0,
            next: None,
            active: false,
        }
    }
}
impl<V: Interpolate> ValueTracks<V> {
    /// Début d'une passe de styles : les pistes non revues seront oubliées.
    pub fn begin(&mut self) {
        self.epoch = self.epoch.wrapping_add(1);
        self.next = None;
        self.active = false;
    }
    pub fn end(&mut self) {
        let epoch = self.epoch;
        self.tracks.retain(|_, t| t.seen == epoch);
    }
    pub(super) fn animating(&self) -> bool {
        self.active
    }
    /// Éléments dont une transition n'a pas encore rendu sa valeur finale à `now`
    /// (résolution partielle des styles).
    pub(crate) fn unsettled(&self, now: Instant) -> impl Iterator<Item = ElementId> + '_ {
        let _ = now;
        self.tracks
            .iter()
            .filter_map(|((id, _), t)| (!t.done).then_some(*id))
    }
    /// Valeur affichée pour `target` ; un changement de cible démarre une transition
    /// depuis la valeur courante.
    pub fn sample(
        &mut self,
        id: ElementId,
        property: u8,
        target: V,
        timing: Transition,
        now: Instant,
        reduced: bool,
    ) -> V {
        let epoch = self.epoch;
        let track = self.tracks.entry((id, property)).or_insert(ValueTween {
            from: target,
            to: target,
            start: now,
            timing,
            seen: epoch,
            done: false,
        });
        track.seen = epoch;
        if track.to != target {
            let (current, _) = track.sample(now);
            *track = ValueTween {
                from: if reduced { target } else { current },
                to: target,
                start: now,
                timing,
                seen: epoch,
                done: false,
            };
        }
        let (value, running) = track.sample(now);
        track.done = !running;
        if running {
            self.active = true;
            self.next = earliest(self.next, now.checked_add(FRAME));
        }
        value
    }
}
