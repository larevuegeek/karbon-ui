use super::*;
impl Sliders {
    pub fn event(
        &mut self,
        event: &UiEvent,
        focused: Option<ElementId>,
        root: &Element,
    ) -> SliderResponse {
        let mut response = SliderResponse::default();
        if matches!(
            event,
            UiEvent::Cancel
                | UiEvent::FocusChanged(false)
                | UiEvent::KeyDown {
                    key: Key::Escape | Key::Tab,
                    ..
                }
        ) {
            response.release = self.drag.is_some();
            if let Some(edit) = self.drag.take().or_else(|| self.keyboard.take()) {
                if edit.value != edit.original
                    && slider(root, edit.id).is_some_and(|(s, _)| {
                        s.enabled && !s.read_only && s.config() == edit.config
                    })
                {
                    response.change = Some((edit.id, edit.original));
                }
                response.repaint = true;
                response.handled = matches!(
                    event,
                    UiEvent::Cancel
                        | UiEvent::KeyDown {
                            key: Key::Escape,
                            ..
                        }
                );
            }
            return response;
        }
        if matches!(event, UiEvent::PointerDown(_)) {
            if self.drag.is_some() {
                response.handled = true;
                return response;
            }
            if let Some(edit) = &mut self.keyboard {
                edit.keys = 0;
                response.commit = commit(*edit);
                response.repaint = true;
            }
            return response;
        }
        if let UiEvent::PointerMove(p) | UiEvent::PointerUp(p) = event
            && let Some(mut edit) = self.drag
        {
            if let Some((s, rect)) = slider(root, edit.id)
                && s.coordinate(*p) != edit.pointer
                && let Some(value) = s.pointer_value(rect, *p, edit.offset)
            {
                edit.pointer = s.coordinate(*p);
                if value != edit.value {
                    edit.value = value;
                    response.change = Some((edit.id, value));
                }
            }
            response.handled = true;
            response.repaint = response.change.is_some();
            if matches!(event, UiEvent::PointerUp(_)) {
                self.drag = None;
                response.release = true;
                response.repaint = true;
                response.commit = commit(edit);
            } else {
                self.drag = Some(edit);
            }
            return response;
        }
        if let UiEvent::KeyDown { key, .. } = event
            && key_bit(*key) != 0
            && let Some(id) = focused
            && let Some((s, _)) = slider(root, id)
        {
            response.handled = true;
            if !s.enabled || s.read_only || self.drag.is_some() {
                return response;
            }
            let mut edit = self.keyboard.filter(|e| e.id == id).unwrap_or(Edit {
                id,
                config: s.config(),
                original: s.value,
                value: s.value,
                offset: 0.,
                keys: 0,
                pointer: 0.,
            });
            edit.keys |= key_bit(*key);
            let value = match key {
                Key::Home => s.min,
                Key::End => s.max,
                Key::Left | Key::Down => s.shifted(edit.value, -1),
                Key::Right | Key::Up => s.shifted(edit.value, 1),
                Key::PageDown => s.shifted(edit.value, -10),
                Key::PageUp => s.shifted(edit.value, 10),
                _ => unreachable!(),
            };
            if value != edit.value {
                edit.value = value;
                response.change = Some((id, value));
                response.repaint = true;
            }
            self.keyboard = Some(edit);
            return response;
        }
        if let UiEvent::KeyUp(key) = event
            && let Some(mut edit) = self.keyboard
            && edit.keys & key_bit(*key) != 0
        {
            edit.keys &= !key_bit(*key);
            response.handled = true;
            response.repaint = true;
            if edit.keys == 0 {
                self.keyboard = None;
                response.commit = commit(edit);
            } else {
                self.keyboard = Some(edit);
            }
        }
        response
    }
}
