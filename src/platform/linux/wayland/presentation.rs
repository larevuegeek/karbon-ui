//! Affichage réel des images pendant la sonde de défilement (`KARBON_SCROLL_PROBE`). La
//! sonde du runtime date les envois d'images ; `wp_presentation` dit quand chacune a paru
//! à l'écran, ou qu'elle a été écartée (remplacée avant d'être montrée). Le résumé, écrit
//! à la fermeture de la fenêtre, distingue ainsi un rappel de trame en retard d'une image
//! vraiment manquée à l'écran.
use super::display::{Event, Kind, NEW, Ptr, create};
use super::protocol::{WP_PRESENTATION_FEEDBACK, wp_presentation, wp_presentation_feedback};
use crate::platform::linux::ffi::wayland::wl_proxy_destroy;

/// Écart au-delà duquel un affichage compte pour un à-coup (comme la sonde).
const JANK_MS: f64 = 25.;
/// Écart au-delà duquel plus rien ne bougeait : compté à part (comme la sonde).
const STOP_MS: f64 = 250.;

pub(super) struct Presentation {
    /// Global `wp_presentation`, lié seulement pendant la sonde (nul sinon).
    global: Ptr,
    /// Retours attendus, un par image envoyée.
    pending: Vec<Ptr>,
    /// Instants d'affichage, horloge du compositeur, en millisecondes.
    shown: Vec<f64>,
    /// Période de rafraîchissement annoncée, en millisecondes.
    refresh: f64,
    discarded: usize,
}

impl Presentation {
    pub fn new(global: Ptr) -> Self {
        Self {
            global,
            pending: Vec::new(),
            shown: Vec::new(),
            refresh: 0.,
            discarded: 0,
        }
    }

    /// Demande l'instant d'affichage de l'image que portera le prochain envoi de `surface`.
    pub fn request(&mut self, surface: Ptr) {
        if self.global.is_null() {
            return;
        }
        let feedback = create!(
            self.global,
            wp_presentation::FEEDBACK,
            &WP_PRESENTATION_FEEDBACK,
            Kind::Feedback,
            surface,
            NEW
        );
        if !feedback.is_null() {
            self.pending.push(feedback);
        }
    }

    /// Événement d'un retour ; `false` s'il n'est pas à cette fenêtre.
    pub fn event(&mut self, event: &Event) -> bool {
        let Some(i) = self.pending.iter().position(|p| *p == event.target) else {
            return false;
        };
        match event.opcode {
            wp_presentation_feedback::EV_PRESENTED => {
                let seconds = (u64::from(event.uint(0)) << 32) | u64::from(event.uint(1));
                self.shown
                    .push(seconds as f64 * 1e3 + f64::from(event.uint(2)) / 1e6);
                self.refresh = f64::from(event.uint(3)) / 1e6;
            }
            wp_presentation_feedback::EV_DISCARDED => self.discarded += 1,
            // `sync_output` : la sortie qui a servi, sans intérêt ici.
            _ => return true,
        }
        let feedback = self.pending.swap_remove(i);
        // SAFETY : retour terminé par cet événement, sans requête destructrice.
        unsafe { wl_proxy_destroy(feedback) };
        true
    }

    /// Résumé des affichages de la mesure (ses dernières secondes) sur la sortie d'erreur.
    fn report(&self) {
        let duration = std::env::var("KARBON_SCROLL_PROBE")
            .ok()
            .and_then(|v| v.split(':').next()?.parse::<f64>().ok());
        let (Some(duration), Some(&last)) = (duration, self.shown.last()) else {
            return;
        };
        let shown: Vec<f64> = self
            .shown
            .iter()
            .copied()
            .filter(|t| *t >= last - duration * 1e3)
            .collect();
        let gaps: Vec<f64> = shown.windows(2).map(|w| w[1] - w[0]).collect();
        let mut moving: Vec<f64> = gaps.iter().copied().filter(|g| *g <= STOP_MS).collect();
        moving.sort_by(f64::total_cmp);
        let Some(&max) = moving.last() else {
            return;
        };
        let percentile = |p: usize| moving[(moving.len() - 1) * p / 100];
        // Rafraîchissements sans image nouvelle entre deux affichages.
        let missed: f64 = if self.refresh > 0. {
            moving
                .iter()
                .map(|g| ((g / self.refresh).round() - 1.).max(0.))
                .sum()
        } else {
            0.
        };
        eprintln!(
            "karbon-ui: sonde, affichage (wp_presentation) : {} images, écart médian {:.1} ms, p95 {:.1} ms, max {:.1} ms, à-coups {}, rafraîchissements manqués {missed}, écartées {}, période {:.2} ms",
            shown.len(),
            percentile(50),
            percentile(95),
            max,
            moving.iter().filter(|g| **g > JANK_MS).count(),
            self.discarded,
            self.refresh,
        );
    }
}

impl Drop for Presentation {
    fn drop(&mut self) {
        self.report();
        for feedback in self.pending.drain(..) {
            // SAFETY : retours en attente, oubliés avec la fenêtre.
            unsafe { wl_proxy_destroy(feedback) };
        }
    }
}
