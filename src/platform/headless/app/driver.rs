//! Pilotage du harnais : messages, état de l'application, fenêtre simulée, horloge
//! virtuelle et tâches de fond.
use super::HeadlessApp;
use crate::{
    application::{Application, Message},
    ui::{runtime::UiEvent, theme::Theme},
    window::WindowCommand,
};
use std::time::Duration;

impl<A: Application> HeadlessApp<A> {
    /// Deliver `message` to [`Application::update`] as if an element had emitted it,
    /// then prepare the next frame. Returns what `update` returned.
    ///
    /// # Errors
    ///
    /// The errors of [`prepare`](Self::prepare).
    pub fn send(&mut self, message: A::Message) -> crate::Result<bool> {
        self.prepare()?;
        let rebuilt = self.ui.deliver(Message::new(message));
        self.prepare()?;
        Ok(rebuilt)
    }
    /// Resize the simulated window to `width` × `height` logical pixels (title bar
    /// included) and lay the view out again.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidInput`](crate::Error::InvalidInput) for a negative or non-finite
    /// size, and the errors of [`prepare`](Self::prepare).
    pub fn resize(&mut self, width: f32, height: f32) -> crate::Result<()> {
        let previous = self.options;
        self.options.width = width;
        self.options.height = height;
        if let Err(error) = self.context() {
            self.options = previous;
            return Err(error);
        }
        self.prepare()
    }
    /// Switch the theme passed to [`Application::view`] and rebuild the view.
    ///
    /// # Errors
    ///
    /// The errors of [`prepare`](Self::prepare).
    pub fn set_theme(&mut self, theme: Theme) -> crate::Result<()> {
        self.options.theme = theme;
        self.prepare()
    }
    /// Move the virtual clock forward by `duration`, deliver the timer event a window
    /// would send, and prepare the frame at the new time (animations and transitions
    /// progress accordingly).
    ///
    /// # Errors
    ///
    /// The errors of [`prepare`](Self::prepare).
    pub fn advance(&mut self, duration: Duration) -> crate::Result<()> {
        self.now += duration;
        self.event(UiEvent::Timer(self.now)).map(drop)
    }
    /// Run the background tasks requested by the application (and those their results
    /// request in turn) on the worker pool until none is left, delivering each result
    /// to [`Application::task_completed`], then prepare the next frame. Window requests
    /// made meanwhile are kept for [`take_window_commands`](Self::take_window_commands).
    ///
    /// # Errors
    ///
    /// [`Error::TimedOut`](crate::Error::TimedOut) when tasks are still running after ten
    /// seconds, [`Error::Io`](crate::Error::Io) when the worker pool cannot start, and
    /// the errors of [`prepare`](Self::prepare).
    pub fn run_tasks(&mut self) -> crate::Result<()> {
        let Self {
            ui,
            executor,
            window_commands,
            ..
        } = self;
        crate::platform::offscreen::settle_tasks(
            ui,
            executor,
            Duration::from_secs(10),
            "headless tasks",
            |commands| {
                window_commands.extend(commands);
                Ok(())
            },
        )?;
        self.prepare()
    }
    /// Drain the window commands (open, close, exit, title bar actions) requested by the
    /// application so far. The headless harness never executes them.
    pub fn take_window_commands(&mut self) -> Vec<WindowCommand> {
        let mut commands = std::mem::take(&mut self.window_commands);
        commands.extend(self.ui.take_window_commands());
        commands
    }
    /// Change the application directly, then rebuild and prepare the view.
    ///
    /// # Errors
    ///
    /// The errors of [`prepare`](Self::prepare).
    pub fn update_app(&mut self, change: impl FnOnce(&mut A)) -> crate::Result<()> {
        change(
            self.ui
                .application_mut::<A>()
                .expect("the headless runtime owns an application of type A"),
        );
        self.ui.invalidate();
        self.prepare()
    }
}
