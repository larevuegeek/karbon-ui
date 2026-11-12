use super::*;
impl UiRuntime {
    pub(super) fn dismiss_popover(&mut self) -> bool {
        let action = self
            .root
            .as_ref()
            .and_then(popover::top)
            .and_then(popover::config)
            .map(|p| p.on_dismiss.emit());
        action.is_some_and(|message| self.send(message))
    }

    pub(super) fn context_menu_event(&mut self, position: Option<PointF>) -> EventResult {
        let mut result = EventResult::default();
        if self.components.edit_menu.is_open() {
            let response = self.components.edit_menu.close();
            return self.menu_response(response);
        }
        if self.root.as_ref().and_then(popover::top).is_some() {
            self.dismiss_popover();
            result.handled = true;
            result.repaint = self.invalidation.rebuild;
            return result;
        }
        if self.has_capture() || self.focused_input().is_some_and(|i| i.composing) {
            result.handled = true;
            return result;
        }
        if position.is_some_and(|p| !p.x.is_finite() || !p.y.is_finite()) {
            return result;
        }
        fn find(
            node: &Element,
            point: Option<PointF>,
            focused: Option<ElementId>,
        ) -> Option<&Element> {
            if let Some(p) = point {
                if !layout::contains(node.clip, p.x, p.y) {
                    return None;
                }
            } else if !focused.is_some_and(|id| node.find(id).is_some()) {
                return None;
            }
            if point.is_none()
                && focused == Some(node.id)
                && let Some(list) = &node.virtual_list
                && let Some(index) = list
                    .active
                    .and_then(|active| list.indices.iter().position(|i| *i == active))
                && let Some(row) = node.children.get(index)
            {
                let id = virtual_list::first_action(row).map_or(row.id, |(id, _)| id);
                if let Some(target) = find(row, None, Some(id)) {
                    return Some(target);
                }
            }
            for child in node.children.iter().rev() {
                if let Some(target) = find(child, point, focused) {
                    return Some(target);
                }
            }
            // La découpe seule ne suffit pas : en `overflow: visible`, celle d'un élément est
            // celle de son parent, et le dernier frère l'emporterait sur tout le parent.
            let inside = point.is_none_or(|p| {
                layout::contains(layout::intersect(node.bounds(), node.clip), p.x, p.y)
            });
            (inside
                && node.context_menu.is_some()
                && node.widget.as_ref().is_none_or(|w| w.semantics().enabled))
            .then_some(node)
        }
        let Some(root) = self.root.as_ref() else {
            return result;
        };
        let scope = layers::active(root);
        let Some(target) = find(scope, position, self.interaction.focused) else {
            // Sans gestionnaire applicatif : le menu intégré d'un champ de saisie.
            return self.open_edit_menu(position);
        };
        let id = target.id;
        let handler = Handler::clone(target.context_menu.as_ref().unwrap());
        let rect = self
            .interaction
            .focused
            .and_then(|id| target.find(id))
            .filter(|_| position.is_none())
            .map_or(target.clip, |n| n.clip);
        let point = position.unwrap_or(PointF {
            x: rect.left,
            y: rect.bottom,
        });
        let cancelled = self.event(UiEvent::Cancel);
        result.release = cancelled.release;
        if self.interaction.focus_order.contains(&id) {
            self.interaction.focused = Some(id);
        }
        self.send(handler.emit(ContextMenuRequest {
            target: id,
            position: point,
            keyboard: position.is_none(),
        }));
        result.handled = true;
        result.repaint = self.invalidation.repaint || self.invalidation.rebuild;
        result
    }

    /// Menu contextuel intégré (Couper, Copier, Coller, Tout sélectionner) du champ de
    /// saisie sous le pointeur, ou du champ focalisé depuis le clavier (sous son caret).
    /// Le champ visé au pointeur prend le focus.
    fn open_edit_menu(&mut self, position: Option<PointF>) -> EventResult {
        let id = match position {
            Some(p) => match self.hit(p) {
                Some(Hit::Input(id)) => id,
                _ => return EventResult::default(),
            },
            None => match self.interaction.focused {
                Some(id) => id,
                None => return EventResult::default(),
            },
        };
        let Some((input, content)) = self.root.as_ref().and_then(|root| {
            let node = root.find(id)?;
            let input = node.widget.as_ref()?.text_input()?;
            input.enabled.then(|| (input, layout::content_rect(node)))
        }) else {
            return EventResult::default();
        };
        let Some(snapshot) = self.components.inputs.snapshot(id, input) else {
            return EventResult::default();
        };
        let anchor = match position {
            Some(p) => RectF::new(p.x, p.y, 1., 1.),
            None => snapshot.caret,
        };
        let at = MenuAnchor {
            content,
            anchor,
            viewport: self.layer_viewport(),
            keyboard: position.is_none(),
        };
        let response = self.components.edit_menu.open(id, input, &snapshot, at);
        if position.is_some() && self.interaction.focused != Some(id) {
            self.interaction.focused = Some(id);
            self.interaction.focus_visible = false;
            self.interaction.keyboard_pressed = None;
        }
        self.menu_response(response)
    }

    /// Zone où se placent les couches (listes, menus, aides) : la fenêtre moins une marge,
    /// sous la barre de titre du moteur quand elle est affichée.
    pub(super) fn layer_viewport(&self) -> RectF {
        self.context.map_or(RectF::new(0., 0., 0., 0.), |c| {
            let top = if self.chrome.visible {
                TITLE_BAR_HEIGHT as f32
            } else {
                0.
            };
            RectF::new(
                4.,
                top + 4.,
                (c.width - 8.).max(0.),
                (c.height - 8.).max(0.),
            )
        })
    }
}
