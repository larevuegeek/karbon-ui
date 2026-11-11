//! Défilement doux à la molette, comme Chromium et Firefox : un cran de molette
//! ([`UiEvent::Wheel`]) n'avance pas le contenu d'un coup ; le conteneur glisse vers sa
//! cible en 150 ms et s'arrête en douceur. Les crans suivants allongent la cible en
//! gardant la vitesse acquise (raccord sans à-coup, comme `UpdateTarget` de Chromium) :
//! une rotation continue de la molette donne un mouvement continu, pas une suite
//! d'élans. Le défilement précis (pavé tactile, `UiEvent::Scroll`) reste immédiat, comme
//! les mouvements réduits.
use super::*;
use std::time::Duration;

/// Durée d'un glissement.
const DURATION: Duration = Duration::from_millis(150);
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

    /// Glissement de `from` à `to` qui part à la vitesse `velocity`.
    fn new(id: ElementId, from: f32, to: f32, velocity: f32, now: Instant) -> Self {
        let distance = to - from;
        let mut duration = DURATION.as_secs_f32();
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

impl UiRuntime {
    /// Lance (ou allonge) le glissement du conteneur `id` de `delta` ; renvoie la part du
    /// déplacement qu'il prend (le reste passe aux conteneurs englobants).
    pub(super) fn smooth_scroll_by(&mut self, id: ElementId, delta: f32) -> f32 {
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
            // Raccord à l'instant du cran : position et vitesse de la courbe en cours
            // (`UpdateTarget` de Chromium). Repartir du dernier décalage affiché perdait le
            // chemin parcouru depuis la dernière image : l'image suivante ne bougeait pas.
            let (from, velocity) = current.map_or((s.offset, 0.), |m| {
                (m.position(now).clamp(0., s.max_offset()), m.velocity(now))
            });
            self.components.scrolls.smooth = Some(SmoothScroll::new(id, from, to, velocity, now));
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

    /// Prochaine image d'un glissement en cours.
    pub(super) fn smooth_scroll_wake(&self) -> Option<Instant> {
        self.components.scrolls.smooth.map(|m| m.next)
    }
}
