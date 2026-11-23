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
    /// The wheel turned, over an element with an `on_pointer_wheel` handler
    /// ([`Element::on_pointer_wheel`]); the amount is in [`PointerEvent::wheel`].
    Wheel,
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
    /// Rank of this press in a series of quick presses of the same button at the same
    /// place: `1` for a single click, `2` for a double click, `3` for a triple click. A
    /// release carries the rank of its press; a move and the wheel carry `0`. A press
    /// follows the previous one within 500 ms and 4 logical pixels (the defaults of
    /// Windows).
    pub clicks: u8,
    /// Wheel turn for [`PointerPhase::Wheel`], in notches (`1.0` per notch of a mouse
    /// wheel, fractions from a touchpad), positive to the right and towards the user
    /// (downwards); `(0.0, 0.0)` for the other phases. The vertical amount stays vertical
    /// with Shift held: the handler decides what Shift means.
    pub wheel: (f32, f32),
    /// The Windows key (Cmd on a Mac keyboard) is held.
    pub meta: bool,
    /// How far the pointer moved since the previous event, in logical pixels: for a move,
    /// from the previous position; while the pointer is locked
    /// ([`Element::pointer_lock`](crate::ui::element::Element::pointer_lock)), the motion
    /// of the mouse itself, which never stops at the edge of the screen. `(0.0, 0.0)` for
    /// a press, a release and the wheel.
    pub delta: (f32, f32),
}

/// Délai et distance qui font d'un appui la suite du précédent (double-clic), comme les
/// valeurs par défaut de Windows (`GetDoubleClickTime`, `SM_CXDOUBLECLK`).
const MULTI_CLICK_TIME: std::time::Duration = std::time::Duration::from_millis(500);
const MULTI_CLICK_DISTANCE: f32 = 4.;

/// Ce qu'un backend dit au runtime d'un bouton ou d'un mouvement de la souris.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PointerInput {
    pub phase: PointerPhase,
    pub button: Option<PointerButton>,
    pub position: PointF,
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
    pub meta: bool,
    /// Mouvement propre de la souris pendant que le pointeur est verrouillé (le backend
    /// le ramène à sa place à chaque fois) ; `None` ailleurs.
    pub motion: Option<(f32, f32)>,
}

/// Gestionnaire `on_pointer` : `None` laisse passer l'événement aux ancêtres.
#[derive(Clone)]
pub(crate) struct PointerHandler(pub(super) Rc<dyn Fn(PointerEvent) -> Option<Message>>);

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
    pub(super) target: Option<ElementId>,
    pub(super) held: u8,
    /// Dernier appui (instant, position, bouton) et son rang dans la série en cours.
    last_press: Option<(Instant, PointF, PointerButton)>,
    clicks: u8,
    /// Dernière position reçue, pour `PointerEvent::delta` (`lock.rs`).
    pub(super) last_position: Option<PointF>,
    /// L'élément qui accepte les fichiers survolés (`file_drop.rs`).
    pub(super) file_target: Option<ElementId>,
}

impl RawPointerState {
    /// Rang de l'appui `input` dans sa série (1, 2, 3…) ; le relâchement garde celui de
    /// son appui, un mouvement vaut 0.
    pub(crate) fn count_clicks(&mut self, input: &PointerInput, now: Instant) -> u8 {
        match (input.phase, input.button) {
            (PointerPhase::Down, Some(button)) => {
                let follows = self.last_press.is_some_and(|(at, place, previous)| {
                    previous == button
                        && now.saturating_duration_since(at) <= MULTI_CLICK_TIME
                        && (place.x - input.position.x).abs() <= MULTI_CLICK_DISTANCE
                        && (place.y - input.position.y).abs() <= MULTI_CLICK_DISTANCE
                });
                self.clicks = if follows {
                    self.clicks.saturating_add(1)
                } else {
                    1
                };
                self.last_press = Some((now, input.position, button));
                self.clicks
            }
            (PointerPhase::Up, Some(_)) => self.clicks,
            _ => 0,
        }
    }
}

/// Bit d'un bouton dans `RawPointerState::held`.
pub(super) fn bit(button: PointerButton) -> u8 {
    match button {
        PointerButton::Primary => 1,
        PointerButton::Secondary => 2,
        PointerButton::Middle => 4,
    }
}

/// Élément le plus profond sous `world` qui reçoit les événements du pointeur (cible de
/// l'événement du DOM).
pub(super) fn target(
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
        let clicks = self.raw_pointer.count_clicks(&input, Instant::now());
        let delta = self.raw_pointer.delta_of(&input);
        self.take_key_focus(&input);
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
                clicks,
                wheel: (0., 0.),
                meta: input.meta,
                delta,
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

/// Gestionnaires `on_pointer_wheel` de `target` et de ses ancêtres, de la cible à la
/// racine, chacun avec l'élément qui le porte.
fn wheel_handlers(
    node: &Element,
    target: ElementId,
    out: &mut Vec<(RectF, PointerHandler)>,
) -> bool {
    if node.id != target && !node.children.iter().any(|c| wheel_handlers(c, target, out)) {
        return false;
    }
    if let Some(handler) = node
        .listeners
        .as_ref()
        .and_then(|l| l.pointer_wheel.clone())
    {
        out.push((node.rect, handler));
    }
    true
}

impl UiRuntime {
    /// Donne un tour de molette (`wheel`, en crans) aux gestionnaires `on_pointer_wheel`
    /// sous le pointeur, du plus profond à la racine, jusqu'au premier qui rend un
    /// message. `handled` dit au backend de ne pas faire défiler.
    pub(crate) fn pointer_wheel(&mut self, input: PointerInput, wheel: (f32, f32)) -> EventResult {
        let mut result = EventResult::default();
        let Some(root) = self.root.as_ref() else {
            return result;
        };
        let scope = layers::active(root);
        let position = input.position;
        let Some(found) = fixed_result(scope, position, &self.motions, true, target)
            .or_else(|| target(scope, position, &self.motions, true))
        else {
            return result;
        };
        let mut path = Vec::new();
        wheel_handlers(scope, found, &mut path);
        for (rect, handler) in path {
            let event = PointerEvent {
                phase: PointerPhase::Wheel,
                button: None,
                x: position.x,
                y: position.y,
                local_x: position.x - rect.left,
                local_y: position.y - rect.top,
                shift: input.shift,
                control: input.control,
                alt: input.alt,
                clicks: 0,
                wheel,
                meta: input.meta,
                delta: (0., 0.),
            };
            if let Some(message) = (handler.0)(event) {
                self.send(message);
                result.handled = true;
                self.invalidation.repaint = true;
                result.repaint = true;
                break;
            }
        }
        result
    }
}
