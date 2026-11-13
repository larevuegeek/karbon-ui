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
    beat: Beat,
}

/// Attente du battement, sans AppKit (testée seule) : la dernière image présentée, les
/// battements à vide et l'état de pause du lien.
#[derive(Debug)]
struct Beat {
    paused: bool,
    /// Instant de la dernière image présentée, tant que le battement suivant n'est pas
    /// arrivé.
    awaiting: Option<Instant>,
    idle: u32,
}

impl Beat {
    /// Une image vient d'être présentée ; `true` si le lien doit reprendre.
    fn presented(&mut self, now: Instant) -> bool {
        self.awaiting = Some(now);
        self.idle = 0;
        std::mem::replace(&mut self.paused, false)
    }
    fn ready(&self, now: Instant) -> bool {
        self.awaiting
            .is_none_or(|since| now >= since + TICK_TIMEOUT)
    }
    fn timeout(&self) -> Option<Instant> {
        self.awaiting.map(|since| since + TICK_TIMEOUT)
    }
    fn expire(&mut self, now: Instant) -> bool {
        let expired = self
            .awaiting
            .is_some_and(|since| now >= since + TICK_TIMEOUT);
        if expired {
            self.awaiting = None;
        }
        expired
    }
    /// Battement ; `true` si le lien doit se mettre en pause.
    fn tick(&mut self, busy: bool) -> bool {
        self.awaiting = None;
        if busy {
            self.idle = 0;
            return false;
        }
        self.idle += 1;
        if self.idle >= IDLE_TICKS && !self.paused {
            self.paused = true;
            return true;
        }
        false
    }
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
            beat: Beat {
                paused: true,
                awaiting: None,
                idle: 0,
            },
        })
    }

    /// Une image vient d'être présentée : la suivante attend le prochain battement.
    pub fn presented(&mut self, now: Instant) {
        if self.beat.presented(now) {
            self.link.setPaused(false);
        }
    }

    /// Une image peut partir maintenant.
    pub fn ready(&self, now: Instant) -> bool {
        self.beat.ready(now)
    }

    /// Échéance au-delà de laquelle l'image part sans battement, tant qu'il est attendu.
    pub fn timeout(&self) -> Option<Instant> {
        self.beat.timeout()
    }

    /// Abandonne l'attente d'un battement qui n'est pas venu dans le délai (fenêtre
    /// couverte, écran en veille) ; `true` si elle vient d'être abandonnée.
    pub fn expire(&mut self, now: Instant) -> bool {
        self.beat.expire(now)
    }

    /// Battement de l'écran ; `busy` : la fenêtre a une image à faire ou une animation
    /// imminente. Après quelques battements sans rien, le lien se met en pause.
    pub fn tick(&mut self, busy: bool) {
        if self.beat.tick(busy) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn beat() -> Beat {
        Beat {
            paused: true,
            awaiting: None,
            idle: 0,
        }
    }

    /// M1 de la revue : un battement qui ne vient pas (fenêtre couverte, écran en veille)
    /// laissait une échéance dans le passé, et la boucle ne dormait plus.
    #[test]
    fn an_awaited_beat_that_never_comes_expires_and_leaves_no_deadline() {
        let start = Instant::now();
        let mut beat = beat();
        assert!(beat.presented(start), "le lien reprend à la première image");
        assert!(!beat.ready(start));
        assert_eq!(beat.timeout(), Some(start + TICK_TIMEOUT));
        assert!(!beat.expire(start + TICK_TIMEOUT / 2));
        let late = start + TICK_TIMEOUT * 2;
        assert!(beat.ready(late), "l'image part sans battement");
        assert!(beat.expire(late));
        assert_eq!(
            beat.timeout(),
            None,
            "plus d'échéance passée qui réveille la boucle"
        );
        assert!(!beat.expire(late), "abandonnée une seule fois");
    }

    #[test]
    fn the_link_pauses_after_idle_beats_and_resumes_on_the_next_image() {
        let now = Instant::now();
        let mut beat = beat();
        beat.presented(now);
        for _ in 1..IDLE_TICKS {
            assert!(!beat.tick(false));
        }
        assert!(beat.tick(false), "pause après quatre battements à vide");
        assert!(!beat.tick(false), "une seule demande de pause");
        assert!(beat.presented(now), "reprise à l'image suivante");
        assert!(
            !beat.tick(true),
            "un battement occupé remet le compte à zéro"
        );
    }
}
