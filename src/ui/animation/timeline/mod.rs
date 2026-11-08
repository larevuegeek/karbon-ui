//! État conservé uniquement pour les éléments qui optent pour un effet.
mod ghost;
mod paths;
#[cfg(test)]
mod tests;
mod track;
mod value;
use crate::ui::theme::Color;
use crate::ui::{
    animation::Animation,
    element::{Element, ElementId, Length},
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
    /// Découpe de l'élément dans son repère local (celui du layout), recalculée sous la
    /// transformation : les découpes du layout sont prises avant les transformations, si
    /// bien qu'un contenu translaté depuis l'extérieur d'un ancêtre `overflow: hidden`
    /// (piste de carrousel) y serait vide.
    pub local: RectF,
}
/// Découpe sans limite, pour une transformation non inversible (finie, pour les bornes
/// des calques).
const UNBOUNDED: RectF = RectF {
    left: -1e7,
    top: -1e7,
    right: 1e7,
    bottom: 1e7,
};
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
    /// Une animation visible anime des couleurs ou des dimensions : la résolution des
    /// styles suit chaque image (mis à jour à chaque échantillonnage des pistes).
    animated_colors: bool,
    /// Une animation visible anime des dimensions : le layout suit chaque image.
    animated_sizes: bool,
    /// Branches de l'arbre qui mènent à un élément animé, et parent de chaque élément :
    /// une image où l'arbre et les styles n'ont pas changé ne parcourt que ces branches.
    paths: Option<paths::MotionPaths>,
}
impl Timeline {
    /// Une transition ou une animation de largeur ou de hauteur est en cours (relayout à
    /// chaque image).
    pub fn animating_lengths(&self) -> bool {
        self.animated_sizes || self.lengths.borrow().animating()
    }
    /// Transitions de couleurs ou de longueurs en cours : échantillonnées pendant la
    /// résolution des styles, qui doit alors suivre chaque image.
    pub fn animating_styles(&self) -> bool {
        self.animated_colors
            || self.colors.borrow().animating()
            || self.lengths.borrow().animating()
    }
    /// Horloge de l'animation `animation` de l'élément `id` : celle de sa piste, ou une
    /// horloge qui commence à `now` pour une animation nouvelle ou remplacée.
    fn clock(
        &self,
        id: ElementId,
        animation: Animation,
        now: Instant,
    ) -> std::borrow::Cow<'_, Clock> {
        match self
            .tracks
            .get(&id)
            .filter(|t| t.spec.animation.is_some_and(|a| a.same_run(animation)))
        {
            Some(track) => std::borrow::Cow::Borrowed(&track.clock),
            None => std::borrow::Cow::Owned(Clock::new(now, Some(animation))),
        }
    }
    /// Appelle `visit` avec chaque membre de `animation` qui vérifie `animates` et sa
    /// progression à `now`, dans l'ordre de la liste (un membre sans effet à cet instant
    /// est sauté).
    fn each_member(
        &self,
        id: ElementId,
        animation: Animation,
        now: Instant,
        animates: fn(Animation) -> bool,
        mut visit: impl FnMut(Animation, f32),
    ) {
        let clock = self.clock(id, animation, now);
        for (index, member) in animation.members().iter().enumerate() {
            if !animates(*member) {
                continue;
            }
            let offset = animation.offset(member);
            let elapsed = (!self.reduced).then(|| clock.elapsed(index, offset, now));
            if let (Some(t), _) = member.progress(elapsed) {
                visit(*member, t);
            }
        }
    }
    /// Couleurs (texte, fond, bordure) de l'élément `id` sous son animation `animation` à
    /// `now`, à partir de ses couleurs calculées `colors`. L'horloge est celle de la piste
    /// de l'élément ; une animation nouvelle ou remplacée commence à `now`.
    pub fn animate_colors(
        &self,
        id: ElementId,
        animation: Animation,
        colors: [Option<Color>; 3],
        now: Instant,
    ) -> [Option<Color>; 3] {
        let mut out = colors;
        self.each_member(id, animation, now, Animation::animates_colors, |m, t| {
            m.colors_at(t, colors, &mut out)
        });
        out
    }
    /// Largeur et hauteur de l'élément `id` sous son animation `animation` à `now`, à
    /// partir de ses dimensions déclarées `sizes` : `None` pour une dimension qu'aucun
    /// membre actif n'anime.
    pub fn animate_sizes(
        &self,
        id: ElementId,
        animation: Animation,
        sizes: [Length; 2],
        now: Instant,
    ) -> [Option<Length>; 2] {
        let mut out = [None; 2];
        self.each_member(id, animation, now, Animation::animates_sizes, |m, t| {
            m.sizes_at(t, sizes, &mut out)
        });
        out
    }
    /// L'arbre ou ses styles ont changé : les branches animées seront relevées de nouveau.
    pub fn invalidate_paths(&mut self) {
        self.paths = None;
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
        self.animated_sizes = false;
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
            let (due, styled) = track.sample(now, self.reduced);
            self.next = earliest(self.next, due);
            self.animated_colors |= styled.styles;
            self.animated_sizes |= styled.sizes;
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
        let paths = self
            .paths
            .take()
            .unwrap_or_else(|| paths::MotionPaths::new(root));
        // Ancêtres du survol et de l'appui : `hot`/`down` d'un élément animé sans
        // parcourir les branches sans animation.
        let hot_set = paths.chain(hovered);
        let down_set = paths.chain(pressed);
        fn visit(
            node: &Element,
            sets: (&[ElementId], &[ElementId]),
            paths: &paths::MotionPaths,
            timeline: &mut Timeline,
            now: Instant,
            changed: &mut bool,
        ) {
            // Contenu sauté (`content-visibility`) : ni peint, ni animé ; ses pistes sont
            // oubliées et repartent de leur valeur d'arrivée quand il revient.
            for child in node.children.iter().filter(|_| !node.content_skipped) {
                if paths.leads(child.id) {
                    visit(child, sets, paths, timeline, now, changed);
                }
            }
            let (hot, down) = (sets.0.contains(&node.id), sets.1.contains(&node.id));
            if let Some(spec) = node.motion.as_deref().copied()
                && !spec.exit_only()
            {
                let target = spec.target(hot, down);
                // Découpe du layout ; sous la transformation d'un ancêtre, `present`
                // ci-dessous la corrige.
                let visible = visible(node, spec, node.clip);
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
        }
        visit(root, (&hot_set, &down_set), &paths, self, now, &mut changed);
        let count = self.tracks.len();
        self.tracks.retain(|_, t| t.seen == self.epoch);
        changed |= count != self.tracks.len();
        changed |= self.tick(now);
        self.frames.clear();
        if !self.tracks.is_empty() {
            /// `clip` : découpe des ancêtres dans le repère de la fenêtre. `flipped` : la
            /// visibilité d'une piste a changé avec la découpe recalculée sous les
            /// transformations (contenu amené dans la vue ou sorti par une piste
            /// translatée, que la découpe du layout ignore).
            fn present(
                node: &Element,
                parent: Affine,
                clip: RectF,
                viewport: RectF,
                timeline: &mut Timeline,
                paths: &paths::MotionPaths,
                flipped: &mut bool,
            ) {
                let (parent, clip) = if layout::fixed(node) {
                    (Affine::IDENTITY, viewport)
                } else {
                    (parent, clip)
                };
                let own = timeline.local(node).map_or(Affine::IDENTITY, |p| p.0);
                let world = own.then(parent);
                let (clipped, shown) = if world != Affine::IDENTITY {
                    // Découpe des ancêtres ramenée dans le repère local, comme en CSS :
                    // celle du layout ignore la transformation.
                    let parent_clip = world
                        .inverse()
                        .map_or(UNBOUNDED, |inverse| inverse.bounds(clip));
                    let own_clip = layout::node_clip(node, node.rect, parent_clip);
                    let clipped = layout::intersect(world.bounds(own_clip), clip);
                    timeline.frames.insert(
                        node.id,
                        Presented {
                            matrix: world,
                            clip: clipped,
                            local: own_clip,
                        },
                    );
                    // Visibilité : la découpe recalculée seulement sous la transformation
                    // d'un ancêtre (celle de l'élément lui-même ne change pas ce que son
                    // layout montre), et vide sous des ancêtres déjà rognés (les bornes
                    // d'une découpe vide, tournée, ne le seraient plus).
                    let shown = if parent == Affine::IDENTITY {
                        node.clip
                    } else if layout::width(clip) <= 0. || layout::height(clip) <= 0. {
                        clip
                    } else {
                        own_clip
                    };
                    (clipped, shown)
                } else {
                    (layout::intersect(node.clip, clip), node.clip)
                };
                if let Some(track) = timeline.tracks.get_mut(&node.id) {
                    let now_visible = visible(node, track.spec, shown);
                    if track.visible != now_visible {
                        track.visible = now_visible;
                        *flipped = true;
                    }
                }
                for child in node.children.iter().filter(|_| !node.content_skipped) {
                    // Sans transformation au-dessus, une branche sans élément animé n'a
                    // aucune image présentée à retenir.
                    if world != Affine::IDENTITY || paths.leads(child.id) {
                        present(child, world, clipped, viewport, timeline, paths, flipped);
                    }
                }
            }
            let mut flipped = false;
            present(
                root,
                Affine::IDENTITY,
                root.clip,
                root.clip,
                self,
                &paths,
                &mut flipped,
            );
            if flipped {
                // Pistes entrées dans la vue (ou sorties) : échantillonnées dès cette image,
                // puis les images présentées refaites avec leurs nouvelles valeurs.
                changed = true;
                self.tick(now);
                self.frames.clear();
                present(
                    root,
                    Affine::IDENTITY,
                    root.clip,
                    root.clip,
                    self,
                    &paths,
                    &mut flipped,
                );
            }
        }
        self.paths = Some(paths);
        if self.geometry_dirty {
            self.revision = self.revision.wrapping_add(1);
            self.geometry_dirty = false;
        }
        changed
    }
    pub fn local(&self, node: &Element) -> Option<(Affine, f32)> {
        let Some(t) = self.tracks.get(&node.id) else {
            // Descendant d'un fantôme qui joue sa propre sortie.
            if self.ghosts.is_empty() {
                return None;
            }
            let now = self.time();
            return self.ghosts.iter().find_map(|g| g.part(node, now));
        };
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
    /// Découpe de `node` dans son repère local pour la peinture (posée après sa matrice).
    /// Sous une translation, celle recalculée (voir [`Presented::local`]) : exacte, elle
    /// montre le contenu qu'une piste translatée amène dans un ancêtre qui rogne (diapo
    /// d'un carrousel), que la découpe du layout cacherait. Sous une mise à l'échelle ou
    /// une rotation, celle du layout, approximation gardée (bornes des filtres d'un
    /// contenu agrandi, `scale-125` autour d'une image floue).
    pub fn paint_clip(&self, node: &Element) -> RectF {
        match self.frame(node.id) {
            Some(f)
                if f.matrix.m11 == 1.
                    && f.matrix.m22 == 1.
                    && f.matrix.m12 == 0.
                    && f.matrix.m21 == 0. =>
            {
                f.local
            }
            _ => node.clip,
        }
    }
    pub fn clip(&self, node: &Element) -> RectF {
        self.frame(node.id).map_or(node.clip, |f| f.clip)
    }
}
/// Un élément dont la boîte visible n'est pas vide demande ses images. Une animation de
/// taille peut partir d'une dimension nulle (barre de hauteur 0 pendant son délai) :
/// l'élément compte alors comme visible tant que son coin se trouve dans sa découpe.
/// `clip` : découpe de l'élément, celle du layout ou celle recalculée sous la
/// transformation d'un ancêtre.
fn visible(node: &Element, spec: crate::ui::animation::Motion, clip: RectF) -> bool {
    if layout::width(clip) > 0. && layout::height(clip) > 0. {
        return true;
    }
    let inside = |a: f32, low: f32, high: f32| a >= low && a <= high;
    spec.animation.is_some_and(Animation::animates_sizes)
        && inside(node.rect.left, clip.left, clip.right)
        && inside(node.rect.top, clip.top, clip.bottom)
}
