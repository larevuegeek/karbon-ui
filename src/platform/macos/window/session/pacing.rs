//! Cadence des images d'une fenêtre (contrat dans `platform/backend.rs`, « Cadence d'une
//! image ») : battement de l'écran (`frame_link.rs`), image présentée puis travail fait
//! pendant l'attente de l'écran, fin de l'attente, et réveils dus du runtime.
use super::lifecycle::*;

/// Après un battement de l'écran, un réveil dû dans la période qui vient part de lui : les
/// animations demandent « dernier échantillon + 16,67 ms » (comme sous Wayland).
const FRAME_CLOCK_HORIZON: std::time::Duration = std::time::Duration::from_nanos(16_666_667);
/// Réveil du runtime assez proche pour garder l'horloge de l'écran en marche.
const MOTION_HORIZON: std::time::Duration = std::time::Duration::from_millis(40);
impl Session {
    /// Une image peut partir maintenant (battement de l'écran reçu depuis la précédente).
    pub fn frame_ready(&self, now: Instant) -> bool {
        self.link.as_ref().is_none_or(|link| link.ready(now))
    }
    /// Battement de l'écran : l'attente est finie, et les animations de l'image suivante
    /// sont échantillonnées tout de suite au lieu d'attendre leur échéance.
    pub fn frame_tick(&mut self, now: Instant) {
        let busy = self.dirty
            || self
                .runtime
                .next_wake()
                .is_some_and(|wake| wake <= now + MOTION_HORIZON);
        if let Some(link) = &mut self.link {
            link.tick(busy);
        }
        self.frame_wait_over(now);
        self.frame_clock = true;
    }
    /// Une image vient d'être présentée : avant l'attente de l'écran, une tranche du
    /// travail étalé et la reconstruction demandée (voir `platform/backend.rs`).
    pub fn frame_presented(&mut self) -> crate::Result<()> {
        let now = Instant::now();
        self.runtime.frame_presented(now);
        if let Some(link) = &mut self.link {
            link.presented(now);
        }
        if crate::platform::shared::pacing::enabled(self.link.is_some()) {
            let before = self.runtime.stats();
            self.runtime.begin_frame_work(now);
            let work = now.elapsed();
            if self.runtime.rebuild_pending() {
                self.prepare()?;
                self.dirty = true;
            }
            let after = self.runtime.stats();
            if self.options.profile && after.views != before.views {
                let ms = |d: std::time::Duration| d.as_secs_f64() * 1e3;
                eprintln!(
                    "karbon-ui: reconstruction pendant l'attente de l'écran : {:.1} ms (travail étalé {:.1}, vue {:.1}, styles {:.1}, layout {:.1})",
                    ms(now.elapsed()),
                    ms(work),
                    ms(after.view_time - before.view_time),
                    ms(after.style_time - before.style_time),
                    ms(after.layout_time - before.layout_time),
                );
            }
        }
        Ok(())
    }
    /// L'attente de l'écran est finie : ce qui est demandé maintenant attend l'image suivante.
    pub fn frame_wait_over(&mut self, now: Instant) {
        if crate::platform::shared::pacing::enabled(self.link.is_some()) {
            self.runtime.hold_rebuild(now);
        }
    }
    /// Échéances atteintes : réveil du runtime (animations, infobulles) et redessin
    /// exact après une animation.
    pub fn tick(&mut self, now: Instant) -> crate::Result<()> {
        // Battement attendu jamais venu : sans cet abandon, son échéance resterait dans le
        // passé et `next_wake` empêcherait la boucle de dormir tant que rien n'est à peindre.
        if self.link.as_mut().is_some_and(|link| link.expire(now)) {
            self.frame_wait_over(now);
        }
        if self.refine_at.is_some_and(|at| at <= now) {
            self.refine_at = None;
            // Mouvement terminé : la surface en demi-résolution n'est plus utile.
            self.motion = None;
            self.presented = None;
            self.dirty = true;
        }
        if self.trim_at.is_some_and(|at| at <= now) {
            self.trim_at = None;
            if let Some((gpu, _)) = &mut self.gpu {
                gpu.trim(metal::Trim::Idle);
            }
        }
        // Juste après un battement, un réveil dû dans la période qui vient part de lui.
        let horizon = if std::mem::take(&mut self.frame_clock) {
            now + FRAME_CLOCK_HORIZON
        } else {
            now
        };
        if self.runtime.next_wake().is_some_and(|wake| wake <= horizon) {
            if self.link.is_none() {
                self.frame_wait_over(now);
            }
            self.input(UiEvent::Timer(now))?;
        }
        Ok(())
    }
}
