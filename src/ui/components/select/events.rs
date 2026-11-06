use super::*;
impl Selects {
    pub fn event(
        &mut self,
        event: &UiEvent,
        focused: Option<ElementId>,
        root: &Element,
        viewport: RectF,
    ) -> SelectResponse {
        // Windows peut renvoyer un mouvement sans déplacement après une peinture.
        // Le clavier doit garder son choix jusqu'à un vrai déplacement du pointeur.
        let pointer_moved = match event {
            UiEvent::PointerMove(p) | UiEvent::PointerDown(p) | UiEvent::PointerUp(p) => {
                let moved = self.pointer.is_none_or(|old| old.x != p.x || old.y != p.y);
                self.pointer = Some(*p);
                moved
            }
            UiEvent::PointerLeave => {
                self.pointer = None;
                false
            }
            _ => false,
        };
        if self.dismiss_capture {
            match event {
                UiEvent::PointerUp(_) | UiEvent::Cancel => {
                    return self.close();
                }
                UiEvent::FocusChanged(false) => {
                    let mut r = self.close();
                    r.handled = false;
                    return r;
                }
                UiEvent::PointerMove(_) | UiEvent::PointerDown(_) => {
                    return SelectResponse {
                        handled: true,
                        ..SelectResponse::default()
                    };
                }
                _ => {}
            }
        }
        if self.open.is_none() {
            let Some(id) = focused else {
                return SelectResponse::default();
            };
            if !control(root, id).is_some_and(|(s, _, _)| s.enabled) {
                return SelectResponse::default();
            }
            match event {
                UiEvent::KeyDown {
                    key: Key::Up | Key::Down | Key::Home | Key::End,
                    ..
                } => {
                    let response = self.expand(id, root, viewport);
                    if let (Some(o), UiEvent::KeyDown { key, .. }) = (&mut self.open, event)
                        && (matches!(key, Key::Home | Key::End)
                            || (o.selected.is_none() && *key == Key::Up))
                    {
                        o.navigate(if *key == Key::Up { Key::End } else { *key });
                    }
                    return response;
                }
                UiEvent::Text(text) if !text.trim().is_empty() => {
                    let response = self.expand(id, root, viewport);
                    if let Some(o) = &mut self.open {
                        o.search(text);
                    }
                    return response;
                }
                _ => return SelectResponse::default(),
            }
        }
        match event {
            UiEvent::FocusChanged(false) => {
                let mut r = self.close();
                r.handled = false;
                return r;
            }
            UiEvent::Cancel
            | UiEvent::KeyDown {
                key: Key::Escape, ..
            } => return self.close(),
            UiEvent::KeyDown { key: Key::Tab, .. } => {
                let mut r = self.close();
                r.handled = false;
                return r;
            }
            UiEvent::Scroll {
                position, delta_y, ..
            } => {
                let o = self.open.as_mut().unwrap();
                if !layout::contains(o.layer.bounds, position.x, position.y) {
                    let mut r = self.close();
                    r.handled = false;
                    return r;
                }
                if delta_y.is_finite() && *delta_y != 0. {
                    let steps = (*delta_y / o.row_height).round();
                    let steps = if steps == 0. { delta_y.signum() } else { steps };
                    o.first = (o.first as f32 + steps).clamp(0., o.max_first() as f32) as usize;
                    o.hover = None;
                    o.pressed = None;
                }
                return SelectResponse {
                    handled: true,
                    repaint: true,
                    ..SelectResponse::default()
                };
            }
            UiEvent::PointerDown(p) => {
                let o = self.open.as_mut().unwrap();
                if !layout::contains(o.layer.bounds, p.x, p.y) {
                    let mut r = self.close();
                    self.dismiss_capture = true;
                    r.capture = true;
                    r.release = false;
                    return r;
                }
                self.captured = true;
                o.pressed = None;
                if let Some((track, thumb)) = o.scroll_bar()
                    && layout::contains(track, p.x, p.y)
                {
                    if layout::contains(thumb, p.x, p.y) {
                        o.drag_scroll = Some(p.y - thumb.top);
                    } else if p.y < thumb.top {
                        o.first = o.first.saturating_sub(o.rows());
                    } else {
                        o.first = (o.first + o.rows()).min(o.max_first());
                    }
                } else if let Some(index) = o.item(*p)
                    && o.options[index].enabled
                {
                    let id = o.options[index].id;
                    o.active = Some(id);
                    o.hover = Some(id);
                    o.pressed = Some(id);
                }
                return SelectResponse {
                    handled: true,
                    repaint: true,
                    capture: true,
                    ..SelectResponse::default()
                };
            }
            UiEvent::PointerMove(p) => {
                let o = self.open.as_mut().unwrap();
                let before = (o.hover, o.active, o.first);
                if let Some(grab) = o.drag_scroll
                    && p.y.is_finite()
                    && let Some((track, thumb)) = o.scroll_bar()
                {
                    let travel = layout::height(track) - layout::height(thumb);
                    if travel > 0. {
                        o.first = (((p.y - track.top - grab) / travel).clamp(0., 1.)
                            * o.max_first() as f32)
                            .round() as usize;
                    }
                } else if pointer_moved {
                    o.hover = o
                        .item(*p)
                        .filter(|&i| o.options[i].enabled)
                        .map(|i| o.options[i].id);
                    if let Some(id) = o.hover {
                        o.active = Some(id);
                    }
                }
                return SelectResponse {
                    handled: true,
                    repaint: before != (o.hover, o.active, o.first),
                    ..SelectResponse::default()
                };
            }
            UiEvent::PointerUp(p) => {
                let o = self.open.as_mut().unwrap();
                let hit = o
                    .item(*p)
                    .filter(|&i| o.options[i].enabled)
                    .map(|i| o.options[i].id);
                if self.captured
                    && o.drag_scroll.is_none()
                    && o.pressed.is_some()
                    && o.pressed == hit
                {
                    let id = o.id;
                    let key = hit.unwrap();
                    return self.choose(id, key, root).unwrap_or_default();
                }
                if self.captured && !layout::contains(o.layer.bounds, p.x, p.y) {
                    return self.close();
                }
                o.drag_scroll = None;
                o.pressed = None;
                let release = self.captured;
                self.captured = false;
                return SelectResponse {
                    handled: true,
                    repaint: true,
                    release,
                    ..SelectResponse::default()
                };
            }
            UiEvent::PointerLeave => {
                self.open.as_mut().unwrap().hover = None;
                return SelectResponse {
                    handled: true,
                    repaint: true,
                    ..SelectResponse::default()
                };
            }
            UiEvent::KeyDown { key, repeat, .. } => {
                let o = self.open.as_mut().unwrap();
                if matches!(key, Key::Enter | Key::Space) && !repeat {
                    if let Some(key) = o.active {
                        let id = o.id;
                        return self.choose(id, key, root).unwrap_or_default();
                    }
                } else if matches!(
                    key,
                    Key::Up | Key::Down | Key::Home | Key::End | Key::PageUp | Key::PageDown
                ) {
                    o.hover = None;
                    o.navigate(*key);
                }
                return SelectResponse {
                    handled: true,
                    repaint: true,
                    ..SelectResponse::default()
                };
            }
            UiEvent::Text(text) => {
                let o = self.open.as_mut().unwrap();
                o.hover = None;
                o.search(text);
                return SelectResponse {
                    handled: true,
                    repaint: true,
                    ..SelectResponse::default()
                };
            }
            UiEvent::KeyUp(_)
            | UiEvent::Edit(_)
            | UiEvent::CompositionStart
            | UiEvent::CompositionUpdate { .. }
            | UiEvent::CompositionCommit(_)
            | UiEvent::CompositionEnd => {
                return SelectResponse {
                    handled: true,
                    ..SelectResponse::default()
                };
            }
            _ => {}
        }
        SelectResponse::default()
    }
}
