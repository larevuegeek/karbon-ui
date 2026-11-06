//! Entrées simulées : pointeur, clavier, édition, défilement, focus et actions
//! d'accessibilité.
use super::HeadlessApp;
use crate::{
    application::Application,
    ui::{
        element::ElementId,
        runtime::{AccessibilityAction, Key, UiEvent},
        scene::PointF,
        text_input::EditCommand,
    },
};

impl<A: Application> HeadlessApp<A> {
    /// Center of the element `id` in window coordinates.
    fn center(&self, id: ElementId) -> crate::Result<PointF> {
        let r = self.bounds(id).ok_or_else(|| {
            crate::Error::invalid_input(format!("no element with id {:#x} in the view", id.0))
        })?;
        Ok(PointF {
            x: (r.left + r.right) / 2.,
            y: (r.top + r.bottom) / 2.,
        })
    }

    /// Move the pointer to the center of element `id`, then press and release the
    /// primary button there.
    ///
    /// The click is hit-tested like a real one: an overlay covering the element (modal,
    /// popover) receives it instead.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidInput`](crate::Error::InvalidInput) when no element has this ID,
    /// and the errors of [`prepare`](Self::prepare).
    pub fn click(&mut self, id: impl Into<ElementId>) -> crate::Result<()> {
        let point = self.center(id.into())?;
        self.click_at(point)
    }
    /// Move the pointer to `point`, then press and release the primary button.
    ///
    /// # Errors
    ///
    /// The errors of [`prepare`](Self::prepare).
    pub fn click_at(&mut self, point: PointF) -> crate::Result<()> {
        self.event(UiEvent::PointerMove(point))?;
        self.event(UiEvent::PointerDown(point))?;
        self.event(UiEvent::PointerUp(point))?;
        Ok(())
    }
    /// Move the pointer to the center of element `id`.
    ///
    /// # Errors
    ///
    /// Same as [`click`](Self::click).
    pub fn hover(&mut self, id: impl Into<ElementId>) -> crate::Result<()> {
        let point = self.center(id.into())?;
        self.event(UiEvent::PointerMove(point)).map(drop)
    }
    /// Type `text` into the focused element, one character at a time, as a keyboard
    /// would. Use [`press`](Self::press) for Enter, Tab and other keys.
    ///
    /// # Errors
    ///
    /// The errors of [`prepare`](Self::prepare).
    pub fn type_text(&mut self, text: &str) -> crate::Result<()> {
        let mut buffer = [0; 4];
        for c in text.chars() {
            self.event(UiEvent::Text((&*c.encode_utf8(&mut buffer)).into()))?;
        }
        Ok(())
    }
    /// Press and release `key`.
    ///
    /// # Errors
    ///
    /// The errors of [`prepare`](Self::prepare).
    pub fn press(&mut self, key: Key) -> crate::Result<()> {
        self.press_key(key, false)
    }
    /// Press and release `key` with Shift held (e.g. Shift+Tab).
    ///
    /// # Errors
    ///
    /// The errors of [`prepare`](Self::prepare).
    pub fn press_with_shift(&mut self, key: Key) -> crate::Result<()> {
        self.press_key(key, true)
    }
    fn press_key(&mut self, key: Key, shift: bool) -> crate::Result<()> {
        self.event(UiEvent::KeyDown {
            key,
            shift,
            repeat: false,
        })?;
        self.event(UiEvent::KeyUp(key)).map(drop)
    }
    /// Apply an editing command (copy, paste, select all, undo…) to the focused input.
    /// Copy and cut fill the simulated clipboard; paste reads it.
    ///
    /// # Errors
    ///
    /// The errors of [`prepare`](Self::prepare).
    pub fn edit(&mut self, command: EditCommand) -> crate::Result<()> {
        self.event(UiEvent::Edit(command)).map(drop)
    }
    /// Scroll by (`delta_x`, `delta_y`) logical pixels with the pointer over the center
    /// of element `id`; positive deltas scroll right and down.
    ///
    /// # Errors
    ///
    /// Same as [`click`](Self::click).
    pub fn scroll(
        &mut self,
        id: impl Into<ElementId>,
        delta_x: f32,
        delta_y: f32,
    ) -> crate::Result<()> {
        let position = self.center(id.into())?;
        self.event(UiEvent::Scroll {
            position,
            delta_x,
            delta_y,
        })
        .map(drop)
    }
    /// Move keyboard focus to element `id`, as an assistive technology would.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidInput`](crate::Error::InvalidInput) when the element is missing,
    /// disabled or not focusable, and the errors of [`prepare`](Self::prepare).
    pub fn focus(&mut self, id: impl Into<ElementId>) -> crate::Result<()> {
        self.accessibility_action(AccessibilityAction::Focus(id.into()))
    }
    /// Apply an [`AccessibilityAction`] (invoke, toggle, set a value…), then prepare the
    /// next frame.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidInput`](crate::Error::InvalidInput) carrying the
    /// [`AccessibilityError`](crate::ui::runtime::AccessibilityError) when the action is
    /// rejected, and the errors of [`prepare`](Self::prepare).
    pub fn accessibility_action(&mut self, action: AccessibilityAction) -> crate::Result<()> {
        self.prepare()?;
        let result = self.ui.accessibility_action(action);
        self.prepare()?;
        result.map_err(|error| {
            crate::Error::invalid_input(format!("accessibility action rejected: {error:?}"))
        })
    }
}
