use super::*;
impl UiRuntime {
    pub(crate) fn font_assets(&self) -> &[crate::ui::font::FontAsset] {
        self.app.fonts()
    }

    /// Application typée, pour le harnais headless qui connaît son type concret.
    pub(crate) fn application<A: Application>(&self) -> Option<&A> {
        self.app.as_any().downcast_ref()
    }
    pub(crate) fn application_mut<A: Application>(&mut self) -> Option<&mut A> {
        self.app.as_any_mut().downcast_mut()
    }
    /// Livrer un message comme s'il venait d'un élément de la vue.
    pub(crate) fn deliver(&mut self, message: Message) -> bool {
        self.send(message)
    }

    /// Drain the background work requested by the application
    /// (see [`Application::take_task_commands`]).
    pub fn take_task_commands(&mut self) -> Vec<crate::task::TaskCommand> {
        self.app.take_task_commands()
    }

    /// Deliver a finished task to the application on the UI thread; returns `true` if a
    /// new frame is needed: the application asked for a rebuild (the view is
    /// invalidated), or the task delivered images to
    /// [`ImageSlot`](crate::ui::template::ImageSlot)s (they are repainted, without
    /// rebuilding the view).
    pub fn task_completed(&mut self, result: crate::task::TaskResult) -> bool {
        let changed = self.app.task_completed(result);
        if changed {
            self.invalidate();
        }
        self.images_arrived() || changed
    }

    /// Drain the application's window requests for the window-management loop.
    pub fn take_window_commands(&mut self) -> Vec<crate::window::WindowCommand> {
        self.app.take_window_commands()
    }

    /// Deliver the window's new geometry and state to the application; returns `true`
    /// (and invalidates the view) if the application asked for a rebuild.
    pub fn window_changed(&mut self, state: crate::window::WindowState) -> bool {
        let changed = self.app.window_changed(state);
        if changed {
            self.invalidate();
        }
        changed
    }
}
