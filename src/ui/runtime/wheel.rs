//! Événement molette côté application (`onwheel`) : les gestionnaires des éléments sous
//! le pointeur, du plus profond à la racine, comme la propagation du DOM. Le premier qui
//! rend un message consomme l'événement : ni les gestionnaires des ancêtres ni le
//! défilement n'ont lieu (`preventDefault()` et `stopPropagation()` d'un navigateur).
use super::hit_test::{fixed_result, pointer_events};
use super::*;
use std::rc::Rc;

/// Mouse wheel or touchpad scroll over an element with an `onwheel` handler
/// ([`Element::on_wheel`]), like the DOM `WheelEvent`.
///
/// Deltas are in logical pixels, positive to the right and downwards (`deltaX`,
/// `deltaY`); a wheel notch is usually 100 pixels or more, a touchpad sends small
/// precise deltas. Keyboard modifiers are not reported yet.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct WheelEvent {
    /// Horizontal delta (`deltaX`).
    pub delta_x: f32,
    /// Vertical delta (`deltaY`): negative when the wheel turns away from the user.
    pub delta_y: f32,
    /// Pointer position in window coordinates, logical pixels (`clientX`).
    pub x: f32,
    /// Pointer position in window coordinates, logical pixels (`clientY`).
    pub y: f32,
}

/// Gestionnaire `onwheel` : `None` laisse passer l'événement.
#[derive(Clone)]
pub(crate) struct WheelHandler(Rc<dyn Fn(WheelEvent) -> Option<Message>>);

impl Element {
    /// React to the mouse wheel over this element, like the DOM `wheel` event
    /// (`onwheel={…}` in templates): `handler` receives the [`WheelEvent`] and returns
    /// the message to send, or `None` to ignore it.
    ///
    /// The handlers of the elements under the pointer run from the deepest to the root.
    /// The first one returning a message consumes the event, as `preventDefault()` and
    /// `stopPropagation()` would: the handlers of its ancestors do not run and no
    /// container scrolls. When every handler returns `None`, the containers scroll as
    /// usual. Typical use: change the volume with the wheel over a player.
    pub fn on_wheel<M: 'static>(
        mut self,
        handler: impl Fn(WheelEvent) -> Option<M> + 'static,
    ) -> Self {
        self.on_wheel = Some(Box::new(WheelHandler(Rc::new(move |event| {
            handler(event).map(Message::new)
        }))));
        self
    }
}

impl UiRuntime {
    /// Gestionnaires `onwheel` sous le pointeur ; `true` quand l'un d'eux a consommé
    /// l'événement (message envoyé à l'application).
    pub(super) fn wheel_handlers(
        &mut self,
        position: PointF,
        delta_x: f32,
        delta_y: f32,
        result: &mut EventResult,
    ) -> bool {
        /// Élément le plus profond touché (cible de l'événement du DOM).
        fn walk(
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
                if let Some(id) = walk(child, world, motions, events) {
                    return Some(id);
                }
            }
            (events && layout::contains(node.rect, local.x, local.y)).then_some(node.id)
        }
        /// Gestionnaires de la cible et de ses ancêtres, de la cible à la racine.
        fn handlers(node: &Element, target: ElementId, out: &mut Vec<WheelHandler>) -> bool {
            if node.id != target && !node.children.iter().any(|c| handlers(c, target, out)) {
                return false;
            }
            out.extend(node.on_wheel.as_deref().cloned());
            true
        }
        let Some(root) = self.root.as_ref() else {
            return false;
        };
        let scope = layers::active(root);
        let Some(target) = fixed_result(scope, position, &self.motions, true, walk)
            .or_else(|| walk(scope, position, &self.motions, true))
        else {
            return false;
        };
        let mut path = Vec::new();
        handlers(scope, target, &mut path);
        let event = WheelEvent {
            delta_x,
            delta_y,
            x: position.x,
            y: position.y,
        };
        for handler in path {
            if let Some(message) = (handler.0)(event) {
                self.send(message);
                result.handled = true;
                self.invalidation.repaint = true;
                return true;
            }
        }
        false
    }
}
