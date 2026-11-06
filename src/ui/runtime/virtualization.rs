use super::*;
impl UiRuntime {
    pub(super) fn select_virtual(&mut self, id: ElementId, index: usize) {
        if let Some(node) = self.root.as_mut().and_then(|r| r.find_mut(id))
            && let Some(list) = &mut node.virtual_list
        {
            list.active = list.last_index().map(|last| index.min(last));
            if let Some(active) = list.active {
                self.components.lists.positions.insert(id, active);
            }
            self.interaction.focused = Some(id);
            self.invalidation.repaint = true;
        }
    }

    pub(super) fn reveal_virtual(&mut self, id: ElementId) {
        let Some(node) = self.root.as_ref().and_then(|r| r.find(id)) else {
            return;
        };
        if let (Some(list), Some(s)) = (&node.virtual_list, node.scroll)
            && let Some(index) = list.active
        {
            let start = index as f32 * list.stride(node.style.vertical_gap());
            let end = start + list.row_height();
            let offset = if start < s.offset {
                start
            } else if end > s.offset + s.viewport_size() {
                (end - s.viewport_size()).min(start)
            } else {
                s.offset
            };
            self.scroll_to(id, offset);
        }
    }

    pub(super) fn virtual_key(&mut self, key: Key) -> bool {
        let Some(node) = self
            .interaction
            .focused
            .and_then(|id| self.root.as_ref()?.find(id))
        else {
            return false;
        };
        let Some(list) = &node.virtual_list else {
            return false;
        };
        if !matches!(
            key,
            Key::Up | Key::Down | Key::Home | Key::End | Key::PageUp | Key::PageDown
        ) {
            return false;
        }
        if let Some(last) = list.last_index() {
            let current = list.active.unwrap_or(0);
            let step = node.scroll.map_or(1, |s| {
                (s.viewport_size() / list.stride(node.style.vertical_gap()))
                    .floor()
                    .max(1.) as usize
            });
            let next = match key {
                Key::Up => current.saturating_sub(1),
                Key::Down => current.saturating_add(1).min(last),
                Key::Home => 0,
                Key::End => last,
                Key::PageUp => current.saturating_sub(step),
                Key::PageDown => current.saturating_add(step).min(last),
                _ => current,
            };
            let id = node.id;
            self.interaction.keyboard_pressed = None;
            self.select_virtual(id, next);
            self.interaction.focus_visible = true;
            self.reveal_virtual(id);
            self.reveal_focus();
        }
        true
    }
}
