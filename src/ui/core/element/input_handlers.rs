//! Gestionnaires d'entrées ajoutés pour les outils d'édition : le clavier adressé au
//! panneau qui a le focus (`on_key`) et le pointeur verrouillé (`pointer_lock`).
use super::Element;

impl Element {
    /// React to the keys while this element or one of its descendants has the focus: the
    /// keyboard focus (a control reached by Tab or clicked), or else the last press of a
    /// mouse button on it. `handler` returns the message to send, or `None` to let the
    /// key go to the ancestors, then to
    /// [`Application::key`](crate::application::Application::key) as before. Delete
    /// removes an object when the 3D view has the focus, and renames a line when the tree
    /// has it.
    ///
    /// Keys never reach these handlers while a text field has the keyboard (typing is the
    /// field's). A key a handler takes keeps its usual effect on the interface (Tab,
    /// Enter, Escape), like the keys of `Application::key`. The press that gives the
    /// focus comes from the window's mouse buttons, on Windows only for now.
    ///
    /// ```
    /// use karbon_ui::ui::{prelude::*, runtime::{KeyCode, KeyPress}};
    ///
    /// #[derive(Clone)]
    /// enum Msg {
    ///     RemoveObject,
    ///     RenameLine,
    /// }
    /// let view = column("view", []).on_key(|press: KeyPress| {
    ///     (press.pressed && press.key == KeyCode::Delete).then_some(Msg::RemoveObject)
    /// });
    /// let tree = column("tree", []).on_key(|press: KeyPress| {
    ///     (press.pressed && press.key == KeyCode::Delete).then_some(Msg::RenameLine)
    /// });
    /// # let _ = row("panels", [view, tree]);
    /// ```
    pub fn on_key<M: 'static>(
        mut self,
        handler: impl Fn(crate::ui::runtime::KeyPress) -> Option<M> + 'static,
    ) -> Self {
        let handler = crate::ui::runtime::raw_input::KeyHandler::new(handler);
        self.listeners_mut().key = Some(handler);
        self
    }

    /// React to files dragged from another program (the Explorer) over this element:
    /// [`FileDropEvent`](crate::ui::runtime::FileDropEvent) with the phase `Over` while
    /// they are above it, `Leave` when they leave it, `Drop` when they are dropped, and
    /// their full paths. `handler` returns the message to send to accept the files (the
    /// system shows the copy cursor, and `Drop` follows), or `None` to refuse them: they
    /// go to the ancestors, and with no taker the system shows the forbidden cursor. On
    /// Windows only for now.
    ///
    /// ```
    /// use karbon_ui::ui::{prelude::*, runtime::{FileDropEvent, FileDropPhase}};
    /// use std::path::PathBuf;
    ///
    /// #[derive(Clone)]
    /// enum Msg {
    ///     Hover(bool),
    ///     Import(Vec<PathBuf>),
    /// }
    /// let browser = column("assets", []).on_file_drop(|drop: &FileDropEvent| {
    ///     let models = drop.paths.iter().all(|p| p.extension().is_some_and(|e| e == "glb"));
    ///     match drop.phase {
    ///         FileDropPhase::Drop if models => Some(Msg::Import(drop.paths.to_vec())),
    ///         FileDropPhase::Leave => Some(Msg::Hover(false)),
    ///         _ if models => Some(Msg::Hover(true)),
    ///         _ => None,
    ///     }
    /// });
    /// # let _ = browser;
    /// ```
    pub fn on_file_drop<M: 'static>(
        mut self,
        handler: impl Fn(&crate::ui::runtime::FileDropEvent) -> Option<M> + 'static,
    ) -> Self {
        let handler = crate::ui::runtime::raw_input::FileDropHandler::new(handler);
        self.listeners_mut().file_drop = Some(handler);
        crate::ui::runtime::raw_input::FILE_DROP_BUILT.with(|built| built.set(true));
        self
    }

    /// Lock the pointer during the gestures this element takes with
    /// [`on_pointer`](Self::on_pointer), when `lock` is `true`: from the press it takes to
    /// the release of the last button, the pointer is hidden and stays where the press
    /// happened, and moves report the motion of the mouse in
    /// [`PointerEvent::delta`](crate::ui::runtime::PointerEvent::delta), which never
    /// stops at the edge of the screen (a free camera turned while the right button is
    /// held, a value dragged without end). The pointer comes back where it was when the
    /// gesture ends; losing the keyboard focus (Alt+Tab) ends the gesture, with a release
    /// of each held button. `false` changes nothing. On Windows only for now: elsewhere
    /// the gesture works as without the lock.
    ///
    /// ```
    /// use karbon_ui::ui::{prelude::*, runtime::{PointerButton, PointerEvent, PointerPhase}};
    ///
    /// #[derive(Clone)]
    /// enum Msg {
    ///     Turn(f32, f32),
    /// }
    /// let view = column("view", [])
    ///     .on_pointer(|e: PointerEvent| match (e.phase, e.button) {
    ///         (PointerPhase::Move, _) => Some(Msg::Turn(e.delta.0, e.delta.1)),
    ///         (_, Some(PointerButton::Secondary)) => Some(Msg::Turn(0., 0.)),
    ///         _ => None,
    ///     })
    ///     .pointer_lock(true);
    /// # let _ = view;
    /// ```
    pub fn pointer_lock(mut self, lock: bool) -> Self {
        if lock {
            self.listeners_mut().pointer_lock = true;
        }
        self
    }
}
