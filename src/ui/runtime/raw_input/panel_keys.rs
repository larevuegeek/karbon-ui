//! Le clavier adressé au panneau qui a le focus (`Element::on_key`) : une touche va
//! d'abord à l'élément qui a le focus clavier, ou au dernier panneau où l'on a appuyé,
//! puis remonte à ses ancêtres tant que leurs gestionnaires la laissent passer, et
//! finit à `Application::key` comme avant.
use super::super::hit_test::fixed_result;
use super::super::*;
use super::{KeyPress, PointerInput, PointerPhase, pointer::target};
use std::rc::Rc;

/// Gestionnaire `on_key` : `None` laisse passer la touche aux ancêtres.
#[derive(Clone)]
pub(crate) struct KeyHandler(Rc<dyn Fn(KeyPress) -> Option<Message>>);

impl KeyHandler {
    /// Le gestionnaire qui rend le message de `handler`, s'il en rend un.
    pub(crate) fn new<M: 'static>(handler: impl Fn(KeyPress) -> Option<M> + 'static) -> Self {
        Self(Rc::new(move |press| handler(press).map(Message::new)))
    }
}

/// Le panneau où l'on a appuyé en dernier, et le focus clavier à cet instant : si le
/// focus a changé depuis (Tab), c'est lui qui dit où vont les touches.
#[derive(Default)]
pub(crate) struct KeyFocus {
    pressed: Option<ElementId>,
    focus_then: Option<ElementId>,
}

/// Gestionnaires `on_key` de `start` et de ses ancêtres, de lui à la racine.
fn key_handlers(node: &Element, start: ElementId, out: &mut Vec<KeyHandler>) -> bool {
    if node.id != start && !node.children.iter().any(|c| key_handlers(c, start, out)) {
        return false;
    }
    if let Some(handler) = node.listeners.as_ref().and_then(|l| l.key.clone()) {
        out.push(handler);
    }
    true
}

impl UiRuntime {
    /// Un appui de n'importe quel bouton : l'élément sous le pointeur devient le point de
    /// départ des touches (le panneau qui a le focus).
    pub(super) fn take_key_focus(&mut self, input: &PointerInput) {
        if input.phase != PointerPhase::Down {
            return;
        }
        let Some(root) = self.root.as_ref() else {
            return;
        };
        let scope = layers::active(root);
        let position = input.position;
        let found = fixed_result(scope, position, &self.motions, true, target)
            .or_else(|| target(scope, position, &self.motions, true));
        self.key_focus = KeyFocus {
            pressed: found,
            focus_then: self.interaction.focused,
        };
    }

    /// L'élément d'où partent les touches : le focus clavier s'il a changé depuis le
    /// dernier appui (Tab), sinon l'élément de cet appui.
    pub(crate) fn key_focus_start(&self) -> Option<ElementId> {
        let focused = self.interaction.focused;
        if focused.is_some() && focused != self.key_focus.focus_then {
            return focused;
        }
        self.key_focus.pressed.or(focused)
    }

    /// Donne `press` aux gestionnaires `on_key`, du point de départ à la racine, jusqu'au
    /// premier qui rend un message. `true` quand l'un l'a prise : `Application::key` ne
    /// la voit pas.
    pub(super) fn panel_key(&mut self, press: KeyPress) -> bool {
        let (Some(root), Some(start)) = (self.root.as_ref(), self.key_focus_start()) else {
            return false;
        };
        let mut handlers = Vec::new();
        key_handlers(root, start, &mut handlers);
        for handler in handlers {
            if let Some(message) = (handler.0)(press) {
                self.send(message);
                return true;
            }
        }
        false
    }
}
