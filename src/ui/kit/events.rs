//! Attributs d'événement des composants : `onclick={Msg::Save}`, `onchange={Msg::Volume}`.
use crate::application::{Action, Handler, Message};
use crate::ui::template::IntoProp;

/// Event prop of a component activated without a value (`onclick={Msg::Save}`).
#[derive(Default)]
pub struct OnClick(Option<Action>);

impl<M: Clone + 'static> IntoProp<OnClick> for M {
    fn into_prop(self) -> OnClick {
        OnClick(Some(Action::new(self)))
    }
}
impl OnClick {
    /// L'action à poser sur le contrôle natif ; sans gestionnaire, elle ne fait rien.
    pub(super) fn action(self) -> Action {
        self.0.unwrap_or_else(Action::none)
    }
    pub(super) fn is_set(&self) -> bool {
        self.0.is_some()
    }
}

/// Event prop of a component that reports a value (`onchange={Msg::Volume}`, or a
/// closure such as `onchange={|v| Msg::Set(3, v)}`).
pub struct OnChange<T>(Option<Handler<T>>);

impl<T> Default for OnChange<T> {
    fn default() -> Self {
        Self(None)
    }
}
impl<T: 'static, F, M: 'static> IntoProp<OnChange<T>> for F
where
    F: Fn(T) -> M + 'static,
{
    fn into_prop(self) -> OnChange<T> {
        OnChange(Some(Handler::new(self)))
    }
}
impl<T: 'static> OnChange<T> {
    /// Fonction à poser sur le contrôle natif ; sans gestionnaire, elle ne fait rien.
    pub(super) fn emitter(self) -> impl Fn(T) -> Message + Clone + 'static {
        let handler = self.0;
        move |value| match &handler {
            Some(handler) => handler.emit(value),
            None => Message::none(),
        }
    }
    /// Action qui émet `value` à l'activation.
    pub(super) fn action(&self, value: T) -> Action
    where
        T: Clone,
    {
        match &self.0 {
            Some(handler) => {
                let handler = handler.clone();
                Action::new(ValueAction(std::rc::Rc::new(move || {
                    handler.emit(value.clone())
                })))
            }
            None => Action::none(),
        }
    }
}

/// Message calculé au moment de l'activation (voir `Message::new`).
#[derive(Clone)]
pub(crate) struct ValueAction(pub(crate) std::rc::Rc<dyn Fn() -> Message>);
