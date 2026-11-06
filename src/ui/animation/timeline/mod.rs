//! État conservé uniquement pour les éléments qui optent pour un effet.
mod ghost;
#[cfg(test)]
mod tests;
mod track;
mod value;
use crate::ui::theme::Color;
use crate::ui::{
    element::{Element, ElementId},
    layout,
    rendering::transform::Affine,
    scene::{PointF, RectF},
};
use ghost::Ghost;
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};
use track::{Clock, Track, Tween};
pub(crate) use value::Interpolate;
use value::ValueTracks;
const FRAME: Duration = Duration::from_nanos(16_666_667);
fn earliest(a: Option<Instant>, b: Option<Instant>) -> Option<Instant> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    }
}
#[derive(Clone, Copy)]
pub(crate) struct Presented {
    pub matrix: Affine,
    pub clip: RectF,
}
#[derive(Default)]
pub(crate) struct Timeline {
    /// Transitions de couleurs CSS, échantillonnées pendant la résolution des styles.
    pub colors: std::cell::RefCell<ValueTracks<Color>>,
    /// Transitions de dimensions CSS (`width`, `height`), échantillonnées de même ;
    /// chaque image relance le layout.
    pub lengths: std::cell::RefCell<ValueTracks<f32>>,
    tracks: HashMap<ElementId, Track>,
    frames: HashMap<ElementId, Presented>,
    now: Option<Instant>,
    next: Option<Instant>,
    pub reduced: bool,
    revision: u64,
    geometry_dirty: bool,
    epoch: u64,
    ghosts: Vec<Ghost>,
    /// Une animation visible anime des couleurs : la résolution des styles suit chaque
    /// image (mis à jour à chaque échantillonnage des pistes).
    animated_colors: bool,
}
impl Timeline {
    /// Une transition de largeur ou de hauteur est en cours (relayout à chaque image).
    pub fn animating_lengths(&self) -> bool {
        self.lengths.borrow().animating()
    }
    /// Transitions de couleurs ou de longueurs en cours : échantillonnées pendant la
    /// résolution des styles, qui doit alors suivre chaque image.
    pub fn animating_styles(&self) -> bool {
        self.animated_colors
            || self.colors.borrow().animating()
            || self.lengths.borrow().animating()
    }
    /// Couleurs (texte, fond, bordure) de l'élément `id` sous son animation `animation` à
    /// `now`, à partir de ses couleurs calculées `colors`. L'horloge est celle de la piste
    /// de l'élément ; une animation nouvelle ou remplacée commence à `now`.
    pub fn animate_colors(
        &self,
        id: ElementId,
        animation: crate::ui::animation::Animation,
        colors: [Option<Color>; 3],
        now: Instant,
    ) -> [Option<Color>; 3] {
        let fresh;
        let clock = match self
            .tracks
            .get(&id)
            .filter(|t| t.spec.animation.is_some_and(|a| a.same_run(animation)))
        {
            Some(track) => &track.clock,
            None => {
                fresh = Clock::new(now, Some(animation));
                &fresh
            }
        };
        let mut out = colors;
        for (index, member) in animation.members().iter().enumerate() {
            if !member.animates_colors() {
                continue;
            }
            let offset = animation.offset(member);
            let elapsed = (!self.reduced).then(|| clock.elapsed(index, offset, now));
            if let (Some(t), _) = member.progress(elapsed) {
                member.colors_at(t, colors, &mut out);
            }
        }
        out
    }
    pub fn has_tracks(&self) -> bool {
        !self.tracks.is_empty()
            || self.colors.borrow().animating()
            || self.lengths.borrow().animating()
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn next_wake(&self) -> Option<Instant> {
        earliest(
            earliest(self.next, self.colors.borrow().next),
            self.lengths.borrow().next,
        )
    }
    /// Décale toutes les horloges de `delta` : une image préparée avant l'ouverture de
    /// la fenêtre garde ses animations à leur début jusqu'à la première présentation.
    pub fn shift(&mut self, delta: Duration) {
        let shift = |t: &mut Instant| *t = t.checked_add(delta).unwrap_or(*t);
        for track in self.tracks.values_mut() {
            track.clock.shift(delta);
            if let Some(tween) = &mut track.tween {
                shift(&mut tween.start);
            }
        }
        for ghost in &mut self.ghosts {
            ghost.clock.shift(delta);
        }
        if let Some(now) = &mut self.now {
            shift(now);
        }
        if let Some(next) = &mut self.next {
            shift(next);
        }
    }
    pub fn time(&self) -> Instant {
        self.now.unwrap_or_else(Instant::now)
    }
    pub fn tick(&mut self, now: Instant) -> bool {
        let now = self.now.map_or(now, |old| old.max(now));
        self.now = Some(now);
        let pending = self.next.filter(|due| *due > now);
        self.next = None;
        let mut changed = !self.ghosts.is_empty();
        self.animated_colors = false;
        if self.reduced {
            self.ghosts.clear();
        }
        self.ghosts.retain(|g| !g.done(now));
        if !self.ghosts.is_empty() {
            self.next = now.checked_add(FRAME);
        }
        for track in self.tracks.values_mut() {
            if !track.visible {
                continue;
            }
            let old = track.value;
            let (due, colors) = track.sample(now, self.reduced);
            self.next = earliest(self.next, due);
            self.animated_colors |= colors;
            changed |= track.visible && old != track.value;
            if track.visible && old != track.value && (old.spatial() || track.value.spatial()) {
                self.geometry_dirty = true;
            }
        }
        // Les entrées et prepare peuvent échantillonner entre deux images.
        // Ils ne doivent pas repousser un réveil encore en attente.
        if self.next.is_some() {
            self.next = earliest(self.next, pending);
        }
        changed
    }
    pub fn sync(
        &mut self,
        root: &Element,
        hovered: Option<ElementId>,
        pressed: Option<ElementId>,
        now: Instant,
    ) -> bool {
        let mut changed = self.tick(now);
        let now = self.time();
        self.epoch = self.epoch.wrapping_add(1);
        fn visit(
            node: &Element,
            hovered: Option<ElementId>,
            pressed: Option<ElementId>,
            timeline: &mut Timeline,
            now: Instant,
            changed: &mut bool,
        ) -> (bool, bool) {
            let (mut hot, mut down) = (hovered == Some(node.id), pressed == Some(node.id));
            for child in &node.children {
                let (h, d) = visit(child, hovered, pressed, timeline, now, changed);
                hot |= h;
                down |= d;
            }
            if let Some(spec) = node.motion.as_deref().copied()
                && !spec.exit_only()
            {
                let target = spec.target(hot, down);
                let visible = layout::width(node.clip) > 0. && layout::height(node.clip) > 0.;
                let t = timeline.tracks.entry(node.id).or_insert_with(|| Track {
                    spec,
                    target,
                    base: target,
                    tween: None,
                    clock: Clock::new(now, spec.animation),
                    value: target,
                    visible,
                    seen: timeline.epoch,
                });
                if t.spec.animation != spec.animation {
                    // Seul `animation-play-state` change : pause ou reprise sans redémarrer.
                    match (t.spec.animation, spec.animation) {
                        (Some(old), Some(new)) if old.same_run(new) => t.clock.update(new, now),
                        _ => t.clock = Clock::new(now, spec.animation),
                    }
                    *changed = true;
                }
                if t.target != target {
                    t.tween = spec
                        .transition
                        .filter(|t| !timeline.reduced && t.effects)
                        .map(|timing| Tween {
                            from: t.base,
                            to: target,
                            start: now,
                            timing,
                        });
                    if t.tween.is_none() {
                        t.base = target;
                    }
                    t.target = target;
                    *changed = true;
                }
                t.spec = spec;
                t.visible = visible;
                t.seen = timeline.epoch;
            }
            (hot, down)
        }
        visit(root, hovered, pressed, self, now, &mut changed);
        let count = self.tracks.len();
        self.tracks.retain(|_, t| t.seen == self.epoch);
        changed |= count != self.tracks.len();
        changed |= self.tick(now);
        self.frames.clear();
        if !self.tracks.is_empty() {
            fn present(
                node: &Element,
                parent: Affine,
                clip: RectF,
                viewport: RectF,
                timeline: &mut Timeline,
            ) {
                let (parent, clip) = if layout::fixed(node) {
                    (Affine::IDENTITY, viewport)
                } else {
                    (parent, clip)
                };
                let local = timeline.local(node).map_or(Affine::IDENTITY, |p| p.0);
                let world = local.then(parent);
                let clipped = layout::intersect(world.bounds(node.clip), clip);
                if world != Affine::IDENTITY {
                    timeline.frames.insert(
                        node.id,
                        Presented {
                            matrix: world,
                            clip: clipped,
                        },
                    );
                }
                for child in &node.children {
                    present(child, world, clipped, viewport, timeline);
                }
            }
            present(root, Affine::IDENTITY, root.clip, root.clip, self);
        }
        if self.geometry_dirty {
            self.revision = self.revision.wrapping_add(1);
            self.geometry_dirty = false;
        }
        changed
    }
    pub fn local(&self, node: &Element) -> Option<(Affine, f32)> {
        let t = self.tracks.get(&node.id)?;
        let matrix = t.value.matrix(node.rect, t.spec.origin);
        (matrix != Affine::IDENTITY || t.value.opacity != 1.).then_some((matrix, t.value.opacity))
    }
    pub fn frame(&self, id: ElementId) -> Option<Presented> {
        self.frames.get(&id).copied()
    }
    pub fn point(&self, id: ElementId, p: PointF) -> Option<PointF> {
        match self.frame(id) {
            Some(f) => Some(f.matrix.inverse()?.point(p)),
            None => Some(p),
        }
    }
    pub fn bounds(&self, id: ElementId, r: RectF) -> RectF {
        self.frame(id).map_or(r, |f| f.matrix.bounds(r))
    }
    pub fn clip(&self, node: &Element) -> RectF {
        self.frame(node.id).map_or(node.clip, |f| f.clip)
    }
}
