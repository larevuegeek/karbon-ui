//! Application dialogs, focus stack and composition above the page.
use crate::application::Action;
use crate::ui::{element::*, scene::Scene, theme::Color};
use std::sync::Arc;

/// Options of a modal dialog, used with [`crate::ui::view::modal`] or
/// [`Element::modal`]. While open, the dialog traps focus and blocks input to
/// the page beneath; the application closes it by no longer including it in
/// the view.
#[derive(Clone)]
pub struct Modal {
    /// Accessible name of the dialog.
    pub label: Arc<str>,
    /// Message emitted when the user asks to close the dialog.
    pub on_dismiss: Option<Action>,
    /// Whether Escape requests dismissal (`true` by default).
    pub close_on_escape: bool,
    /// Whether a click on the backdrop requests dismissal (`false` by default).
    pub close_on_backdrop: bool,
    /// Element focused when the dialog opens; the first focusable one otherwise.
    pub initial_focus: Option<ElementId>,
    /// Colour drawn over the page behind the dialog (translucent black by default).
    pub backdrop: Color,
    /// Vertical placement of the dialog in the window.
    pub placement: ModalPlacement,
}
/// Vertical placement of a [`Modal`] dialog.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ModalPlacement {
    /// Centred in the window (the default).
    #[default]
    Center,
    /// Against the top of the window, centred horizontally; the dialog's own margins
    /// set the gap (Bootstrap's `.modal-dialog`, `margin: 1.75rem auto`).
    Top,
}
impl Modal {
    /// Dialog named `label`, closable with Escape once [`Modal::on_dismiss`] is set.
    pub fn new(label: impl Into<Arc<str>>) -> Self {
        Self {
            label: label.into(),
            on_dismiss: None,
            close_on_escape: true,
            close_on_backdrop: false,
            initial_focus: None,
            backdrop: Color::rgba(0, 0, 0, 150),
            placement: ModalPlacement::Center,
        }
    }
    /// Choose the vertical placement of the dialog.
    pub fn placement(mut self, placement: ModalPlacement) -> Self {
        self.placement = placement;
        self
    }
    /// Emit a clone of `message` when the user asks to close the dialog (Escape or
    /// backdrop click, as configured). Without it the dialog cannot be dismissed.
    pub fn on_dismiss<M: Clone + 'static>(mut self, message: M) -> Self {
        self.on_dismiss = Some(Action::new(message));
        self
    }
    /// Choose whether Escape requests dismissal.
    pub fn close_on_escape(mut self, enabled: bool) -> Self {
        self.close_on_escape = enabled;
        self
    }
    /// Choose whether a click on the backdrop requests dismissal.
    pub fn close_on_backdrop(mut self, enabled: bool) -> Self {
        self.close_on_backdrop = enabled;
        self
    }
    /// Focus the element `id` (inside the dialog) when it opens; the dialog's own id
    /// focuses the dialog itself, so that no control shows focus until Tab.
    pub fn initial_focus(mut self, id: impl Into<ElementId>) -> Self {
        self.initial_focus = Some(id.into());
        self
    }
    /// Set the backdrop colour.
    pub fn backdrop(mut self, color: Color) -> Self {
        self.backdrop = color;
        self
    }
}
impl Widget for Modal {
    fn measure(&self, _: SizeF, _: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
        Ok(SizeF::default())
    }
    fn paint(&self, _: &PaintContext, _: &mut Scene) {}
    fn modal(&self) -> Option<&Modal> {
        Some(self)
    }
    fn focusable(&self) -> bool {
        true // Repli pour un dialogue sans contrôle focalisable.
    }
    fn semantics(&self) -> Semantics<'_> {
        Semantics {
            role: Role::Dialog,
            name: &self.label,
            enabled: true,
        }
    }
}
pub(crate) fn config(node: &Element) -> Option<&Modal> {
    node.widget.as_ref()?.modal()
}
