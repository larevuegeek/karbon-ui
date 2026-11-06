//! Fantômes : copies d'éléments retirés de la vue qui jouent leur animation de sortie.
use super::{
    Timeline, earliest,
    track::{Clock, animation_sample},
};
use crate::ui::{
    animation::{Animation, Effect},
    element::{Element, ElementId},
    layout,
    rendering::transform::Affine,
};
use std::time::Instant;
/// Copie non interactive d'un élément retiré de la vue, qui joue son animation de
/// sortie (`data-exit`) puis disparaît.
pub(crate) struct Ghost {
    pub element: Element,
    exit: Animation,
    origin: (f32, f32),
    pub(super) clock: Clock,
}
impl Ghost {
    pub(super) fn done(&self, now: Instant) -> bool {
        let elapsed = now
            .saturating_duration_since(self.clock.start)
            .as_secs_f64()
            * 1000.;
        elapsed >= self.exit.exit_length()
    }
}
impl Timeline {
    /// Garde un fantôme pour chaque élément à animation de sortie absent de `live`
    /// (le plus haut seulement : ses descendants partent avec lui).
    pub fn collect_ghosts(
        &mut self,
        old: Element,
        live: &std::collections::HashSet<ElementId>,
        now: Instant,
    ) {
        if self.reduced {
            return;
        }
        fn walk(
            node: Element,
            live: &std::collections::HashSet<ElementId>,
            out: &mut Vec<Element>,
        ) {
            for child in node.children {
                let exits = child.motion.as_ref().is_some_and(|m| m.exit.is_some());
                if exits && !live.contains(&child.id) {
                    out.push(child);
                } else {
                    walk(child, live, out);
                }
            }
        }
        let mut removed = Vec::new();
        walk(old, live, &mut removed);
        let now = self.now.map_or(now, |old| old.max(now));
        for element in removed {
            let motion = element.motion.as_deref().copied().unwrap_or_default();
            if layout::width(element.clip) <= 0. || layout::height(element.clip) <= 0. {
                continue;
            }
            self.ghosts.push(Ghost {
                exit: motion.exit.unwrap(),
                origin: motion.origin,
                clock: Clock::new(now, motion.exit),
                element,
            });
        }
        if !self.ghosts.is_empty() {
            self.next = earliest(self.next, Some(now));
        }
    }
    pub fn has_ghosts(&self) -> bool {
        !self.ghosts.is_empty()
    }
    /// Fantômes avec leur transformation et leur opacité à l'instant courant.
    pub fn ghosts(&self) -> impl Iterator<Item = (&Element, Affine, f32)> {
        let now = self.time();
        self.ghosts.iter().map(move |g| {
            let (effect, _) = animation_sample(g.exit, Some((&g.clock, now)));
            let effect = Effect {
                opacity: effect.opacity.clamp(0., 1.),
                ..effect
            };
            (
                &g.element,
                effect.matrix(g.element.rect, g.origin),
                effect.opacity,
            )
        })
    }
}
