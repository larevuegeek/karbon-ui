//! Affichage réel des images pendant la sonde de défilement (`KARBON_SCROLL_PROBE`) :
//! `wp_presentation` dit quand chaque image a paru à l'écran, ou qu'elle a été écartée.
//! Résumé commun à macOS dans `platform/shared/presentation.rs`.
use super::display::{Event, Kind, NEW, Ptr, create};
use super::protocol::{WP_PRESENTATION_FEEDBACK, wp_presentation, wp_presentation_feedback};
use crate::platform::linux::ffi::wayland::wl_proxy_destroy;
use crate::platform::shared::presentation::Shown;

pub(super) struct Presentation {
    /// Global `wp_presentation`, lié seulement pendant la sonde (nul sinon).
    global: Ptr,
    /// Retours attendus, un par image envoyée.
    pending: Vec<Ptr>,
    /// Instants d'affichage, horloge du compositeur.
    shown: Shown,
}

impl Presentation {
    pub fn new(global: Ptr) -> Self {
        Self {
            global,
            pending: Vec::new(),
            shown: Shown::default(),
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
                self.shown.shown(
                    seconds as f64 * 1e3 + f64::from(event.uint(2)) / 1e6,
                    f64::from(event.uint(3)) / 1e6,
                );
            }
            wp_presentation_feedback::EV_DISCARDED => self.shown.discarded(),
            // `sync_output` : la sortie qui a servi, sans intérêt ici.
            _ => return true,
        }
        let feedback = self.pending.swap_remove(i);
        // SAFETY : retour terminé par cet événement, sans requête destructrice.
        unsafe { wl_proxy_destroy(feedback) };
        true
    }
}

impl Drop for Presentation {
    fn drop(&mut self) {
        self.shown.report("wp_presentation");
        for feedback in self.pending.drain(..) {
            // SAFETY : retours en attente, oubliés avec la fenêtre.
            unsafe { wl_proxy_destroy(feedback) };
        }
    }
}
