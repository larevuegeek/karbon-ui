//! Pistes d'effets : transition entre deux effets cibles et échantillonnage d'une
//! animation d'entrée (une ou plusieurs animations, chacune avec son minutage).
use super::{FRAME, earliest};
use crate::ui::animation::{
    Animation, Due, Effect, FillMode, Motion, Transition, keyframes::property,
};
use std::time::{Duration, Instant};
#[derive(Clone, Copy)]
pub(super) struct Tween {
    pub(super) from: Effect,
    pub(super) to: Effect,
    pub(super) start: Instant,
    pub(super) timing: Transition,
}
impl Tween {
    fn sample(self, now: Instant) -> (Effect, Option<Instant>) {
        if self.from == self.to {
            return (self.to, None);
        }
        let elapsed = now.saturating_duration_since(self.start).as_secs_f64() * 1000.;
        if elapsed < self.timing.delay as f64 {
            return (
                self.from,
                self.start
                    .checked_add(Duration::from_millis(self.timing.delay as u64)),
            );
        }
        let t = if self.timing.duration == 0 {
            1.
        } else {
            ((elapsed - self.timing.delay as f64) / self.timing.duration as f64).min(1.) as f32
        };
        (
            self.from.mix(self.to, self.timing.easing.sample(t)),
            if t < 1. { now.checked_add(FRAME) } else { None },
        )
    }
}
/// Pause d'un membre : instant de la mise en pause en cours et durée cumulée des pauses
/// terminées.
#[derive(Clone, Copy, Default)]
struct Pause {
    since: Option<Instant>,
    total: Duration,
}
/// Horloge d'une animation d'élément : début, et pauses de chaque membre (vide tant
/// qu'aucun n'a été mis en pause).
#[derive(Clone)]
pub(crate) struct Clock {
    pub(super) start: Instant,
    pauses: Vec<Pause>,
}
impl Clock {
    pub(crate) fn new(start: Instant, animation: Option<Animation>) -> Self {
        let mut clock = Self {
            start,
            pauses: Vec::new(),
        };
        if let Some(a) = animation {
            clock.update(a, start);
        }
        clock
    }
    /// Met en pause ou reprend les membres dont `animation-play-state` a changé.
    pub(super) fn update(&mut self, animation: Animation, now: Instant) {
        let members = animation.members();
        if self.pauses.is_empty() && members.iter().all(|m| !m.paused) {
            return;
        }
        self.pauses.resize(members.len(), Pause::default());
        for (pause, member) in self.pauses.iter_mut().zip(members) {
            match (pause.since, member.paused) {
                (None, true) => pause.since = Some(now),
                (Some(since), false) => {
                    pause.total += now.saturating_duration_since(since);
                    pause.since = None;
                }
                _ => {}
            }
        }
    }
    pub(super) fn shift(&mut self, delta: Duration) {
        let shift = |t: &mut Instant| *t = t.checked_add(delta).unwrap_or(*t);
        shift(&mut self.start);
        for pause in &mut self.pauses {
            if let Some(since) = &mut pause.since {
                shift(since);
            }
        }
    }
    fn pause(&self, member: usize) -> Pause {
        self.pauses.get(member).copied().unwrap_or_default()
    }
    /// Temps du membre d'indice `member` (ms) à `now`, décalage `offset` compris (avance
    /// et délai du groupe) ; figé pendant une pause.
    pub(crate) fn elapsed(&self, member: usize, offset: f64, now: Instant) -> f64 {
        let pause = self.pause(member);
        let start = self.start.checked_add(pause.total).unwrap_or(self.start);
        let now = pause.since.unwrap_or(now);
        now.saturating_duration_since(start).as_secs_f64() * 1000. + offset
    }
    /// Instant du prochain changement d'un membre.
    fn due(&self, member: usize, offset: f64, due: Due, now: Instant) -> Option<Instant> {
        match due {
            Due::Never => None,
            Due::Frame => now.checked_add(FRAME),
            Due::At(ms) => {
                let start = self.start.checked_add(self.pause(member).total)?;
                start.checked_add(Duration::from_millis((ms - offset).max(0.) as u64))
            }
        }
    }
}
/// Effet de l'animation (de ses membres) à `now` et prochain réveil ; `clock` à `None`
/// donne l'état final (mouvements réduits). Les propriétés d'un membre plus loin dans la
/// liste remplacent celles des précédents ; un membre qui anime une couleur demande ses
/// images même si son effet est constant (couleurs échantillonnées avec les styles).
pub(super) fn animation_sample(
    a: Animation,
    clock: Option<(&Clock, Instant)>,
) -> (Effect, Option<Instant>) {
    let mut effect = Effect::NONE;
    let mut next = None;
    let grouped = !a.group.is_empty();
    for (index, m) in a.members().iter().enumerate() {
        let offset = a.offset(m);
        let elapsed = clock.map(|(c, now)| c.elapsed(index, offset, now));
        let (t, due) = m.progress(elapsed);
        let mut wake = m.animates_colors();
        if m.properties & property::EFFECTS != 0 {
            let value = match m.constant_effect() {
                Some(v) if m.fill == FillMode::Both || v == Effect::NONE => Some(v),
                _ => {
                    wake = true;
                    t.map(|t| m.effect_at(t))
                }
            };
            if let Some(mut v) = value {
                if grouped {
                    v.x *= a.distance_scale;
                    v.y *= a.distance_scale;
                }
                if m.properties & property::OPACITY != 0 {
                    effect.opacity = v.opacity;
                }
                if m.properties & property::TRANSFORM != 0 {
                    effect = Effect {
                        opacity: effect.opacity,
                        ..v
                    };
                }
            }
        }
        if let Some((c, now)) = clock
            && wake
            && !m.paused
        {
            next = earliest(next, c.due(index, offset, due, now));
        }
    }
    (effect, next)
}
pub(super) struct Track {
    pub(super) spec: Motion,
    pub(super) target: Effect,
    pub(super) base: Effect,
    pub(super) tween: Option<Tween>,
    pub(super) clock: Clock,
    pub(super) value: Effect,
    pub(super) visible: bool,
    pub(super) seen: u64,
}
impl Track {
    /// Échantillonne la piste ; renvoie le prochain réveil et si l'animation de l'élément
    /// anime encore des couleurs (résolution des styles à chaque image).
    pub(super) fn sample(&mut self, now: Instant, reduced: bool) -> (Option<Instant>, bool) {
        let mut next = None;
        if reduced {
            self.base = self.target;
            self.tween = None;
        } else if let Some(tween) = self.tween {
            let (value, due) = tween.sample(now);
            self.base = value;
            next = due;
            if due.is_none() {
                self.tween = None;
            }
        }
        let mut colors = false;
        let entry = self.spec.animation.map_or(Effect::NONE, |a| {
            let clock = (!reduced).then_some((&self.clock, now));
            let (effect, due) = animation_sample(a, clock);
            colors = due.is_some() && a.animates_colors();
            next = earliest(next, due);
            effect
        });
        self.value = self.base.combine(entry);
        let visible = self.visible && !reduced;
        (next.filter(|_| visible), colors && visible)
    }
}
