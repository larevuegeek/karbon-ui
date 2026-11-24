//! Ce que reçoivent la source et les cibles d'un glisser.
use super::Payload;
use crate::application::Message;
use std::rc::Rc;

/// The step of a free drag ([`DragEvent`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DragPhase {
    /// To the source: the pointer went past 4 logical units, the drag begins (the click
    /// on the source does not happen).
    Start,
    /// To the source: the pointer moved during the drag.
    Move,
    /// To a target under the pointer: return a message to accept the payload here, `None`
    /// to refuse it (the ancestors are asked next).
    Over,
    /// To the target that accepted last: the pointer left it, or the drag ended without
    /// a drop on it.
    Leave,
    /// To the target that accepted last: the payload is dropped on it.
    Drop,
    /// To the source: the drag is over, dropped on a target or not
    /// ([`DragEvent::dropped`], [`DragEvent::cancelled`]).
    End,
}

/// A step of a free drag, given to its source and to the targets under the pointer.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct DragEvent {
    /// Which step.
    pub phase: DragPhase,
    /// What is dragged.
    pub payload: Payload,
    /// Pointer position in window coordinates, logical pixels.
    pub x: f32,
    /// Pointer position in window coordinates, logical pixels.
    pub y: f32,
    /// Pointer position from the top left corner of the element whose handler receives
    /// the event, logical pixels.
    pub local_x: f32,
    /// Pointer position from the top left corner of the element, logical pixels.
    pub local_y: f32,
    /// Width of the element whose handler receives the event, logical pixels: with
    /// `local_x`, the fraction of a 3D view under the pointer.
    pub width: f32,
    /// Height of that element, logical pixels.
    pub height: f32,
    /// Pointer position on the screen, as
    /// [`PointerEvent::screen`](crate::ui::runtime::PointerEvent::screen): where a panel
    /// dropped outside the window should open.
    pub screen: (f32, f32),
    /// [`DragPhase::Move`] and [`DragPhase::End`]: a target accepts the payload under the
    /// pointer (the source shows « drop here »).
    pub accepted: bool,
    /// [`DragPhase::End`]: a target took the drop.
    pub dropped: bool,
    /// [`DragPhase::End`] and [`DragPhase::Leave`]: the drag was cancelled (Escape, the
    /// window lost the keyboard), not released.
    pub cancelled: bool,
}

impl DragEvent {
    /// An event of phase `phase` carrying `payload` at `(x, y)` (window and element), of
    /// an element of no size, nothing accepted, dropped nor cancelled: for the tests of an
    /// application, which then set the fields they need.
    pub fn new(phase: DragPhase, payload: Payload, x: f32, y: f32) -> Self {
        Self {
            phase,
            payload,
            x,
            y,
            local_x: x,
            local_y: y,
            width: 0.,
            height: 0.,
            screen: (x, y),
            accepted: false,
            dropped: false,
            cancelled: false,
        }
    }

    /// The pointer as fractions of the width and height of the element (`(0.5, 0.5)` at
    /// its middle), `None` for an element of no size.
    pub fn fraction(&self) -> Option<(f32, f32)> {
        (self.width > 0. && self.height > 0.)
            .then(|| (self.local_x / self.width, self.local_y / self.height))
    }
}

/// Gestionnaire `on_drag`, `on_drag_over` ou `on_drop` : `None` refuse (une cible) ou ne
/// dit rien (la source).
#[derive(Clone)]
pub(crate) struct DragHandler(pub(super) Rc<DragCall>);

/// Ce qu'appelle un gestionnaire de glisser.
type DragCall = dyn Fn(&DragEvent) -> Option<Message>;

impl DragHandler {
    /// Le gestionnaire qui rend le message de `handler`, s'il en rend un.
    pub(crate) fn new<M: 'static>(handler: impl Fn(&DragEvent) -> Option<M> + 'static) -> Self {
        Self(Rc::new(move |event| handler(event).map(Message::new)))
    }
}
