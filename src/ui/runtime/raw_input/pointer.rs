//! Les trois boutons de la souris sur un élément (`on_pointer`) : appui, mouvement,
//! relâchement, avec la position dans l'élément. L'élément dont le gestionnaire a pris un
//! appui reçoit la suite du geste, même hors de lui, jusqu'au dernier bouton relâché.
// Seul le backend Windows achemine ces entrées pour l'instant : ailleurs, elles ne
// servent qu'aux tests.
#![cfg_attr(not(windows), allow(dead_code))]
use super::super::hit_test::{fixed_result, pointer_events};
use super::super::*;
use std::rc::Rc;

/// A button of the mouse.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PointerButton {
    /// The main button, usually the left one.
    Primary,
    /// The other side button, usually the right one.
    Secondary,
    /// The wheel pressed down, or the middle button.
    Middle,
}

/// What happened to the pointer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PointerPhase {
    /// A button went down.
    Down,
    /// The pointer moved.
    Move,
    /// A button was released.
    Up,
}

/// A button of the mouse pressed, held while the pointer moves, or released, over an
/// element with an `on_pointer` handler ([`Element::on_pointer`]).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct PointerEvent {
    /// Down, move or up.
    pub phase: PointerPhase,
    /// The button that went down or up; `None` for a move.
    pub button: Option<PointerButton>,
    /// Pointer position in window coordinates, logical pixels.
    pub x: f32,
    /// Pointer position in window coordinates, logical pixels.
    pub y: f32,
    /// Pointer position from the top left corner of the element, logical pixels.
    pub local_x: f32,
    /// Pointer position from the top left corner of the element, logical pixels.
    pub local_y: f32,
    /// Shift is held.
    pub shift: bool,
    /// Ctrl is held.
    pub control: bool,
    /// Alt is held.
    pub alt: bool,
}

/// Ce qu'un backend dit au runtime d'un bouton ou d'un mouvement de la souris.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PointerInput {
    pub phase: PointerPhase,
    pub button: Option<PointerButton>,
    pub position: PointF,
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
}

/// Gestionnaire `on_pointer` : `None` laisse passer l'événement aux ancêtres.
#[derive(Clone)]
pub(crate) struct PointerHandler(Rc<dyn Fn(PointerEvent) -> Option<Message>>);

impl PointerHandler {
    /// Le gestionnaire qui rend le message de `handler`, s'il en rend un.
    pub(crate) fn new<M: 'static>(handler: impl Fn(PointerEvent) -> Option<M> + 'static) -> Self {
        Self(Rc::new(move |event| handler(event).map(Message::new)))
    }
}

/// L'élément qui a pris le geste en cours, et les boutons encore enfoncés (un bit par
/// bouton).
#[derive(Default)]
pub(crate) struct RawPointerState {
    target: Option<ElementId>,
    held: u8,
}

/// Bit d'un bouton dans `RawPointerState::held`.
fn bit(button: PointerButton) -> u8 {
    match button {
        PointerButton::Primary => 1,
        PointerButton::Secondary => 2,
        PointerButton::Middle => 4,
    }
}

/// Élément le plus profond sous `world` qui reçoit les événements du pointeur (cible de
/// l'événement du DOM).
fn target(
    node: &Element,
    world: PointF,
    motions: &crate::ui::animation::timeline::Timeline,
    events: bool,
) -> Option<ElementId> {
    if layout::hidden(node) || !layout::contains(motions.clip(node), world.x, world.y) {
        return None;
    }
    let local = motions.point(node.id, world)?;
    if layout::rounded_clip(node)
        .is_some_and(|(r, radii)| !layout::contains_rounded(r, radii, local))
    {
        return None;
    }
    let events = pointer_events(node, events);
    for index in layout::paint_order(node).rev() {
        let child = &node.children[index];
        if layout::fixed(child) {
            continue;
        }
        if let Some(id) = target(child, world, motions, events) {
            return Some(id);
        }
    }
    (events && layout::contains(node.rect, local.x, local.y)).then_some(node.id)
}

/// Gestionnaires de `target` et de ses ancêtres, de la cible à la racine, chacun avec
/// l'élément qui le porte.
fn handlers(
    node: &Element,
    target: ElementId,
    out: &mut Vec<(ElementId, RectF, PointerHandler)>,
) -> bool {
    if node.id != target && !node.children.iter().any(|c| handlers(c, target, out)) {
        return false;
    }
    if let Some(handler) = node.listeners.as_ref().and_then(|l| l.pointer.clone()) {
        out.push((node.id, node.rect, handler));
    }
    true
}

impl UiRuntime {
    /// Donne `input` aux gestionnaires `on_pointer` : à l'élément qui a pris le geste en
    /// cours s'il y en a un, sinon à ceux sous le pointeur, du plus profond à la racine,
    /// jusqu'au premier qui rend un message.
    pub(crate) fn pointer_input(&mut self, input: PointerInput) -> EventResult {
        let mut result = EventResult::default();
        let path = self.pointer_path(input.position);
        let mut taken = None;
        for (id, rect, handler) in path {
            let event = PointerEvent {
                phase: input.phase,
                button: input.button,
                x: input.position.x,
                y: input.position.y,
                local_x: input.position.x - rect.left,
                local_y: input.position.y - rect.top,
                shift: input.shift,
                control: input.control,
                alt: input.alt,
            };
            if let Some(message) = (handler.0)(event) {
                self.send(message);
                result.handled = true;
                self.invalidation.repaint = true;
                result.repaint = true;
                taken = Some(id);
                break;
            }
        }
        let state = &mut self.raw_pointer;
        match (input.phase, input.button) {
            (PointerPhase::Down, Some(button)) => {
                if state.target.is_none() {
                    state.target = taken;
                }
                if state.target.is_some() {
                    state.held |= bit(button);
                }
            }
            (PointerPhase::Up, Some(button)) => {
                state.held &= !bit(button);
                if state.held == 0 {
                    state.target = None;
                }
            }
            _ => {}
        }
        result
    }

    /// Un élément a pris un geste du pointeur, qui n'est pas fini (un bouton est encore
    /// enfoncé) : le backend garde la souris pour lui.
    pub(crate) fn pointer_gesture(&self) -> bool {
        self.raw_pointer.target.is_some()
    }

    /// Les gestionnaires à essayer pour un événement à `position` : celui de l'élément qui
    /// a pris le geste en cours, ou ceux sous le pointeur.
    fn pointer_path(&self, position: PointF) -> Vec<(ElementId, RectF, PointerHandler)> {
        let Some(root) = self.root.as_ref() else {
            return Vec::new();
        };
        if let Some(id) = self.raw_pointer.target {
            let held = root.find(id).and_then(|node| {
                let handler = node.listeners.as_ref()?.pointer.clone()?;
                Some((id, node.rect, handler))
            });
            return held.into_iter().collect();
        }
        let scope = layers::active(root);
        let Some(found) = fixed_result(scope, position, &self.motions, true, target)
            .or_else(|| target(scope, position, &self.motions, true))
        else {
            return Vec::new();
        };
        let mut path = Vec::new();
        handlers(scope, found, &mut path);
        path
    }
}
