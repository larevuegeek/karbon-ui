pub(super) use super::super::*;
impl UiRuntime {
    /// Dispatch an input event to the UI and report what the backend must do next.
    ///
    /// Messages emitted by the affected elements are delivered to the application, whose
    /// `update` may invalidate the view.
    ///
    /// It does not lay out the tree: see the type documentation for events dispatched
    /// without a [`prepare`](Self::prepare) in between.
    pub fn event(&mut self, event: UiEvent) -> EventResult {
        let ends = matches!(
            event,
            UiEvent::PointerUp(_) | UiEvent::Cancel | UiEvent::FocusChanged(false)
        );
        // Le `Cancel` envoyé par la reconstruction qui ferme le popover ne supprime pas
        // l'appui à rejouer.
        if matches!(event, UiEvent::PointerUp(_) | UiEvent::FocusChanged(false)) {
            self.interaction.replay = None;
        }
        let down = matches!(
            event,
            UiEvent::PointerDown(_) | UiEvent::PointerDownExtended(_)
        );
        let mut result = self.dispatch(event);
        result.repaint |= self.notify_listeners(down);
        if ends && std::mem::take(&mut self.interaction.replay_release) {
            result.release = true;
        }
        result
    }
    pub(super) fn dispatch(&mut self, event: UiEvent) -> EventResult {
        if let Some(reply) = self.drag_drop_event(&event) {
            return reply;
        }
        if let UiEvent::Timer(now) = event {
            return self.timer_event(now);
        }
        match event {
            UiEvent::KeyDown {
                key: Key::Escape, ..
            } => {}
            UiEvent::KeyDown { .. } | UiEvent::Text(_) | UiEvent::Edit(_) => {
                self.tooltip_input_mode(true)
            }
            UiEvent::PointerMove(p)
                if self
                    .interaction
                    .pointer
                    .is_none_or(|old| old.x != p.x || old.y != p.y) =>
            {
                self.tooltip_input_mode(false)
            }
            UiEvent::PointerDown(_) | UiEvent::PointerDownExtended(_) => {
                self.interaction.tooltip_keyboard = false
            }
            _ => {}
        }
        let escape_tip = matches!(
            event,
            UiEvent::KeyDown {
                key: Key::Escape,
                ..
            }
        ) && self.tooltip().is_some();
        if matches!(
            event,
            UiEvent::PointerDown(_)
                | UiEvent::PointerDownExtended(_)
                | UiEvent::ContextMenu(_)
                | UiEvent::KeyDown { .. }
                | UiEvent::Text(_)
                | UiEvent::Edit(_)
                | UiEvent::Scroll { .. }
                | UiEvent::Wheel { .. }
                | UiEvent::CompositionStart
                | UiEvent::Cancel
        ) && self.components.tooltips.dismiss()
        {
            self.invalidation.repaint_semantics();
        }
        if let UiEvent::FocusChanged(active) = event {
            self.interaction.window_active = active;
        }
        if escape_tip {
            self.interaction.escape_consumed = true;
            self.sync_tooltip();
            return EventResult {
                handled: true,
                repaint: true,
                ..EventResult::default()
            };
        }
        let previous_motion_hover = self.motion_hover();
        let mut result = self.event_inner(event);
        self.invalidation.repaint |= previous_motion_hover != self.motion_hover();
        self.sync_tooltip();
        result.repaint |= self.invalidation.repaint;
        result
    }

    pub(crate) fn event_inner(&mut self, event: UiEvent) -> EventResult {
        let extend = matches!(event, UiEvent::PointerDownExtended(_));
        let event = if let UiEvent::PointerDownExtended(p) = event {
            UiEvent::PointerDown(p)
        } else {
            event
        };
        let mut result = EventResult {
            release: std::mem::take(&mut self.interaction.release_pending),
            ..EventResult::default()
        };
        if let UiEvent::ContextMenu(position) = event {
            let mut reply = self.context_menu_event(position);
            reply.release |= result.release;
            return reply;
        }
        let escape = matches!(
            event,
            UiEvent::KeyDown {
                key: Key::Escape,
                ..
            }
        );
        if self.interaction.swallow_escape(&event) {
            result.handled = true;
            return result;
        }
        if let Some(owner) = self.interaction.backdrop_capture {
            match event {
                UiEvent::PointerUp(p) => {
                    self.interaction.backdrop_capture = None;
                    let dismiss = self
                        .root
                        .as_ref()
                        .map(layers::active)
                        .filter(|n| n.id == owner)
                        .filter(|n| !layout::contains(n.clip, p.x, p.y))
                        .and_then(modal::config)
                        .filter(|m| m.close_on_backdrop)
                        .and_then(|m| m.on_dismiss.as_ref())
                        .or_else(|| {
                            self.root
                                .as_ref()
                                .and_then(popover::top)
                                .filter(|n| n.id == owner && !layout::contains(n.clip, p.x, p.y))
                                .and_then(popover::config)
                                .map(|p| &p.on_dismiss)
                        })
                        .map(Action::emit);
                    if let Some(message) = dismiss {
                        self.send(message);
                    }
                    result.handled = true;
                    result.release = true;
                    result.repaint = self.invalidation.repaint || self.invalidation.rebuild;
                    return result;
                }
                UiEvent::Cancel | UiEvent::FocusChanged(false) => {
                    self.interaction.backdrop_capture = None;
                    result.release = true;
                }
                UiEvent::PointerMove(_) | UiEvent::PointerDown(_) => {
                    result.handled = true;
                    return result;
                }
                _ => {}
            }
        }
        if self.components.edit_menu.is_open() || self.components.edit_menu.captured() {
            let response = self.components.edit_menu.event(&event);
            let reply = self.menu_response(response);
            result.release |= reply.release;
            if reply.handled {
                self.interaction.escape_consumed |= escape;
                if let UiEvent::PointerMove(p) | UiEvent::PointerUp(p) | UiEvent::PointerDown(p) =
                    event
                {
                    self.interaction.pointer = Some(p);
                }
                self.interaction.hovered = None;
                result.handled = true;
                result.capture = reply.capture;
                result.repaint = reply.repaint;
                result.clipboard = reply.clipboard;
                return result;
            }
        }
        if let Some(root) = &self.root {
            let response = self.components.selects.event(
                &event,
                self.interaction.focused,
                root,
                self.layer_viewport(),
            );
            let reply = self.select_response(response);
            result.release |= reply.release;
            if reply.handled {
                self.interaction.escape_consumed |= escape;
                if let UiEvent::PointerMove(p) | UiEvent::PointerUp(p) | UiEvent::PointerDown(p) =
                    event
                {
                    self.interaction.pointer = Some(p);
                }
                self.interaction.hovered = None;
                result.handled = true;
                result.capture = reply.capture;
                result.repaint = reply.repaint;
                return result;
            }
        }
        if let Some(root) = &self.root {
            let response = self
                .components
                .sliders
                .event(&event, self.interaction.focused, root);
            let reply = self.slider_response(response);
            result.release |= reply.release;
            if reply.handled {
                self.interaction.escape_consumed |= escape;
                if let UiEvent::PointerMove(p) | UiEvent::PointerUp(p) = event {
                    self.interaction.pointer = Some(p);
                    self.interaction.hovered = self.hit(p);
                }
                if matches!(event, UiEvent::KeyDown { .. }) {
                    self.interaction.focus_visible = true;
                    self.reveal_focus();
                    self.invalidation.repaint = true;
                }
                result.handled = true;
                result.capture = reply.capture;
                result.repaint = self.invalidation.pending();
                return result;
            }
        }
        if let Some(popup) = self.root.as_ref().and_then(popover::top) {
            if let UiEvent::FocusChanged(false) = event {
                self.dismiss_popover();
            } else if let UiEvent::Scroll { position, .. } | UiEvent::Wheel { position, .. } = event
                && !layout::contains(popup.clip, position.x, position.y)
            {
                self.dismiss_popover();
                result.handled = true;
                result.repaint = self.invalidation.rebuild;
                return result;
            } else if let UiEvent::KeyDown {
                key: Key::Tab,
                shift,
                repeat,
            } = event
                && !self.focused_input().is_some_and(|i| i.composing)
            {
                if !repeat && self.dismiss_popover() {
                    self.interaction.pending_tab = Some(shift);
                }
                result.handled = true;
                result.repaint = self.invalidation.rebuild;
                return result;
            } else if let Some(item_role) = popover::config(popup)
                .filter(|p| p.kind != PopoverKind::Combobox)
                .and_then(|p| p.kind.item_role())
                && let UiEvent::KeyDown {
                    key: Key::Up | Key::Down | Key::Home | Key::End,
                    ..
                } = event
            {
                let UiEvent::KeyDown { key, .. } = event else {
                    unreachable!()
                };
                let items: Vec<_> = self
                    .interaction
                    .focus_order
                    .iter()
                    .copied()
                    .filter(|id| {
                        popup
                            .find(*id)
                            .and_then(|n| n.widget.as_ref())
                            .is_some_and(|w| w.semantics().role == item_role)
                    })
                    .collect();
                if !items.is_empty() {
                    let current = self
                        .interaction
                        .focused
                        .and_then(|id| items.iter().position(|x| *x == id))
                        .unwrap_or(0);
                    let index = match key {
                        Key::Home => 0,
                        Key::End => items.len() - 1,
                        Key::Up => (current + items.len() - 1) % items.len(),
                        _ => (current + 1) % items.len(),
                    };
                    self.interaction.focus_keyboard(items[index]);
                    self.reveal_focus();
                    self.invalidation.repaint = true;
                }
                result.handled = true;
                result.repaint = self.invalidation.repaint;
                return result;
            }
        }
        // Liste de suggestions (combobox) : non modale. Un clic hors d'elle et de son champ
        // la ferme et agit quand même sur la page ; Échap la ferme.
        if let Some(root) = &self.root
            && let Some(list) = popover::top(root).filter(|n| popover::is_combobox(n))
        {
            let outside = match event {
                UiEvent::PointerDown(p) => {
                    !self.has_capture()
                        && !layout::contains(list.clip, p.x, p.y)
                        && !popover::anchor(root, list)
                            .is_some_and(|a| layout::contains(a, p.x, p.y))
                }
                _ => false,
            };
            let escape_list = escape && !self.focused_input().is_some_and(|i| i.composing);
            let dismiss = (outside || escape_list)
                .then(|| popover::config(list).map(|p| p.on_dismiss.emit()))
                .flatten();
            if let Some(message) = dismiss {
                self.send(message);
            }
            if escape_list {
                result.handled = true;
                self.interaction.escape_consumed = true;
                result.repaint = self.invalidation.repaint || self.invalidation.rebuild;
                return result;
            }
        }
        if let Some(dialog) = self.root.as_ref().map(layers::active)
            && (modal::config(dialog).is_some() || popover::config(dialog).is_some())
        {
            if let UiEvent::PointerDown(p) = event
                && !self.has_capture()
                && !layout::contains(dialog.clip, p.x, p.y)
            {
                // Popover (menu) : un appui ailleurs que sur son ancre le ferme tout de
                // suite et atteint ce qu'il y a dessous, rejoué après la reconstruction,
                // comme une page web. Une modale garde son fond bloquant ; l'ancre referme
                // seulement son popover.
                let on_anchor = self
                    .root
                    .as_ref()
                    .and_then(|root| popover::anchor(root, dialog))
                    .is_some_and(|a| layout::contains(a, p.x, p.y));
                if modal::config(dialog).is_none()
                    && !on_anchor
                    && !self.interaction.replaying
                    && let Some(message) = popover::config(dialog).map(|c| c.on_dismiss.emit())
                {
                    self.send(message);
                    self.interaction.replay = Some(event.clone());
                    result.handled = true;
                    result.capture = true;
                    result.repaint = true;
                    return result;
                }
                self.interaction.backdrop_capture = Some(dialog.id);
                result.handled = true;
                result.capture = true;
                return result;
            }
            if let UiEvent::KeyDown {
                key: Key::Escape,
                repeat,
                ..
            } = event
                && !self.focused_input().is_some_and(|i| i.composing)
            {
                let dismiss = modal::config(dialog)
                    .filter(|m| m.close_on_escape)
                    .and_then(|m| m.on_dismiss.as_ref())
                    .or_else(|| popover::config(dialog).map(|p| &p.on_dismiss))
                    .map(Action::emit);
                if self.gesture_active() {
                    let reply = self.event(UiEvent::Cancel);
                    result.release |= reply.release;
                } else if !repeat && let Some(message) = dismiss {
                    self.send(message);
                }
                result.handled = true; // Une modale non fermable ne ferme jamais la fenêtre hôte.
                self.interaction.escape_consumed = true;
                result.repaint = self.invalidation.repaint || self.invalidation.rebuild;
                return result;
            }
        }
        if let Some(root) = &self.root {
            let captured = self.components.inputs.dragging.is_some();
            let response = self
                .components
                .inputs
                .event(&event, self.interaction.focused, root);
            if captured && self.components.inputs.dragging.is_none() {
                result.release = true;
            }
            self.invalidation.repaint |= response.repaint;
            if let Some(message) = response.key {
                self.send(message);
            }
            if let Some(message) = response.change {
                self.invalidation.semantics();
                self.send(message);
            }
            if let Some(message) = response.submit {
                self.send(message);
            }
            if response.handled {
                self.interaction.escape_consumed |= escape;
                result.handled = true;
                result.repaint = self.invalidation.pending();
                result.clipboard = response.clipboard;
                return result;
            }
        }
        match event {
            UiEvent::Scroll {
                position,
                delta_x,
                delta_y,
            } => {
                if !self.wheel_event(position, delta_x, delta_y, &mut result) {
                    return result;
                }
            }
            UiEvent::Wheel {
                position,
                delta_x,
                delta_y,
            } => {
                self.components.scrolls.wheel_smooth = !self.motions.reduced;
                let done = self.wheel_event(position, delta_x, delta_y, &mut result);
                self.components.scrolls.wheel_smooth = false;
                if !done {
                    return result;
                }
            }

            UiEvent::PointerMove(p) => self.pointer_move_event(p, &mut result),

            UiEvent::PointerDown(p) => self.pointer_down_event(p, extend, &mut result),

            UiEvent::PointerUp(p) => self.pointer_up_event(p, &mut result),

            UiEvent::PointerLeave => self.pointer_leave_event(),

            UiEvent::Cancel => self.cancel_pointer_event(&mut result),

            UiEvent::KeyDown {
                key: Key::Tab,
                shift,
                ..
            } => {
                result.handled = self.move_tab(shift);
            }

            UiEvent::KeyDown { key, repeat, .. } => self.key_down_event(key, repeat, &mut result),

            UiEvent::KeyUp(Key::Space) => self.space_up_event(&mut result),

            UiEvent::KeyUp(_) => (),

            UiEvent::PointerDownExtended(_) => unreachable!(),

            UiEvent::ContextMenu(_) => (),

            UiEvent::Text(_)
            | UiEvent::Edit(_)
            | UiEvent::CompositionStart
            | UiEvent::CompositionUpdate { .. }
            | UiEvent::CompositionCommit(_)
            | UiEvent::CompositionEnd
            | UiEvent::CaretTick
            | UiEvent::Timer(_)
            | UiEvent::FocusChanged(_) => (),
        }
        result.repaint = self.invalidation.pending();
        self.interaction.escape_consumed |= escape && result.handled;
        if self
            .root
            .as_ref()
            .is_some_and(|r| modal::config(layers::active(r)).is_some())
            && matches!(
                event,
                UiEvent::KeyDown { key: Key::Tab, .. }
                    | UiEvent::Scroll { .. }
                    | UiEvent::Wheel { .. }
            )
        {
            result.handled = true;
        }
        result
    }
}
