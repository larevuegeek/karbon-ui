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

    /// Ask the application whether the window may close
    /// ([`Application::close_requested`]). A refusal rebuilds the view, so the
    /// application can show its question.
    pub fn close_requested(&mut self) -> bool {
        let allowed = self.app.close_requested();
        if !allowed {
            self.invalidate();
        }
        allowed
    }

    /// Drain the background work requested by the application
    /// (see [`Application::take_task_commands`]), followed by the decodes of its
    /// [`ImageLoader`](crate::resources::ImageLoader) and the subscription of its
    /// [`Inbox`](crate::task::Inbox).
    pub fn take_task_commands(&mut self) -> Vec<crate::task::TaskCommand> {
        let mut commands = self.app.take_task_commands();
        let cancel_all = commands
            .iter()
            .any(|c| matches!(c, crate::task::TaskCommand::CancelAll));
        if let Some(images) = self.app.image_loader() {
            // Un `CancelAll` de l'application emporte aussi les chargements en vol :
            // ils sont redemandés, après lui.
            if cancel_all {
                images.cancelled_all();
            }
            commands.extend(images.take_commands());
        }
        // Boîte aux lettres : la fenêtre s'abonne (de nouveau après un `CancelAll`, qui
        // emporte aussi l'abonnement), sans occuper d'ouvrier.
        if let Some(inbox) = self.app.inbox()
            && inbox.wants_listener(cancel_all)
        {
            commands.push(crate::task::TaskCommand::Spawn(crate::task::Task::listen(
                crate::task::INBOX_TASK,
                inbox.listener(),
            )));
        }
        commands
    }

    /// Deliver a finished task to the application on the UI thread; returns `true` if a
    /// new frame is needed: the application asked for a rebuild (the view is
    /// invalidated), or the task delivered images to
    /// [`ImageSlot`](crate::ui::template::ImageSlot)s (they are repainted, without
    /// rebuilding the view). Decodes of the application's
    /// [`ImageLoader`](crate::resources::ImageLoader) and the wake-ups of its
    /// [`Inbox`](crate::task::Inbox) are handled here, not passed on.
    pub fn task_completed(&mut self, result: crate::task::TaskResult) -> bool {
        let inbox = (result.id == crate::task::INBOX_TASK)
            .then(|| self.app.inbox())
            .flatten();
        let changed = if let Some(inbox) = inbox {
            inbox.deliver_any(self.app.as_any_mut())
        } else {
            match self.app.image_loader() {
                Some(images) if images.owns(result.id) => images.completed(result),
                _ => self.app.task_completed(result),
            }
        };
        if changed {
            self.invalidate();
        }
        self.images_arrived() || changed
    }

    /// Drain the application's window requests for the window-management loop.
    pub fn take_window_commands(&mut self) -> Vec<crate::window::WindowCommand> {
        let mut commands = self.app.take_window_commands();
        if self.probe_take_exit() {
            commands.push(crate::window::WindowCommand::Exit);
        }
        commands
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

    /// Deliver a click or a menu choice on the window's tray icon to the application;
    /// returns `true` (and invalidates the view) if the application asked for a rebuild.
    pub fn tray_event(&mut self, event: crate::window::TrayEvent) -> bool {
        let changed = self.app.tray_event(event);
        if changed {
            self.invalidate();
        }
        changed
    }
}
