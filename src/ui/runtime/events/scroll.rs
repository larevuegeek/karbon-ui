use super::super::*;
impl UiRuntime {
    pub(super) fn wheel_event(
        &mut self,
        position: PointF,
        mut delta_x: f32,
        mut delta_y: f32,
        result: &mut EventResult,
    ) -> bool {
        self.interaction.pointer = Some(position);
        if self.has_capture() || !delta_x.is_finite() || !delta_y.is_finite() {
            return false;
        }
        let hit = self.hit(position);
        // Champ multiligne sous le pointeur : son texte défile d'abord.
        if let Some(Hit::Input(id)) = hit {
            let consumed = self.components.inputs.scroll_by(id, delta_y);
            if consumed != 0. {
                delta_y -= consumed;
                result.handled = true;
                self.invalidation.repaint = true;
                self.invalidation.semantics();
            }
        }
        let ids = hit
            .map(|hit| self.scroll_ancestors(hit.id()))
            .unwrap_or_default();
        for id in ids {
            let node = self.root.as_ref().unwrap().find(id).unwrap();
            let s = node.scroll.unwrap();
            // `overscroll-behavior: contain` : le reste du défilement ne passe pas aux
            // conteneurs englobants.
            let contain = node.rules.as_ref().is_some_and(|r| r.overscroll_contain);
            let delta = if s.axis == Axis::Horizontal {
                &mut delta_x
            } else {
                &mut delta_y
            };
            let consumed = self.scroll_to(id, s.offset + *delta);
            *delta -= consumed;
            result.handled |= consumed != 0.;
            if contain {
                *delta = 0.;
                result.handled = true;
            }
        }
        self.interaction.hovered = self.hit(position);

        true
    }
}
