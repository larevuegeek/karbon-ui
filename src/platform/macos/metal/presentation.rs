//! Affichage réel des images Metal pendant la sonde de défilement (`KARBON_SCROLL_PROBE`) :
//! chaque drawable présenté reçoit un rappel (`addPresentedHandler`) qui relève son
//! instant d'affichage (`presentedTime`, horloge de `CACurrentMediaTime`), nul pour une
//! image écartée. Équivalent de `wp_presentation` sous Wayland ; le résumé
//! (`platform/shared/presentation.rs`) est écrit quand la fenêtre disparaît.
use crate::platform::shared::presentation::Shown;
use block2::RcBlock;
use objc2::{MainThreadMarker, runtime::ProtocolObject};
use objc2_app_kit::NSScreen;
use objc2_metal::MTLDrawable;
use std::ptr::NonNull;
use std::sync::{Arc, Mutex};

pub(super) struct Presentation {
    /// Partagé avec les rappels, appelés sur un fil de Metal.
    shown: Arc<Mutex<Shown>>,
}

impl Presentation {
    /// Relevé actif seulement pendant la sonde.
    pub fn from_env() -> Option<Self> {
        std::env::var_os("KARBON_SCROLL_PROBE").map(|_| Self {
            shown: Arc::default(),
        })
    }

    /// Demande l'instant d'affichage de `drawable`, à appeler avant sa présentation.
    pub fn watch(&self, drawable: &ProtocolObject<dyn MTLDrawable>) {
        // Période de l'écran principal (celui de la fenêtre clé), lue sur le fil
        // principal ; 0 : inconnue.
        let refresh = MainThreadMarker::new()
            .and_then(NSScreen::mainScreen)
            .map(|screen| screen.maximumFramesPerSecond())
            .filter(|fps| *fps > 0)
            .map_or(0., |fps| 1e3 / fps as f64);
        let shown = Arc::clone(&self.shown);
        let handler = RcBlock::new(move |drawable: NonNull<ProtocolObject<dyn MTLDrawable>>| {
            // SAFETY : Metal passe le drawable vivant le temps du rappel.
            let at = unsafe { drawable.as_ref() }.presentedTime();
            let Ok(mut shown) = shown.lock() else {
                return;
            };
            if at > 0. {
                shown.shown(at * 1e3, refresh);
            } else {
                shown.discarded();
            }
        });
        // SAFETY : Metal copie le bloc, qui ne retient que des valeurs partagées et
        // `Send` ; le drawable n'est pas encore présenté.
        unsafe { drawable.addPresentedHandler(RcBlock::as_ptr(&handler)) };
    }
}

impl Drop for Presentation {
    fn drop(&mut self) {
        if let Ok(shown) = self.shown.lock() {
            shown.report("MTLDrawable.presentedTime");
        }
    }
}
