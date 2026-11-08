//! Boucle d'événements X11 (XCB, chargé au lancement) : fenêtres de premier niveau
//! dessinées par Vulkan (chaîne d'échange XCB) ou par Cairo (`xcb_put_image`), minuteries,
//! réveil des tâches de fond et accessibilité AT-SPI. Choisie quand il n'y a pas de
//! compositeur Wayland (`WAYLAND_DISPLAY` absent), ou par `KARBON_BACKEND=x11` (sous
//! XWayland par exemple).
//!
//! La boucle attend (`poll`) la connexion X, un `eventfd` écrit par les tâches de fond et
//! le bus d'accessibilité, jusqu'au prochain réveil demandé. Contrairement à Wayland, X11
//! n'a pas de rappel de trame : une fenêtre affichée est peinte dès qu'elle a changé, au
//! rythme de l'écran sous Vulkan (« fifo »).
mod clipboard;
mod connection;
mod events;
mod ime;
mod input;
#[cfg(test)]
mod tests;
mod window;
mod xinput;

use crate::{
    platform::linux::{
        ffi::xcb::*,
        shell::{RunGuard, Wake, composition::Composition, keyboard::Keys},
        vulkan,
    },
    task::TaskExecutor,
    ui::{runtime::UiEvent, scene::PointF},
    window::{ExitPolicy, WindowAction, WindowCommand, WindowDescriptor, WindowId},
};
use clipboard::Clipboard;
use connection::Connection;
use input::Cursors;
use std::{
    collections::VecDeque,
    panic::{AssertUnwindSafe, catch_unwind},
    rc::Rc,
    time::Instant,
};
use window::Window;

struct X11 {
    // Les fenêtres (donc les applications et les surfaces Vulkan) sont détruites avant la
    // connexion.
    windows: Vec<Window>,
    /// Contexte Vulkan partagé par les fenêtres (pas encore tenté, ou absent).
    gpu: Option<Option<Rc<vulkan::Gpu>>>,
    /// Pont AT-SPI (absent sans bus d'accessibilité).
    accessibility: Option<crate::platform::linux::accessibility::Bridge>,
    wake: Wake,
    executor: Option<TaskExecutor>,
    policy: ExitPolicy,
    pending: Vec<WindowCommand>,
    closed: Vec<WindowId>,
    error: Option<crate::Error>,
    exit: bool,
    keys: Keys,
    /// Fenêtre sous le pointeur et position logique.
    pointer_window: Option<WindowId>,
    position: PointF,
    cursors: Cursors,
    /// Méthode d'entrée (IBus), si elle tourne.
    ime: Option<crate::platform::linux::ime::Ibus>,
    /// Composition en cours dans le champ qui a le focus.
    composition: Composition,
    /// XInput 2.1 (défilement fin), s'il est disponible.
    xinput: Option<xinput::XInput>,
    clipboard: Clipboard,
    /// Instant (serveur) de la dernière entrée : propriété de la sélection, conversions.
    time: u32,
    /// Événements reçus pendant une lecture du presse-papiers, traités ensuite, avant
    /// ceux lus après eux (l'ordre est conservé).
    deferred: VecDeque<*mut xcb_generic_event_t>,
    connection: Connection,
}
/// Boucle d'événements X11 ; erreurs documentées par `platform::run_windows`.
pub(super) fn run_windows(windows: Vec<WindowDescriptor>, policy: ExitPolicy) -> crate::Result<()> {
    crate::window::validation::validate(&windows, policy)?;
    let _guard = RunGuard::enter()?;
    let connection = Connection::connect()?;
    let mut keys = Keys::new();
    keys.set_keyboard(connection.keyboard(), false);
    let mut x = X11 {
        windows: Vec::new(),
        gpu: None,
        accessibility: crate::platform::linux::accessibility::Bridge::connect(),
        wake: Wake::new()?,
        executor: None,
        policy,
        pending: Vec::new(),
        closed: Vec::new(),
        error: None,
        exit: false,
        keys,
        pointer_window: None,
        position: PointF { x: 0., y: 0. },
        cursors: Cursors::new(&connection),
        xinput: xinput::XInput::new(&connection),
        ime: crate::platform::linux::ime::Ibus::connect(),
        composition: Composition::default(),
        clipboard: Clipboard::new(&connection),
        time: XCB_CURRENT_TIME,
        deferred: VecDeque::new(),
        connection,
    };
    for descriptor in windows {
        x.open(descriptor)?;
    }
    let result = x.run();
    for window in std::mem::take(&mut x.windows) {
        window.close(&x.connection);
    }
    x.cursors.close(&x.connection);
    x.clipboard.close(&x.connection);
    for event in std::mem::take(&mut x.deferred) {
        // SAFETY : événement possédé, libéré une fois.
        unsafe { libc::free(event.cast()) };
    }
    result?;
    match x.error.take() {
        Some(error) => Err(error),
        None => Ok(()),
    }
}
impl X11 {
    fn run(&mut self) -> crate::Result<()> {
        while self.service() {
            self.wait()?;
        }
        Ok(())
    }
    fn record(&mut self, error: crate::Error) {
        self.error.get_or_insert(error);
    }
    /// Exécute `f` sur la fenêtre `id` ; une erreur ou une panique de l'application est
    /// conservée et ferme la fenêtre.
    fn with<R>(
        &mut self,
        id: WindowId,
        f: impl FnOnce(&mut Window, &Connection) -> crate::Result<R>,
    ) -> Option<R> {
        let connection = &self.connection;
        let window = self.windows.iter_mut().find(|w| w.session.id == id)?;
        let error = match catch_unwind(AssertUnwindSafe(|| f(window, connection))) {
            Ok(Ok(value)) => return Some(value),
            Ok(Err(error)) => error,
            Err(payload) => crate::Error::Panicked {
                message: payload
                    .downcast_ref::<&str>()
                    .map(|s| s.to_string())
                    .or_else(|| payload.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "non-string panic payload".into()),
            },
        };
        self.record(error);
        self.closed.push(id);
        None
    }
    fn ids(&self) -> Vec<WindowId> {
        self.windows.iter().map(|w| w.session.id).collect()
    }
    fn open(&mut self, descriptor: WindowDescriptor) -> crate::Result<()> {
        if self.windows.iter().any(|w| w.session.id == descriptor.id) {
            return Err(crate::Error::DuplicateWindowId(descriptor.id));
        }
        let gpu = window::shared_gpu(&mut self.gpu, &descriptor.options);
        let window = Window::open(descriptor, &self.connection, gpu, self.wake.waker())?;
        if let Some(xinput) = &self.xinput {
            xinput.select(&self.connection, window.window);
        }
        self.windows.push(window);
        Ok(())
    }
    fn action(&mut self, id: WindowId, action: WindowAction) {
        let Some(window) = self.windows.iter().find(|w| w.session.id == id) else {
            return;
        };
        match action {
            WindowAction::Close => self.closed.push(id),
            WindowAction::Minimize => window.minimize(&self.connection),
            WindowAction::ToggleMaximize => {
                window.set_maximized(&self.connection, !window.session.maximized)
            }
        }
    }
    fn execute(&mut self, command: WindowCommand) {
        match command {
            WindowCommand::Open(descriptor) => {
                if let Err(error) = self.open(descriptor) {
                    self.record(error);
                }
            }
            WindowCommand::Close(id) => self.closed.push(id),
            WindowCommand::Action(id, action) => self.action(id, action),
            WindowCommand::Change(id, change) => {
                if let Some(reason) = crate::window::change::invalid_change(&change) {
                    self.record(crate::Error::invalid_window_options(reason));
                } else if let Some(window) = self.windows.iter_mut().find(|w| w.session.id == id) {
                    window.change(&self.connection, change);
                }
            }
            WindowCommand::FileDialog(id, task, dialog) => {
                if let Some(window) = self.windows.iter().find(|w| w.session.id == id) {
                    let parent = format!("x11:{:x}", window.window);
                    window.session.file_dialog(task, dialog, parent);
                }
            }
            WindowCommand::Exit => self.exit = true,
        }
    }
    /// Envoie `event` à la fenêtre `id` et applique sa réponse (presse-papiers, curseur).
    fn send(&mut self, id: WindowId, event: UiEvent) {
        let Some(mut result) = self.with(id, |w, _| w.session.input(event)) else {
            return;
        };
        match result.clipboard.take() {
            Some(crate::ui::text_input::ClipboardRequest::Write { text, cut }) => {
                self.clipboard.write(&self.connection, &text, self.time);
                if cut {
                    self.with(id, |w, _| {
                        w.session.input(UiEvent::Edit(
                            crate::ui::text_input::EditCommand::DeleteSelection,
                        ))
                    });
                }
            }
            Some(crate::ui::text_input::ClipboardRequest::Read) => {
                let text = self
                    .clipboard
                    .read(&self.connection, self.time, &mut self.deferred);
                if let Some(text) = text {
                    self.with(id, |w, _| w.session.input(UiEvent::Text(text.into())));
                }
            }
            None => {}
        }
        self.update_cursor();
    }
    /// Travail entre deux attentes ; `false` quand la boucle doit s'arrêter.
    fn service(&mut self) -> bool {
        for w in &mut self.windows {
            if let Some(error) = w.session.failure.take() {
                self.error.get_or_insert(error);
                self.closed.push(w.session.id);
            }
        }
        for id in std::mem::take(&mut self.closed) {
            if let Some(index) = self.windows.iter().position(|w| w.session.id == id) {
                // L'application est détruite avec sa fenêtre.
                let window = self.windows.remove(index);
                self.cursors.forget(window.window);
                window.close(&self.connection);
                if self.pointer_window == Some(id) {
                    self.pointer_window = None;
                }
            }
            if self.policy == ExitPolicy::MainWindowClosed(id) {
                self.exit = true;
            }
        }
        // Position à jour pour l'état remis à l'application (relue après un déplacement).
        for window in &mut self.windows {
            window.locate(&self.connection);
        }
        let now = Instant::now();
        for id in self.ids() {
            let executor = &mut self.executor;
            let commands = self
                .windows
                .iter_mut()
                .find(|w| w.session.id == id)
                .and_then(|window| {
                    let outcome = catch_unwind(AssertUnwindSafe(|| {
                        window.session.tasks(executor);
                        window.session.tick(now)?;
                        window.session.report();
                        Ok(window.session.take_window_commands())
                    }));
                    match outcome {
                        Ok(Ok(commands)) => Some(commands),
                        Ok(Err(error)) => {
                            self.error.get_or_insert(error);
                            self.closed.push(id);
                            None
                        }
                        Err(_) => {
                            self.error.get_or_insert(crate::Error::Panicked {
                                message: "application panicked".into(),
                            });
                            self.closed.push(id);
                            None
                        }
                    }
                });
            self.pending.extend(commands.unwrap_or_default());
        }
        while !self.pending.is_empty() {
            for command in std::mem::take(&mut self.pending) {
                self.execute(command);
            }
        }
        for id in self.ids() {
            self.with(id, |w, connection| {
                if w.session.dirty {
                    w.paint(connection)
                } else {
                    Ok(())
                }
            });
        }
        self.sync_ime();
        if let Some(bridge) = &mut self.accessibility {
            bridge.announce(&self.windows);
        }
        self.connection.flush();
        !(self.exit || self.error.is_some() || (self.windows.is_empty() && self.closed.is_empty()))
    }
    /// Délai d'attente : prochain réveil, ou aucun si une fenêtre peut être peinte.
    fn timeout(&self) -> i32 {
        if !self.closed.is_empty() || self.windows.iter().any(|w| w.session.dirty && w.ready()) {
            return 0;
        }
        let now = Instant::now();
        let wake = self
            .windows
            .iter()
            .filter(|w| w.ready())
            .filter_map(|w| w.session.next_wake())
            .min();
        match wake {
            Some(at) => at
                .saturating_duration_since(now)
                .as_millis()
                .saturating_add(1)
                .min(i32::MAX as u128) as i32,
            None => -1,
        }
    }
    /// Attend un événement du serveur, un réveil des tâches ou l'échéance, puis traite les
    /// événements reçus.
    fn wait(&mut self) -> crate::Result<()> {
        // Événements déjà lus (réponses attendues pendant le service).
        self.process();
        let xcb = self.connection.xcb;
        // SAFETY : connexion valide.
        if unsafe { (xcb.xcb_connection_has_error)(self.connection.raw) } != 0 {
            return Err(crate::Error::Platform {
                operation: "X11 connection",
                code: 0x8000_4005_u32 as i32,
            });
        }
        let mut fds = [
            libc::pollfd {
                // SAFETY : connexion valide.
                fd: unsafe { (xcb.xcb_get_file_descriptor)(self.connection.raw) },
                events: libc::POLLIN,
                revents: 0,
            },
            libc::pollfd {
                fd: self.wake.fd(),
                events: libc::POLLIN,
                revents: 0,
            },
            libc::pollfd {
                fd: self
                    .accessibility
                    .as_ref()
                    .and_then(|b| b.fd())
                    .unwrap_or(-1),
                events: libc::POLLIN,
                revents: 0,
            },
            libc::pollfd {
                fd: self.ime.as_ref().and_then(|i| i.fd()).unwrap_or(-1),
                events: libc::POLLIN,
                revents: 0,
            },
        ];
        // SAFETY : quatre structures valides pendant l'appel (un descripteur négatif est
        // ignoré par `poll`).
        unsafe { libc::poll(fds.as_mut_ptr(), 4, self.timeout()) };
        if fds[1].revents & libc::POLLIN != 0 {
            self.wake.clear();
        }
        self.process();
        self.ime_requests();
        if let Some(bridge) = &mut self.accessibility {
            bridge.pump(&mut self.windows);
        }
        Ok(())
    }
}
