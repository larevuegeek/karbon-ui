//! La part d'un `split` rendue à l'application quand l'utilisateur la change.
use super::Element;
use crate::application::Handler;

impl Element {
    /// On a [`split`](Element::split) element: receive the new share of the first pane
    /// (0 to 1) each time the user moves the divider, with the mouse or the keyboard,
    /// to keep it (a saved layout, a [`DockTree`](crate::ui::dock::DockTree)). The
    /// runtime keeps the share across rebuilds either way; without this handler the
    /// divider behaves as before.
    ///
    /// ```
    /// use karbon_ui::ui::{element::Split, prelude::*};
    ///
    /// #[derive(Clone)]
    /// enum Msg {
    ///     Share(f32),
    /// }
    /// let panes = Element::split(1, Split::horizontal(0.3), column("a", []), column("b", []))
    ///     .on_split(Msg::Share);
    /// # let _ = panes;
    /// ```
    pub fn on_split<M: 'static>(mut self, handler: impl Fn(f32) -> M + 'static) -> Self {
        self.listeners_mut().split = Some(Handler::new(handler));
        self
    }
}
