//! Défilement doux à la molette, comme Chromium et Firefox : un cran de molette
//! ([`UiEvent::Wheel`]) n'avance pas le contenu d'un coup ; le conteneur glisse vers sa
//! cible en 150 ms et s'arrête en douceur. Les crans suivants allongent la cible en
//! gardant la vitesse acquise (raccord sans à-coup, comme `UpdateTarget` de Chromium) :
//! une rotation continue de la molette donne un mouvement continu, pas une suite
//! d'élans. Le défilement précis (pavé tactile, `UiEvent::Scroll`) reste immédiat, comme
//! les mouvements réduits.
use super::*;
use std::time::Duration;

/// Durée d'un glissement de molette.
const DURATION: Duration = Duration::from_millis(150);
/// Durée d'un glissement demandé par l'application (`scroll_request`,
/// `scroll_into_view`) : plus long qu'un cran, il part et arrive en douceur.
const REQUEST_DURATION: Duration = Duration::from_millis(450);
/// Une image.
const FRAME: Duration = Duration::from_nanos(16_666_667);
/// Pente initiale (normalisée) maximale : au-delà, la courbe dépasserait sa cible.
const MAX_SLOPE: f32 = 3.;

/// Glissement en cours d'un conteneur.
#[derive(Clone, Copy)]
pub(super) struct SmoothScroll {
    pub(super) id: ElementId,
    from: f32,
    pub(super) to: f32,
    start: Instant,
    /// Durée du glissement, en secondes.
    duration: f32,
    /// Pente initiale de la courbe normalisée (3 : sortie cubique depuis l'arrêt).
    slope: f32,
    /// Réveil demandé pour l'image suivante (simple indication de cadence).
    next: Instant,
}

impl SmoothScroll {
    /// Avancement normalisé à l'instant `now`.
    fn progress(&self, now: Instant) -> f32 {
        (now.saturating_duration_since(self.start).as_secs_f32() / self.duration).clamp(0., 1.)
    }

    /// Courbe cubique de pente initiale `slope` et d'arrivée à vitesse nulle :
    /// h(0) = 0, h'(0) = slope, h(1) = 1, h'(1) = 0.
    fn curve(&self, t: f32) -> (f32, f32) {
        let c = self.slope;
        let (a, b) = (c - 2., 3. - 2. * c);
        let h = ((a * t + b) * t + c) * t;
        let dh = (3. * a * t + 2. * b) * t + c;
        (h, dh)
    }

    /// Position à l'instant `now`.
    fn position(&self, now: Instant) -> f32 {
        self.from + (self.to - self.from) * self.curve(self.progress(now)).0
    }

    /// Vitesse (pixels par seconde) à l'instant `now`.
    fn velocity(&self, now: Instant) -> f32 {
        let t = self.progress(now);
        if t >= 1. {
            return 0.;
        }
        (self.to - self.from) * self.curve(t).1 / self.duration
    }

    /// Glissement de `from` à `to`, en `duration`, qui part à la vitesse `velocity`.
    fn new(
        id: ElementId,
        (from, to): (f32, f32),
        velocity: f32,
        duration: Duration,
        now: Instant,
    ) -> Self {
        let distance = to - from;
        let mut duration = duration.as_secs_f32();
        let mut slope = if distance.abs() > f32::EPSILON {
            velocity * duration / distance
        } else {
            0.
        };
        // Vitesse acquise contraire : on repart de l'arrêt (pas de recul visible).
        if slope < 0. {
            slope = 0.;
        }
        // Trop rapide pour la distance restante : le glissement se raccourcit pour garder
        // la vitesse sans dépasser la cible.
        if slope > MAX_SLOPE {
            duration = (MAX_SLOPE * distance / velocity)
                .abs()
                .max(FRAME.as_secs_f32());
            slope = MAX_SLOPE;
        }
        Self {
            id,
            from,
            to,
            start: now,
            duration,
            slope,
            next: now,
        }
    }
}

impl super::components::ScrollState {
    /// Le contenu du conteneur `id` s'est déplacé de `delta` (ancrage du défilement) : le
    /// glissement en cours le suit, sans revenir vers l'ancienne position.
    pub(super) fn shift_smooth(&mut self, id: ElementId, delta: f32) {
        if let Some(m) = self.smooth.as_mut().filter(|m| m.id == id) {
            m.from += delta;
            m.to += delta;
        }
    }
}

/// Demande à appliquer : conteneur, demande, conteneur déjà affiché à l'image précédente.
pub(super) type ScrollRequestAt = (ElementId, crate::ui::scroll::ScrollRequest, bool);

/// Demandes nouvelles de l'arbre disposé (jeton différent du dernier appliqué), relevées
/// avant que `ScrollState::save` n'enregistre les conteneurs de cette image.
pub(super) fn scroll_requests(
    root: &Element,
    state: &mut super::components::ScrollState,
) -> Vec<ScrollRequestAt> {
    fn collect(node: &Element, out: &mut Vec<(ElementId, crate::ui::scroll::ScrollRequest)>) {
        if node.scroll.is_some()
            && let Some(r) = node.rules.as_ref().and_then(|r| r.scroll_request)
        {
            out.push((node.id, r));
        }
        for child in &node.children {
            collect(child, out);
        }
    }
    /// Éléments `scroll_into_view` et le décalage qui amène chacun en haut de la vue de
    /// son conteneur défilant vertical le plus proche (élément, conteneur, décalage).
    fn into_view(
        node: &Element,
        scroller: Option<(ElementId, crate::ui::scroll::ScrollMetrics, f32)>,
        out: &mut Vec<(ElementId, ElementId, f32)>,
    ) {
        let rules = node.rules.as_ref();
        if rules.is_some_and(|r| r.scroll_into_view)
            && let Some((id, s, padding)) = scroller
        {
            let margin = rules.map_or(0., |r| r.scroll_margin.top);
            out.push((
                node.id,
                id,
                s.offset + node.rect.top - margin - (s.viewport.top + padding),
            ));
        }
        // Contenu sauté : ses descendants n'ont pas de place à jour.
        if node.content_skipped {
            return;
        }
        let scroller = match node.scroll {
            Some(s) if s.axis == Axis::Vertical => {
                Some((node.id, s, rules.map_or(0., |r| r.scroll_padding.top)))
            }
            _ => scroller,
        };
        for child in &node.children {
            into_view(child, scroller, out);
        }
    }
    let mut found = Vec::new();
    collect(root, &mut found);
    let mut requests: Vec<ScrollRequestAt> = found
        .into_iter()
        .filter(|(id, r)| state.requests.insert(*id, r.token) != Some(r.token))
        .map(|(id, r)| (id, r, state.offsets.contains_key(&id)))
        .collect();
    let mut views = Vec::new();
    into_view(root, None, &mut views);
    let previous = std::mem::take(&mut state.into_view);
    for (element, scroller, offset) in views {
        state.into_view.insert(element);
        if !previous.contains(&element) {
            let r = crate::ui::scroll::ScrollRequest {
                token: 0,
                offset,
                smooth: true,
            };
            requests.push((scroller, r, state.offsets.contains_key(&scroller)));
        }
    }
    requests
}

impl UiRuntime {
    /// Lance (ou allonge) le glissement du conteneur `id` de `delta` ; renvoie la part du
    /// déplacement qu'il prend (le reste passe aux conteneurs englobants).
    pub(super) fn smooth_scroll_by(&mut self, id: ElementId, delta: f32) -> f32 {
        self.glide_by(id, delta, DURATION)
    }

    /// Comme [`smooth_scroll_by`](Self::smooth_scroll_by), en `duration`.
    fn glide_by(&mut self, id: ElementId, delta: f32, duration: Duration) -> f32 {
        let Some(s) = self
            .root
            .as_ref()
            .and_then(|r| r.find(id))
            .and_then(|n| n.scroll)
        else {
            return 0.;
        };
        // L'horloge des animations peut être en avance sur l'horloge murale (images
        // datées par la plateforme, tests) : le glissement part de la plus récente.
        let now = Instant::now().max(self.motions.time());
        let current = self.components.scrolls.smooth.filter(|m| m.id == id);
        let base = current.map_or(s.offset, |m| m.to);
        let to = (base + delta).clamp(0., s.max_offset());
        let consumed = to - base;
        if consumed != 0. {
            // Un seul glissement à la fois : celui d'un autre conteneur (conteneur intérieur
            // dont ce cran dépasse le bout, ou conteneur quitté par le pointeur) arrive tout
            // de suite à sa cible au lieu d'être abandonné en chemin.
            if let Some(other) = self.components.scrolls.smooth.filter(|m| m.id != id) {
                self.scroll_to(other.id, other.to);
            }
            // Raccord à l'instant du cran : position et vitesse de la courbe en cours
            // (`UpdateTarget` de Chromium). Repartir du dernier décalage affiché perdait le
            // chemin parcouru depuis la dernière image : l'image suivante ne bougeait pas.
            let (from, velocity) = current.map_or((s.offset, 0.), |m| {
                (m.position(now).clamp(0., s.max_offset()), m.velocity(now))
            });
            self.components.scrolls.smooth =
                Some(SmoothScroll::new(id, (from, to), velocity, duration, now));
        }
        consumed
    }

    /// Avance le glissement en cours à l'instant `now` ; `true` si le contenu a bougé.
    /// Chaque image échantillonne la courbe à son instant : une image en avance ou en
    /// retard sur la cadence prévue avance d'autant, sans être sautée.
    pub(super) fn smooth_scroll_tick(&mut self, now: Instant) -> bool {
        let Some(mut m) = self.components.scrolls.smooth else {
            return false;
        };
        // La barre saisie reprend la main.
        if self.components.scrolls.drag.is_some() {
            self.components.scrolls.smooth = None;
            return false;
        }
        let moved = self.scroll_to(m.id, m.position(now)) != 0.;
        if m.progress(now) >= 1. {
            self.components.scrolls.smooth = None;
        } else {
            m.next = now + FRAME;
            self.components.scrolls.smooth = Some(m);
        }
        moved
    }

    /// Défilements demandés par l'application (`scroll_request`, relevés par
    /// [`scroll_requests`]) : chaque nouveau jeton amène son conteneur à la position
    /// voulue, en glissant s'il était déjà affiché, d'un coup s'il apparaît.
    pub(super) fn apply_scroll_requests(&mut self, requests: Vec<ScrollRequestAt>) {
        for (id, r, known) in requests {
            if r.smooth && known {
                self.smooth_scroll_to(id, r.offset);
            } else {
                if self.components.scrolls.smooth.is_some_and(|m| m.id == id) {
                    self.components.scrolls.smooth = None;
                }
                self.scroll_to(id, r.offset);
            }
        }
    }

    /// Glisse le conteneur `id` jusqu'à `offset` (borné à sa plage de défilement), à
    /// l'allure d'un glissement demandé par l'application.
    pub(super) fn smooth_scroll_to(&mut self, id: ElementId, offset: f32) {
        let current = self
            .components
            .scrolls
            .smooth
            .filter(|m| m.id == id)
            .map(|m| m.to);
        let Some(base) = current.or_else(|| {
            self.root
                .as_ref()
                .and_then(|r| r.find(id))
                .and_then(|n| n.scroll)
                .map(|s| s.offset)
        }) else {
            return;
        };
        self.glide_by(id, offset - base, REQUEST_DURATION);
    }

    /// Prochaine image d'un glissement en cours.
    pub(super) fn smooth_scroll_wake(&self) -> Option<Instant> {
        self.components.scrolls.smooth.map(|m| m.next)
    }
}
