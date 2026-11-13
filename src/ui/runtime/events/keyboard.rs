use super::super::*;
impl UiRuntime {
    pub(super) fn key_down_event(&mut self, key: Key, repeat: bool, result: &mut EventResult) {
        if self.radio_key(key) {
            result.handled = true;
        } else if key == Key::Escape && self.gesture_active() {
            *result = self.event(UiEvent::Cancel);
            result.handled = true;
        } else if let Some(id) = self.interaction.focused
            && let Some(node) = self.root.as_ref().and_then(|r| r.find(id))
        {
            if let Layout::Split(split) = node.layout {
                if layout::resizable_divider(node) {
                    let available = (if split.axis == Axis::Horizontal {
                        layout::width(node.rect)
                    } else {
                        layout::height(node.rect)
                    } - layout::DIVIDER_SIZE)
                        .max(1.);
                    let fraction = match key {
                        Key::Home => Some(0.),
                        Key::End => Some(1.),
                        Key::Left if split.axis == Axis::Horizontal => {
                            Some(split.fraction - 16. / available)
                        }
                        Key::Right if split.axis == Axis::Horizontal => {
                            Some(split.fraction + 16. / available)
                        }
                        Key::Up if split.axis == Axis::Vertical => {
                            Some(split.fraction - 16. / available)
                        }
                        Key::Down if split.axis == Axis::Vertical => {
                            Some(split.fraction + 16. / available)
                        }
                        _ => None,
                    };
                    if let Some(fraction) = fraction {
                        self.resize_split(id, fraction);
                        self.interaction.focus_visible = true;
                        result.handled = true;
                    }
                }
            } else if node.virtual_list.is_some()
                || node
                    .widget
                    .as_ref()
                    .is_some_and(|w| w.action().is_some() || w.select().is_some())
            {
                if key == Key::Enter {
                    if !repeat {
                        self.reveal_virtual(id);
                        self.activate(id);
                    }
                    result.handled = true;
                    self.invalidation.repaint = true;
                }
                if key == Key::Space {
                    if !repeat {
                        self.reveal_virtual(id);
                        self.interaction.keyboard_pressed = Some(id);
                        self.invalidation.repaint = true;
                    }
                    result.handled = true;
                }
            }
        }
        if !result.handled {
            result.handled = self.virtual_key(key);
        }
        if !result.handled {
            result.handled = self.scroll_key(key);
        }
    }

    pub(super) fn space_up_event(&mut self, result: &mut EventResult) {
        if let Some(id) = self.interaction.keyboard_pressed.take() {
            if self.interaction.focused == Some(id) {
                self.activate(id);
            }
            result.handled = true;
            self.invalidation.repaint = true;
        }
    }
}
