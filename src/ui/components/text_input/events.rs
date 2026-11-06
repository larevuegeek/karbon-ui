use super::*;
impl TextInputs {
    pub fn event(
        &mut self,
        event: &crate::ui::runtime::UiEvent,
        focused: Option<ElementId>,
        root: &Element,
    ) -> InputResponse {
        use crate::ui::runtime::{Key, UiEvent};
        let mut result = InputResponse::default();
        if let UiEvent::FocusChanged(active) = event {
            self.active = *active;
            self.caret_visible = true;
            if !active {
                for e in self.states.values_mut() {
                    e.composition = None;
                }
                self.dragging = None;
            }
            result.repaint = true;
            return result;
        }
        let Some(id) = focused else {
            return result;
        };
        let Some(input) = root
            .find(id)
            .and_then(|n| n.widget.as_ref())
            .and_then(|w| w.text_input())
            .filter(|i| i.enabled)
        else {
            return result;
        };
        let Some(e) = self.states.get_mut(&id) else {
            return result;
        };
        let mut changed = false;
        match event {
            UiEvent::CaretTick => {
                self.caret_visible = !self.caret_visible;
                result.handled = true;
            }
            UiEvent::Text(value) => {
                if !input.read_only {
                    changed = e.replace(value, input.max_chars);
                }
                result.handled = true;
            }
            UiEvent::CompositionStart => {
                if !input.read_only {
                    e.composition = Some(Composition {
                        range: e.range(),
                        value: "".into(),
                        caret: 0,
                    });
                }
                result.handled = true;
            }
            UiEvent::CompositionUpdate {
                value,
                cursor_utf16,
            } => {
                if !input.read_only {
                    let range = e
                        .composition
                        .as_ref()
                        .map_or_else(|| e.range(), |c| c.range.clone());
                    let remaining = input.max_chars.saturating_sub(
                        e.text[..range.start].chars().count() + e.text[range.end..].chars().count(),
                    );
                    let value: Arc<str> = editor::sanitize(value, remaining, e.multiline).into();
                    let mut units = 0;
                    let mut caret = 0;
                    for (byte, c) in value.char_indices() {
                        if units >= *cursor_utf16 {
                            break;
                        }
                        units += c.len_utf16();
                        caret = byte + c.len_utf8();
                    }
                    e.composition = Some(Composition {
                        range,
                        value,
                        caret,
                    });
                }
                result.handled = true;
            }
            UiEvent::CompositionCommit(value) => {
                if !input.read_only {
                    if let Some(c) = e.composition.take() {
                        e.anchor = c.range.start;
                        e.cursor = c.range.end;
                    }
                    changed = e.replace(value, input.max_chars);
                }
                result.handled = true;
            }
            UiEvent::CompositionEnd => {
                e.composition = None;
                result.handled = true;
            }
            UiEvent::Edit(command) => {
                result.handled = true;
                if e.composition.is_some() {
                    e.composition = None;
                }
                match command {
                    EditCommand::SelectAll => {
                        e.anchor = 0;
                        e.cursor = e.text.len();
                    }
                    EditCommand::Copy | EditCommand::Cut => {
                        if !e.range().is_empty() && !input.masked {
                            result.clipboard = Some(ClipboardRequest::Write {
                                text: e.text[e.range()].to_string(),
                                cut: matches!(command, EditCommand::Cut) && !input.read_only,
                            });
                        }
                    }
                    EditCommand::Paste => {
                        if !input.read_only {
                            result.clipboard = Some(ClipboardRequest::Read);
                        }
                    }
                    EditCommand::DeleteSelection => {
                        if !input.read_only {
                            changed = e.replace("", input.max_chars);
                        }
                    }
                    EditCommand::Undo | EditCommand::Redo => {
                        if !input.read_only {
                            let current = e.snapshot();
                            let previous = if matches!(command, EditCommand::Undo) {
                                let p = e.undo.pop();
                                if p.is_some() {
                                    e.redo.push(current);
                                    trim_history(&mut e.redo);
                                }
                                p
                            } else {
                                let p = e.redo.pop();
                                if p.is_some() {
                                    e.undo.push(current);
                                    trim_history(&mut e.undo);
                                }
                                p
                            };
                            if let Some(p) = previous {
                                e.text = p.text;
                                e.anchor = p.anchor;
                                e.cursor = p.cursor;
                                changed = true;
                            }
                        }
                    }
                    EditCommand::WordLeft { extend } | EditCommand::WordRight { extend } => {
                        let right = matches!(command, EditCommand::WordRight { .. });
                        let line = e.line.as_ref().unwrap();
                        let mut byte = e.cursor;
                        if right {
                            while byte < e.text.len()
                                && !e.text[byte..].chars().next().unwrap().is_whitespace()
                            {
                                byte = line.next(byte);
                            }
                            while byte < e.text.len()
                                && e.text[byte..].chars().next().unwrap().is_whitespace()
                            {
                                byte = line.next(byte);
                            }
                        } else {
                            while byte > 0
                                && e.text[..byte].chars().next_back().unwrap().is_whitespace()
                            {
                                byte = line.previous(byte);
                            }
                            while byte > 0
                                && !e.text[..byte].chars().next_back().unwrap().is_whitespace()
                            {
                                byte = line.previous(byte);
                            }
                        }
                        e.move_to(byte, *extend);
                    }
                }
            }
            UiEvent::KeyDown { key, shift, repeat } => {
                if e.composition.is_some() && *key != Key::Escape {
                    return result;
                }
                if e.composition.is_none()
                    && let Some(handler) = &input.on_key
                {
                    result.key = Some(handler.emit(*key));
                    // Les flèches verticales n'éditent rien sur une ligne : elles
                    // appartiennent à l'application (liste de suggestions).
                    if !input.multiline
                        && matches!(key, Key::Up | Key::Down | Key::PageUp | Key::PageDown)
                    {
                        result.handled = true;
                        return result;
                    }
                }
                if input.multiline
                    && e.composition.is_none()
                    && let Some(edited) = e.multiline_key(*key, *shift, input)
                {
                    changed = edited;
                    result.handled = true;
                } else {
                    let line = e.line.as_ref().unwrap();
                    match key {
                        Key::Left => {
                            let target = if !shift && !e.range().is_empty() {
                                e.range().start
                            } else {
                                line.previous(e.cursor)
                            };
                            e.move_to(target, *shift);
                        }
                        Key::Right => {
                            let target = if !shift && !e.range().is_empty() {
                                e.range().end
                            } else {
                                line.next(e.cursor)
                            };
                            e.move_to(target, *shift);
                        }
                        Key::Home => e.move_to(0, *shift),
                        Key::End => e.move_to(e.text.len(), *shift),
                        Key::Backspace | Key::Delete => {
                            if !input.read_only {
                                if e.range().is_empty() {
                                    if *key == Key::Backspace {
                                        e.anchor = line.previous(e.cursor);
                                    } else {
                                        e.anchor = line.next(e.cursor);
                                    }
                                }
                                changed = e.replace("", input.max_chars);
                            }
                        }
                        Key::Enter => {
                            if !repeat {
                                result.submit = input.on_submit.as_ref().map(Action::emit);
                            }
                        }
                        Key::Escape => {
                            e.composition = None;
                            e.anchor = e.cursor;
                        }
                        // WM_CHAR fournit l'espace et le texte, pas WM_KEYDOWN.
                        Key::Space => {}
                        _ => return result,
                    }
                    result.handled = true;
                }
            }
            _ => return result,
        }
        self.caret_visible = if matches!(event, UiEvent::CaretTick) {
            self.caret_visible
        } else {
            true
        };
        result.repaint = result.handled;
        if result.handled && !matches!(event, UiEvent::CaretTick) {
            e.reveal = None;
            e.follow = true;
            self.revision = self.revision.wrapping_add(1);
        }
        if changed {
            result.change = Some(input.on_change.emit(e.text.clone()));
        }
        result
    }
}
