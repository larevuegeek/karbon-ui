use super::*;
impl UiRuntime {
    pub(super) fn scroll_to(&mut self, id: ElementId, offset: f32) -> f32 {
        let Some(node) = self.root.as_mut().and_then(|r| r.find_mut(id)) else {
            return 0.;
        };
        let delta = scroll::set_offset(node, offset);
        if delta != 0. {
            self.components
                .scrolls
                .offsets
                .insert(id, node.scroll_offset);
            self.interaction.hovered = self.interaction.pointer.and_then(|p| self.hit(p));
            self.invalidation.repaint = true;
            self.invalidation.stats.scrolls += 1;
        }
        delta
    }

    pub(super) fn scroll_ancestors(&self, target: ElementId) -> Vec<ElementId> {
        fn walk(node: &Element, target: ElementId, ids: &mut Vec<ElementId>) -> Option<bool> {
            let stop = if node.id == target {
                false
            } else {
                node.children.iter().find_map(|c| walk(c, target, ids))?
            };
            if !stop && node.scroll.is_some() {
                ids.push(node.id);
            }
            Some(stop || layout::fixed(node))
        }
        let mut ids = Vec::new();
        if let Some(root) = &self.root {
            walk(root, target, &mut ids);
        }
        ids
    }

    pub(super) fn reveal_focus(&mut self) {
        let Some(id) = self.interaction.focused else {
            return;
        };
        self.reveal_element(id);
    }

    pub(super) fn reveal_element(&mut self, id: ElementId) {
        for ancestor in self.scroll_ancestors(id) {
            if ancestor == id {
                continue;
            }
            let root = self.root.as_ref().unwrap();
            let target = root.find(id).unwrap();
            let container = root.find(ancestor).unwrap();
            let s = container.scroll.unwrap();
            // `scroll-margin` de l'élément et `scroll-padding` du conteneur.
            let margin = target
                .rules
                .as_ref()
                .map_or(Default::default(), |r| r.scroll_margin);
            let padding = container
                .rules
                .as_ref()
                .map_or(Default::default(), |r| r.scroll_padding);
            let (before, after) = match s.axis {
                Axis::Vertical => (margin.top + padding.top, margin.bottom + padding.bottom),
                Axis::Horizontal => (margin.left + padding.left, margin.right + padding.right),
            };
            let bounds = target.rect;
            let start = scroll::axis_start(bounds, s.axis) - before;
            let view_start = scroll::axis_start(s.viewport, s.axis);
            let end =
                scroll::axis_start(bounds, s.axis) + scroll::axis_size(bounds, s.axis) + after;
            let view_end = view_start + s.viewport_size();
            let delta = if start < view_start {
                start - view_start
            } else if end > view_end {
                (end - view_end).min(start - view_start)
            } else {
                0.
            };
            self.scroll_to(ancestor, s.offset + delta);
        }
    }

    pub(super) fn scroll_key(&mut self, key: Key) -> bool {
        let Some(id) = self.interaction.focused else {
            return false;
        };
        for ancestor in self.scroll_ancestors(id) {
            let s = self
                .root
                .as_ref()
                .unwrap()
                .find(ancestor)
                .unwrap()
                .scroll
                .unwrap();
            let delta = match key {
                Key::Home => -s.offset,
                Key::End => s.max_offset() - s.offset,
                Key::PageUp => -s.viewport_size() * 0.9,
                Key::PageDown => s.viewport_size() * 0.9,
                Key::Up if s.axis == Axis::Vertical => -40.,
                Key::Down if s.axis == Axis::Vertical => 40.,
                Key::Left if s.axis == Axis::Horizontal => -40.,
                Key::Right if s.axis == Axis::Horizontal => 40.,
                _ => continue,
            };
            if self.scroll_to(ancestor, s.offset + delta) != 0. {
                return true;
            }
            // Le viewport lui-même consomme la touche même à une extrémité.
            if ancestor == id {
                return true;
            }
        }
        false
    }
}
