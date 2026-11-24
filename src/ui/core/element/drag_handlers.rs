//! Le glisser libre entre panneaux (`ui::drag`) : les données qu'un élément porte, la
//! source qui suit le geste, les cibles qui acceptent et reçoivent le lâcher.
use super::Element;
use crate::ui::drag::{DragEvent, DragHandler, Payload};

impl Element {
    /// Make this element (and what it contains) draggable with the main button, carrying
    /// `payload`: past 4 logical units the press becomes a drag, the click does not
    /// happen, and the targets under the pointer are asked whether they accept it
    /// ([`on_drag_over`](Self::on_drag_over)). A press taken by an
    /// [`on_pointer`](Self::on_pointer) handler does not start a drag. Escape cancels it.
    ///
    /// ```
    /// use karbon_ui::ui::{drag::{DragEvent, DragPhase, Payload}, prelude::*};
    ///
    /// #[derive(Clone)]
    /// enum Msg {
    ///     Dragging(Option<(f32, f32)>),
    /// }
    /// let thumbnail = column("brick", [])
    ///     .drag_payload(Payload::new("texture", "Brique", 42_u64))
    ///     .on_drag(|e: &DragEvent| match e.phase {
    ///         DragPhase::End => Some(Msg::Dragging(None)),
    ///         _ => Some(Msg::Dragging(Some((e.x, e.y)))),
    ///     });
    /// # let _ = thumbnail;
    /// ```
    pub fn drag_payload(mut self, payload: Payload) -> Self {
        self.listeners_mut().drag_payload = Some(payload);
        self
    }

    /// Follow the drag of this source ([`drag_payload`](Self::drag_payload)): its start,
    /// each move (with whether a target accepts under the pointer), and its end (dropped
    /// or not, cancelled or not, where on the screen). The application draws the preview
    /// from it ([`drag::preview`](crate::ui::drag::preview)). `None` sends nothing.
    pub fn on_drag<M: 'static>(
        mut self,
        handler: impl Fn(&DragEvent) -> Option<M> + 'static,
    ) -> Self {
        self.listeners_mut().drag = Some(DragHandler::new(handler));
        self
    }

    /// Say whether this element accepts what is dragged over it: `handler` receives
    /// [`DragPhase::Over`](crate::ui::drag::DragPhase::Over) at each move above it and
    /// returns the message to send to accept (its look changes: the application keeps
    /// « hovered » from it), or `None` to refuse (the ancestors are asked next). It
    /// receives [`DragPhase::Leave`](crate::ui::drag::DragPhase::Leave) when the
    /// pointer leaves it or the drag is cancelled.
    ///
    /// ```
    /// use karbon_ui::ui::{drag::{DragEvent, DragPhase}, prelude::*};
    ///
    /// #[derive(Clone)]
    /// enum Msg {
    ///     Aim(Option<(f32, f32)>),
    ///     Place(u64, (f32, f32)),
    /// }
    /// let view = column("view", [])
    ///     .on_drag_over(|e: &DragEvent| match e.phase {
    ///         DragPhase::Over if e.payload.is("texture") => Some(Msg::Aim(e.fraction())),
    ///         DragPhase::Leave => Some(Msg::Aim(None)),
    ///         _ => None,
    ///     })
    ///     .on_drop(|e: &DragEvent| {
    ///         let asset = *e.payload.get::<u64>()?;
    ///         Some(Msg::Place(asset, e.fraction()?))
    ///     });
    /// # let _ = view;
    /// ```
    pub fn on_drag_over<M: 'static>(
        mut self,
        handler: impl Fn(&DragEvent) -> Option<M> + 'static,
    ) -> Self {
        self.listeners_mut().drag_over = Some(DragHandler::new(handler));
        self
    }

    /// Receive what is dropped on this element, once it accepted it
    /// ([`on_drag_over`](Self::on_drag_over)): `handler` gets
    /// [`DragPhase::Drop`](crate::ui::drag::DragPhase::Drop) and returns the message to
    /// send (`None`: the drop does not take place, the source learns it).
    pub fn on_drop<M: 'static>(
        mut self,
        handler: impl Fn(&DragEvent) -> Option<M> + 'static,
    ) -> Self {
        self.listeners_mut().drop = Some(DragHandler::new(handler));
        self
    }
}
