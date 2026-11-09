//! Messages typés portés par les éléments, effacés jusqu'à l'adaptateur de l'application.
//!
//! Chaque élément possède ses gestionnaires : remplacer l'arbre remplace aussi ses
//! gestionnaires, sans registre global ni identifiant à invalider.
use std::{
    any::{Any, type_name},
    fmt,
    rc::Rc,
};

/// Message effacé, livré à `Application::update` après conversion vers son type concret.
pub(crate) struct Message {
    value: Box<dyn Any>,
    type_name: &'static str,
}
impl Message {
    pub(crate) fn new<M: 'static>(value: M) -> Self {
        // Message déjà construit (composants du kit) : transmis tel quel.
        let mut value = Some(value);
        let any: &mut dyn Any = &mut value;
        if let Some(slot) = any.downcast_mut::<Option<Message>>() {
            return slot.take().expect("message present");
        }
        if let Some(slot) = any.downcast_mut::<Option<Action>>() {
            return slot.take().expect("action present").emit();
        }
        if let Some(slot) = any.downcast_mut::<Option<crate::ui::kit::ValueAction>>() {
            return (slot.take().expect("action present").0)();
        }
        Self {
            value: Box::new(value.expect("value present")),
            type_name: type_name::<M>(),
        }
    }
    /// Message sans effet : contrôle du kit sans gestionnaire.
    pub(crate) fn none() -> Self {
        Self::new(Ignored)
    }
    /// Message sans effet, à ne pas livrer à l'application.
    pub(crate) fn ignored(&self) -> bool {
        self.value.is::<Ignored>()
    }
    /// Retrouver le message concret ; l'erreur conserve le nom du type réellement émis.
    /// Le message est-il de type `M` ?
    pub(crate) fn is<M: 'static>(&self) -> bool {
        self.value.is::<M>()
    }
    pub(crate) fn downcast<M: 'static>(self) -> Result<M, &'static str> {
        let type_name = self.type_name;
        self.value
            .downcast::<M>()
            .map(|value| *value)
            .map_err(|_| type_name)
    }
}
impl fmt::Debug for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Message<{}>", self.type_name)
    }
}

/// Contenu du message sans effet.
#[derive(Clone)]
struct Ignored;

/// Fonction applicative qui transforme une valeur de widget en message.
pub(crate) struct Handler<T>(Rc<dyn Fn(T) -> Message>);
impl<T: 'static> Handler<T> {
    pub(crate) fn new<M: 'static>(handler: impl Fn(T) -> M + 'static) -> Self {
        Self(Rc::new(move |value| Message::new(handler(value))))
    }
    pub(crate) fn emit(&self, value: T) -> Message {
        (self.0)(value)
    }
}
impl<T> Clone for Handler<T> {
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}
impl<T> fmt::Debug for Handler<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Handler")
    }
}

/// Message emitted when an element is activated: click, Enter/Space, or the
/// accessibility *Invoke*, *Toggle* and *Select* commands.
///
/// Built-in widgets create it from the message given to their constructor. Custom
/// [`Widget`](crate::ui::element::Widget)s store one and return it from
/// [`Widget::action`](crate::ui::element::Widget::action):
///
/// ```
/// use karbon_ui::application::Action;
/// #[derive(Clone)]
/// enum Msg { Play }
/// let play = Action::new(Msg::Play);
/// # let _ = play;
/// ```
#[derive(Clone)]
pub struct Action(Kind);
#[derive(Clone)]
enum Kind {
    Message(Rc<dyn Fn() -> Message>),
    /// Case ou interrupteur : la nouvelle valeur est calculée lors de l'activation.
    Toggle(Handler<bool>, bool),
}
impl Action {
    /// Emit a clone of `message` each time the element is activated.
    pub fn new<M: Clone + 'static>(message: M) -> Self {
        Self(Kind::Message(Rc::new(move || {
            Message::new(message.clone())
        })))
    }
    /// Action sans effet : contrôle du kit sans gestionnaire.
    pub(crate) fn none() -> Self {
        Self::new(Ignored)
    }
    pub(crate) fn toggle(handler: &Handler<bool>, value: bool) -> Self {
        Self(Kind::Toggle(handler.clone(), value))
    }
    pub(crate) fn emit(&self) -> Message {
        match &self.0 {
            Kind::Message(message) => message(),
            Kind::Toggle(handler, value) => handler.emit(*value),
        }
    }
}
impl fmt::Debug for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Action")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Debug, Clone, PartialEq)]
    enum Msg {
        Save,
        Name(String),
        Enabled(bool),
    }
    #[test]
    fn handlers_build_typed_messages_and_report_the_emitted_type() {
        assert_eq!(
            Action::new(Msg::Save).emit().downcast::<Msg>(),
            Ok(Msg::Save)
        );
        let name = Handler::new(Msg::Name);
        assert_eq!(
            name.clone().emit("Ada".into()).downcast::<Msg>(),
            Ok(Msg::Name("Ada".into()))
        );
        let toggle = Handler::new(Msg::Enabled);
        assert_eq!(
            Action::toggle(&toggle, true).emit().downcast::<Msg>(),
            Ok(Msg::Enabled(true))
        );
        let error = Action::new(7_u8).emit().downcast::<Msg>().unwrap_err();
        assert_eq!(error, "u8");
    }
}
