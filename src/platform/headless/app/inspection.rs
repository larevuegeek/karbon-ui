//! Relecture de l'état : application, scène, arbre, bornes, focus, accessibilité,
//! presse-papiers simulé et rendu logiciel.
use super::{HeadlessApp, HeadlessOptions};
use crate::{
    application::Application,
    platform::Offscreen,
    ui::{
        element::{Element, ElementId},
        runtime::{AccessibleElement, UiRuntime},
        scene::{DrawCommand, Image, RectF, Scene},
    },
};
use std::time::Instant;

impl<A: Application> HeadlessApp<A> {
    /// The application, to assert on its state.
    pub fn app(&self) -> &A {
        self.ui
            .application::<A>()
            .expect("the headless runtime owns an application of type A")
    }
    /// Options of the simulated window, with its current size and theme.
    pub fn options(&self) -> HeadlessOptions {
        self.options
    }
    /// Current virtual time.
    pub fn now(&self) -> Instant {
        self.now
    }
    /// Scene painted by the last frame.
    pub fn scene(&self) -> &Scene {
        self.ui.scene()
    }
    /// Current view tree.
    pub fn root(&self) -> Option<&Element> {
        self.ui.root()
    }
    /// Element `id` of the current view tree, if present.
    pub fn find(&self, id: impl Into<ElementId>) -> Option<&Element> {
        self.ui.root()?.find(id.into())
    }
    /// Bounds of element `id` assigned by the last layout, in logical window
    /// coordinates.
    pub fn bounds(&self, id: impl Into<ElementId>) -> Option<RectF> {
        self.find(id).map(Element::bounds)
    }
    /// Element with keyboard focus, if any.
    pub fn focused(&self) -> Option<ElementId> {
        self.ui.focused()
    }
    /// Accessibility tree, as exposed to screen readers.
    pub fn accessibility(&self) -> Vec<AccessibleElement> {
        self.ui.accessibility()
    }
    /// Accessibility node of element `id`, if it is exposed.
    pub fn accessible(&self, id: impl Into<ElementId>) -> Option<AccessibleElement> {
        let id = id.into();
        self.ui
            .accessibility()
            .into_iter()
            .find(|node| node.id == id)
    }
    /// Current text of the text input `id`, including edits not yet echoed back by the
    /// application.
    pub fn input_value(&self, id: impl Into<ElementId>) -> Option<String> {
        self.ui.input_snapshot(id.into()).map(|input| input.value)
    }
    /// Every string drawn by the last frame, in paint order.
    pub fn texts(&self) -> Vec<&str> {
        self.ui
            .scene()
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. }
                | DrawCommand::FontText { text, .. }
                | DrawCommand::FormattedText { text, .. } => Some(&**text),
                DrawCommand::RichText { paragraph, .. } => Some(&*paragraph.text),
                _ => None,
            })
            .collect()
    }
    /// `true` if the last frame draws a string containing `needle`.
    pub fn contains_text(&self, needle: &str) -> bool {
        self.texts().iter().any(|text| text.contains(needle))
    }
    /// Simulated clipboard content.
    pub fn clipboard(&self) -> Option<&str> {
        self.clipboard.as_deref()
    }
    /// Replace the simulated clipboard content, read by paste commands.
    pub fn set_clipboard(&mut self, text: impl Into<String>) {
        self.clipboard = Some(text.into());
    }
    /// Rasterise the last frame with the headless software rasteriser, at `scale`
    /// device pixels per logical pixel (see the [module documentation](crate::platform::headless) for what
    /// is drawn exactly).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidInput`](crate::Error::InvalidInput) when `scale` is not positive
    /// or the scaled size is empty or above 16384 pixels per edge.
    pub fn render(&mut self, scale: f32) -> crate::Result<Image> {
        if !(scale.is_finite() && scale > 0.) {
            return Err(crate::Error::invalid_input("invalid render scale"));
        }
        let width = (self.options.width * scale).round() as u32;
        let height = (self.options.height * scale).round() as u32;
        let pixels = self
            .renderer
            .render(self.ui.scene(), width, height, scale)?;
        Image::new(width, height, pixels)
    }
    /// The underlying runtime, for queries the harness does not wrap.
    pub fn runtime(&self) -> &UiRuntime {
        &self.ui
    }
    /// Mutable access to the underlying runtime; call [`prepare`](Self::prepare)
    /// afterwards.
    pub fn runtime_mut(&mut self) -> &mut UiRuntime {
        &mut self.ui
    }
}
