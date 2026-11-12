//! Horloge de l'écran d'une fenêtre : un lien d'affichage (`NSView.displayLink`, macOS 14)
//! bat à chaque rafraîchissement, comme le rappel de trame de Wayland. Une image présentée
//! attend le battement suivant avant la prochaine ; au battement, les animations sont
//! échantillonnées tout de suite (comme `requestAnimationFrame`) et l'image part.
//! Sans lien (macOS plus ancien), les images suivent les échéances du runtime.
//!
//! Mesure du 2026-11-12 (MacBook Air M1, 60 Hz, clone RustMusic, sonde 30:8) : avant ce
//! lien, ~69 images envoyées par seconde, une sur cinq remplacée avant d'être montrée et
//! ~165 rafraîchissements sans image neuve en 30 s ; avec lui, 60 images/s, 6 à 13
//! écartées et 0 à 7 rafraîchissements manqués (médiane 2).
use super::{Instant, View};
use objc2::{rc::Retained, runtime::AnyObject, sel};
use objc2_foundation::{NSRunLoop, NSRunLoopCommonModes};
use objc2_quartz_core::CADisplayLink;
use std::time::Duration;

/// Battements sans rien à dessiner avant de mettre le lien en pause.
const IDLE_TICKS: u32 = 4;
/// Attente maximale d'un battement après une image (fenêtre masquée : l'écran ne bat plus
/// pour elle) ; au-delà, l'image suivante part sans lui.
const TICK_TIMEOUT: Duration = Duration::from_millis(50);

pub(in crate::platform::macos) struct FrameLink {
    link: Retained<CADisplayLink>,
    paused: bool,
    /// Instant de la dernière image présentée, tant que le battement suivant n'est pas
    /// arrivé.
    awaiting: Option<Instant>,
    idle: u32,
}

impl FrameLink {
    /// Lien de `view`, qui appelle `karbonDisplayLink:` à chaque rafraîchissement ;
    /// `None` avant macOS 14.
    pub fn new(view: &View) -> Option<Self> {
        // SAFETY : `respondsToSelector:` de NSObject, sans effet.
        let responds: bool = unsafe {
            objc2::msg_send![view, respondsToSelector: sel!(displayLinkWithTarget:selector:)]
        };
        if !responds {
            return None;
        }
        // SAFETY : `view` répond à `karbonDisplayLink:` (`view.rs`) ; le lien retient la vue
        // jusqu'à `invalidate` (`Drop`).
        let link = unsafe {
            view.displayLinkWithTarget_selector(
                &*(view as *const View).cast::<AnyObject>(),
                sel!(karbonDisplayLink:),
            )
        };
        link.setPaused(true);
        // SAFETY : boucle principale, modes communs (attente des événements et suivi d'un
        // geste de redimensionnement).
        unsafe { link.addToRunLoop_forMode(&NSRunLoop::mainRunLoop(), NSRunLoopCommonModes) };
        Some(Self {
            link,
            paused: true,
            awaiting: None,
            idle: 0,
        })
    }

    /// Une image vient d'être présentée : la suivante attend le prochain battement.
    pub fn presented(&mut self, now: Instant) {
        self.awaiting = Some(now);
        self.idle = 0;
        if self.paused {
            self.paused = false;
            self.link.setPaused(false);
        }
    }

    /// Une image peut partir maintenant.
    pub fn ready(&self, now: Instant) -> bool {
        self.awaiting
            .is_none_or(|since| now >= since + TICK_TIMEOUT)
    }

    /// Échéance au-delà de laquelle l'image part sans battement, tant qu'il est attendu.
    pub fn timeout(&self) -> Option<Instant> {
        self.awaiting.map(|since| since + TICK_TIMEOUT)
    }

    /// Battement de l'écran ; `busy` : la fenêtre a une image à faire ou une animation
    /// imminente. Après quelques battements sans rien, le lien se met en pause.
    pub fn tick(&mut self, busy: bool) {
        self.awaiting = None;
        if busy {
            self.idle = 0;
            return;
        }
        self.idle += 1;
        if self.idle >= IDLE_TICKS && !self.paused {
            self.paused = true;
            self.link.setPaused(true);
        }
    }
}

impl Drop for FrameLink {
    fn drop(&mut self) {
        // Le lien relâche sa cible (la vue) et quitte la boucle.
        self.link.invalidate();
    }
}
