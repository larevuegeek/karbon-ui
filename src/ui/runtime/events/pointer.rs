use super::super::*;
impl UiRuntime {
    pub(super) fn pointer_move_event(&mut self, p: PointF, result: &mut EventResult) {
        let moved = self
            .interaction
            .pointer
            .is_none_or(|old| old.x != p.x || old.y != p.y);
        self.interaction.pointer = Some(p);
        if moved {
            self.interaction.hover_frozen = None;
        }
        if self.components.inputs.pointer_move(p) {
            self.invalidation.repaint = true;
            result.handled = true;
        }
        let hit = self.hit(p);
        if moved
            && !self.has_capture()
            && self
                .root
                .as_ref()
                .and_then(popover::top)
                .and_then(popover::config)
                // Une liste de suggestions laisse le focus dans son champ.
                .is_some_and(|p| !matches!(p.kind, PopoverKind::Panel | PopoverKind::Combobox))
            && let Some(Hit::Button(id)) = hit
        {
            self.interaction.focus_keyboard(id);
            self.invalidation.repaint = true;
        }
        if self.interaction.hovered.map(Hit::id) != hit.map(Hit::id) {
            self.interaction.hovered = hit;
            self.invalidation.repaint = true;
        }
        if let Some(drag) = self.components.splits.drag {
            if let Some(node) = self.root.as_ref().and_then(|r| r.find(drag.id)) {
                let position = if drag.axis == Axis::Horizontal {
                    p.x - layout::content_rect(node).left - layout::DIVIDER_SIZE / 2.
                } else {
                    p.y - layout::content_rect(node).top - layout::DIVIDER_SIZE / 2.
                };
                let available = (if drag.axis == Axis::Horizontal {
                    layout::width(layout::content_rect(node))
                } else {
                    layout::height(layout::content_rect(node))
                } - layout::DIVIDER_SIZE)
                    .max(1.);
                self.resize_split(drag.id, position / available);
            }
            result.handled = true;
        }
        if let Some(drag) = self.components.scrolls.drag
            && let Some(s) = self
                .root
                .as_ref()
                .and_then(|r| r.find(drag.id))
                .and_then(|n| n.scroll)
        {
            let coordinate = if s.axis == Axis::Horizontal { p.x } else { p.y };
            self.scroll_to(drag.id, s.drag_offset(coordinate, drag.grab));
            result.handled = true;
        }
    }

    pub(super) fn pointer_down_event(&mut self, p: PointF, extend: bool, result: &mut EventResult) {
        self.interaction.pointer = Some(p);
        self.interaction.focus_visible = false;
        self.interaction.keyboard_pressed = None;
        self.interaction.hovered = self.hit(p);
        match self.interaction.hovered {
            Some(Hit::Slider(id)) => {
                self.interaction.focused = Some(id);
                let response = self
                    .components
                    .sliders
                    .begin(id, p, self.root.as_ref().unwrap());
                let reply = self.slider_response(response);
                result.capture = reply.capture;
                result.release |= reply.release;
                result.handled = true;
            }
            Some(Hit::Input(id)) => {
                self.interaction.focused = Some(id);
                // L'horloge des animations n'avance que pendant une animation : sans
                // l'heure réelle, deux clics éloignés compteraient pour un double clic.
                let now = Instant::now().max(self.motions.time());
                self.components.inputs.pointer_down(id, p, extend, now);
                result.capture = true;
                result.handled = true;
            }
            Some(Hit::Button(id)) | Some(Hit::Select(id)) => {
                self.components.item_drag = self
                    .root
                    .as_ref()
                    .and_then(|r| r.find(id))
                    .and_then(|n| n.drag_source.as_deref())
                    .map(|source| drag_drop::State {
                        source: *source,
                        id,
                        start: p,
                        position: p,
                        active: false,
                    });
                self.interaction.focused = Some(id);
                if let Some((list, index)) = self
                    .root
                    .as_ref()
                    .and_then(|r| virtual_list::containing_row(r, id))
                {
                    self.select_virtual(list, index);
                }
                self.interaction.pressed = Some(id);
                result.capture = true;
                result.handled = true;
            }
            Some(Hit::Divider(id, axis)) => {
                let original = self
                    .root
                    .as_ref()
                    .and_then(|r| r.find(id))
                    .and_then(|n| {
                        if let Layout::Split(s) = n.layout {
                            Some(s.fraction)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(0.5);
                self.interaction.focused = Some(id);
                self.components.splits.drag = Some(Drag { id, axis, original });
                result.capture = true;
                result.handled = true;
            }
            Some(Hit::ScrollThumb(id)) => {
                let s = self
                    .root
                    .as_ref()
                    .unwrap()
                    .find(id)
                    .unwrap()
                    .scroll
                    .unwrap();
                let coordinate = if s.axis == Axis::Horizontal { p.x } else { p.y };
                self.interaction.focused = Some(id);
                self.components.scrolls.drag = Some(ScrollDrag {
                    id,
                    grab: coordinate - scroll::axis_start(s.thumb, s.axis),
                    original: s.offset,
                });
                result.capture = true;
                result.handled = true;
            }
            Some(Hit::ScrollTrack(id)) => {
                let s = self
                    .root
                    .as_ref()
                    .unwrap()
                    .find(id)
                    .unwrap()
                    .scroll
                    .unwrap();
                let coordinate = if s.axis == Axis::Horizontal { p.x } else { p.y };
                let direction = if coordinate < scroll::axis_start(s.thumb, s.axis) {
                    -1.
                } else {
                    1.
                };
                self.interaction.focused = Some(id);
                self.scroll_to(id, s.offset + direction * s.viewport_size() * 0.9);
                result.handled = true;
            }
            Some(Hit::ScrollArea(id)) => {
                self.interaction.focused = Some(id);
                if let Some(node) = self.root.as_ref().and_then(|r| r.find(id))
                    && let (Some(list), Some(s)) = (&node.virtual_list, node.scroll)
                    && layout::contains(s.viewport, p.x, p.y)
                {
                    let index = ((p.y - s.viewport.top + s.offset)
                        / list.stride(node.style.vertical_gap()))
                    .floor()
                    .max(0.) as usize;
                    self.select_virtual(id, index);
                }
                result.handled = true;
            }
            None => self.interaction.focused = None,
        }
        self.invalidation.repaint = true;
    }

    pub(super) fn pointer_up_event(&mut self, p: PointF, result: &mut EventResult) {
        self.interaction.pointer = Some(p);
        if self.components.inputs.dragging.take().is_some() {
            result.release = true;
            result.handled = true;
        }
        if self.components.scrolls.drag.take().is_some() {
            result.release = true;
            result.handled = true;
        }
        if self.components.splits.drag.take().is_some() {
            result.release = true;
            result.handled = true;
        }
        let released = self.hit(p).map(Hit::id);
        if let Some(id) = self.interaction.pressed.take() {
            result.release = true;
            result.handled = true;
            if released == Some(id) {
                self.activate(id);
            }
        }
        self.interaction.hovered = self.hit(p);
        self.invalidation.repaint = true;
    }

    pub(super) fn pointer_leave_event(&mut self) {
        self.interaction.pointer = None;
        self.interaction.hover_frozen = None;
        if self.interaction.hovered.take().is_some() {
            self.invalidation.repaint = true;
        }
    }

    pub(super) fn cancel_pointer_event(&mut self, result: &mut EventResult) {
        if self.components.inputs.dragging.take().is_some() {
            result.release = true;
            result.handled = true;
        }
        if let Some(drag) = self.components.scrolls.drag.take() {
            self.scroll_to(drag.id, drag.original);
            result.release = true;
            result.handled = true;
        }
        if let Some(drag) = self.components.splits.drag.take() {
            self.resize_split(drag.id, drag.original);
            result.release = true;
            result.handled = true;
        }
        if self.interaction.pressed.take().is_some() {
            result.release = true;
            result.handled = true;
        }
        self.interaction.keyboard_pressed = None;
        self.interaction.hovered = None;
        self.interaction.pointer = None;
        self.invalidation.repaint = true;
    }
}
