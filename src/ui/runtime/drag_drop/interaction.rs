use super::super::*;
use crate::ui::drag_drop::{DragSource, DropEvent, DropPosition, DropTarget};

#[derive(Clone, Copy)]
pub(in crate::ui::runtime) struct State {
    pub source: DragSource,
    pub id: ElementId,
    pub start: PointF,
    pub position: PointF,
    pub active: bool,
}
fn target(node: &Element, point: PointF, group: u64) -> Option<(&DropTarget, RectF)> {
    if !layout::contains(node.clip, point.x, point.y)
        || node.widget.as_ref().is_some_and(|w| !w.semantics().enabled)
    {
        return None;
    }
    for child in node.children.iter().rev() {
        if let Some(found) = target(child, point, group) {
            return Some(found);
        }
    }
    node.drop_target
        .as_deref()
        .filter(|t| t.group == group)
        .filter(|_| layout::contains(node.rect, point.x, point.y))
        .map(|t| (t, layout::intersect(node.rect, node.clip)))
}
fn position(target: &DropTarget, rect: RectF, point: PointF) -> DropPosition {
    let before = match target.axis {
        Axis::Vertical => point.y < (rect.top + rect.bottom) / 2.,
        Axis::Horizontal => point.x < (rect.left + rect.right) / 2.,
    };
    if before {
        DropPosition::Before
    } else {
        DropPosition::After
    }
}
impl UiRuntime {
    pub(in crate::ui::runtime) fn drag_drop_event(
        &mut self,
        event: &UiEvent,
    ) -> Option<EventResult> {
        let state = self.components.item_drag?;
        match event {
            UiEvent::PointerMove(point) => {
                if !point.x.is_finite() || !point.y.is_finite() {
                    return Some(EventResult {
                        handled: true,
                        ..Default::default()
                    });
                }
                let active =
                    state.active || (point.x - state.start.x).hypot(point.y - state.start.y) >= 6.;
                if let Some(drag) = &mut self.components.item_drag {
                    drag.position = *point;
                    drag.active = active;
                }
                if !active {
                    return None;
                }
                self.interaction.pressed = None;
                self.interaction.pointer = Some(*point);
                self.invalidation.repaint = true;
                // Auto-scroll sur mouvement près du bord du viewport le plus profond.
                fn edge(node: &Element, p: PointF) -> Option<(ElementId, f32)> {
                    if !layout::contains(node.clip, p.x, p.y) {
                        return None;
                    }
                    for child in node.children.iter().rev() {
                        if let Some(s) = edge(child, p) {
                            return Some(s);
                        }
                    }
                    let s = node.scroll?;
                    if !layout::contains(s.viewport, p.x, p.y) {
                        return None;
                    }
                    let (coordinate, start, end) = match s.axis {
                        Axis::Vertical => (p.y, s.viewport.top, s.viewport.bottom),
                        Axis::Horizontal => (p.x, s.viewport.left, s.viewport.right),
                    };
                    let delta = if coordinate < start + 22. {
                        -18.
                    } else if coordinate > end - 22. {
                        18.
                    } else {
                        0.
                    };
                    (delta != 0. && s.max_offset() > 0.).then_some((node.id, s.offset + delta))
                }
                if let Some((id, offset)) = self
                    .root
                    .as_ref()
                    .and_then(|r| edge(layers::active(r), *point))
                {
                    self.scroll_to(id, offset);
                }
                Some(EventResult {
                    handled: true,
                    repaint: true,
                    ..Default::default()
                })
            }
            UiEvent::PointerUp(point) => {
                self.components.item_drag = None;
                if !state.active {
                    return None;
                }
                self.interaction.pressed = None;
                self.interaction.pointer = Some(*point);
                self.interaction.hovered = self.hit(*point);
                let message = self
                    .root
                    .as_ref()
                    .and_then(|r| target(layers::active(r), *point, state.source.group))
                    .filter(|(t, _)| t.key != state.source.key)
                    .map(|(t, rect)| {
                        t.on_drop.emit(DropEvent {
                            group: t.group,
                            source: state.source.key,
                            target: t.key,
                            position: position(t, rect, *point),
                        })
                    });
                if let Some(message) = message
                    && self.app.update(message)
                {
                    self.invalidate();
                }
                self.invalidation.repaint = true;
                Some(EventResult {
                    handled: true,
                    repaint: true,
                    release: true,
                    ..Default::default()
                })
            }
            UiEvent::KeyDown {
                key: Key::Escape, ..
            } => {
                self.components.item_drag = None;
                let mut reply = self.event(UiEvent::Cancel);
                reply.handled = true;
                reply.release = true;
                self.interaction.escape_consumed = true;
                Some(reply)
            }
            UiEvent::FocusChanged(false) => {
                self.components.item_drag = None;
                self.interaction.window_active = false;
                let mut reply = self.event(UiEvent::Cancel);
                reply.handled = true;
                reply.release = true;
                Some(reply)
            }
            UiEvent::Cancel => {
                self.components.item_drag = None;
                if state.active {
                    self.interaction.pressed = None;
                    self.interaction.hovered = None;
                    self.interaction.pointer = None;
                    self.invalidation.repaint = true;
                    Some(EventResult {
                        handled: true,
                        repaint: true,
                        release: true,
                        ..Default::default()
                    })
                } else {
                    None
                }
            }
            UiEvent::KeyDown { .. } if state.active => Some(EventResult {
                handled: true,
                ..Default::default()
            }),
            _ => None,
        }
    }
    pub(in crate::ui::runtime) fn paint_drag_drop(&mut self) {
        let Some(state) = self.components.item_drag.filter(|s| s.active) else {
            return;
        };
        let Some(root) = &self.root else {
            return;
        };
        let theme = self.context.unwrap().theme;
        if let Some(node) = root.find(state.id) {
            self.scene.commands.push(DrawCommand::Stroke {
                rect: layout::intersect(node.rect, node.clip),
                radius: 6.,
                width: 2.,
                color: theme.accent,
            });
        }
        if let Some((t, rect)) = target(layers::active(root), state.position, state.source.group) {
            let before = position(t, rect, state.position) == DropPosition::Before;
            let (start, end) = match t.axis {
                Axis::Vertical => {
                    let y = if before { rect.top } else { rect.bottom };
                    (PointF { x: rect.left, y }, PointF { x: rect.right, y })
                }
                Axis::Horizontal => {
                    let x = if before { rect.left } else { rect.right };
                    (PointF { x, y: rect.top }, PointF { x, y: rect.bottom })
                }
            };
            self.scene.commands.push(DrawCommand::Line {
                start,
                end,
                width: 3.,
                color: theme.accent,
            });
        }
    }
}
