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
    /// Descendants qui ont leur propre animation de sortie : ils la jouent dans le
    /// fantôme, avec la même horloge (fond qui s'efface et dialogue qui rétrécit).
    parts: Vec<Part>,
    /// Durée (ms) de la plus longue des sorties, celle du fantôme comprise.
    length: f64,
}
/// Descendant d'un fantôme qui joue sa propre animation de sortie.
struct Part {
    id: ElementId,
    exit: Animation,
    origin: (f32, f32),
}
impl Ghost {
    pub(super) fn done(&self, now: Instant) -> bool {
        let elapsed = now
            .saturating_duration_since(self.clock.start)
            .as_secs_f64()
            * 1000.;
        elapsed >= self.length
    }
    /// Effet de sortie du descendant `id` à `now`, s'il en joue une.
    pub(super) fn part(&self, node: &Element, now: Instant) -> Option<(Affine, f32)> {
        let part = self.parts.iter().find(|p| p.id == node.id)?;
        let (effect, _) = animation_sample(part.exit, Some((&self.clock, now)));
        let opacity = effect.opacity.clamp(0., 1.);
        Some((effect.matrix(node.rect, part.origin), opacity))
    }
}
/// Descendants de `node` (lui exclu) qui déclarent une animation de sortie.
fn parts(node: &Element, out: &mut Vec<Part>) {
    for child in &node.children {
        if let Some(exit) = child.motion.as_ref().and_then(|m| m.exit) {
            out.push(Part {
                id: child.id,
                exit,
                origin: child.motion.as_ref().map_or((0.5, 0.5), |m| m.origin),
            });
        }
        parts(child, out);
    }
}
impl Timeline {
    /// Garde un fantôme pour chaque élément à animation de sortie absent de `live`
    /// (le plus haut seulement : ses descendants partent avec lui et jouent, dans le
    /// fantôme, leur propre animation de sortie s'ils en ont une).
    /// Retire de `old` les éléments qui sortent avec une animation (fantômes) ; renvoie
    /// le nombre d'éléments parcourus (0 en mouvements réduits).
    pub fn collect_ghosts(
        &mut self,
        old: &mut Element,
        live: &std::collections::HashSet<ElementId>,
        now: Instant,
    ) -> usize {
        if self.reduced {
            return 0;
        }
        // Un élément revenu dans la vue ne suit plus l'effet d'un fantôme de même identité.
        for ghost in &mut self.ghosts {
            ghost.parts.retain(|p| !live.contains(&p.id));
        }
        // L'ancien arbre n'est pas consommé : seuls les éléments qui sortent en sont
        // retirés, le reste est libéré plus tard par le runtime (hors de l'image).
        fn walk(
            node: &mut Element,
            live: &std::collections::HashSet<ElementId>,
            out: &mut Vec<Element>,
        ) -> usize {
            let mut count = 1;
            let mut i = 0;
            while i < node.children.len() {
                let child = &node.children[i];
                let exits = child.motion.as_ref().is_some_and(|m| m.exit.is_some());
                if exits && !live.contains(&child.id) {
                    out.push(node.children.remove(i));
                } else {
                    count += walk(&mut node.children[i], live, out);
                    i += 1;
                }
            }
            count
        }
        let mut removed = Vec::new();
        let count = walk(old, live, &mut removed);
        let now = self.now.map_or(now, |old| old.max(now));
        for element in removed {
            let motion = element.motion.as_deref().copied().unwrap_or_default();
            if layout::width(element.clip) <= 0. || layout::height(element.clip) <= 0. {
                continue;
            }
            let exit = motion.exit.unwrap();
            let mut inner = Vec::new();
            parts(&element, &mut inner);
            let length = inner
                .iter()
                .map(|p| p.exit.exit_length())
                .fold(exit.exit_length(), f64::max);
            self.ghosts.push(Ghost {
                exit,
                origin: motion.origin,
                clock: Clock::new(now, motion.exit),
                element,
                parts: inner,
                length,
            });
        }
        if !self.ghosts.is_empty() {
            self.next = earliest(self.next, Some(now));
        }
        count
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
