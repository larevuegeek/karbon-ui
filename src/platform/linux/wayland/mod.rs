//! Boucle d'événements Wayland : connexion au compositeur, fenêtres `xdg_toplevel`
//! présentées par Vulkan (ou dessinées par Cairo dans des tampons `wl_shm` en repli),
//! pointeur, clavier, méthode d'entrée, presse-papiers, minuteries et réveil des tâches de
//! fond.
//!
//! La boucle attend (`poll`) la connexion et un `eventfd` écrit par les tâches de fond,
//! jusqu'au prochain réveil demandé (runtime, répétition d'une touche). Les événements du
//! compositeur sont mis en file par un répartiteur unique (`display.rs`) puis traités ici,
//! sans emprunt tenu pendant un appel à libwayland.
mod clipboard;
mod display;
mod events;
mod input;
mod protocol;
mod seat;
mod shm;
#[cfg(test)]
mod tests;
mod text_input;
mod window;

use crate::platform::linux::shell::{RunGuard, Wake};
use crate::{
    platform::linux::ffi::wayland::{
        wl_display_cancel_read, wl_display_dispatch_pending, wl_display_flush, wl_display_get_fd,
        wl_display_prepare_read, wl_display_read_events,
    },
    task::TaskExecutor,
    ui::runtime::UiEvent,
    window::{ExitPolicy, WindowAction, WindowCommand, WindowDescriptor, WindowId},
};
use clipboard::Clipboard;
use display::Display;
use seat::Seat;
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    time::Instant,
};
use text_input::TextInput;
use window::Window;

struct Wayland {
    // Les fenêtres (donc les applications et les surfaces Vulkan) sont détruites avant la
    // connexion.
    windows: Vec<Window>,
    /// Contexte Vulkan partagé par les fenêtres (pas encore tenté, ou absent).
    gpu: Option<Option<std::rc::Rc<crate::platform::linux::vulkan::Gpu>>>,
    /// Pont AT-SPI (absent sans bus d'accessibilité).
    accessibility: Option<crate::platform::linux::accessibility::Bridge>,
    text_input: TextInput,
    seat: Seat,
    clipboard: Clipboard,
    display: Display,
    wake: Wake,
    executor: Option<TaskExecutor>,
    policy: ExitPolicy,
    pending: Vec<WindowCommand>,
    closed: Vec<WindowId>,
    error: Option<crate::Error>,
    exit: bool,
}
/// Boucle d'événements Wayland ; erreurs documentées par `platform::run_windows`.
pub(super) fn run_windows(windows: Vec<WindowDescriptor>, policy: ExitPolicy) -> crate::Result<()> {
    crate::window::validation::validate(&windows, policy)?;
    let _guard = RunGuard::enter()?;
    let display = Display::connect()?;
    let seat = Seat::new(
        display
            .globals
            .seat
            .map_or(std::ptr::null_mut(), |(_, s)| s),
    );
    let clipboard = Clipboard::new(display.globals.data_manager, seat.seat);
    let text_input = TextInput::new(display.globals.text_input, seat.seat);
    let mut wl = Wayland {
        windows: Vec::new(),
        gpu: None,
        accessibility: crate::platform::linux::accessibility::Bridge::connect(),
        text_input,
        seat,
        clipboard,
        display,
        wake: Wake::new()?,
        executor: None,
        policy,
        pending: Vec::new(),
        closed: Vec::new(),
        error: None,
        exit: false,
    };
    // Capacités du siège et échelles des sorties, avant la première fenêtre.
    wl.display.roundtrip()?;
    wl.process();
    for descriptor in windows {
        wl.open(descriptor)?;
    }
    let result = wl.run();
    wl.windows.clear();
    wl.text_input.close();
    wl.seat.close();
    result?;
    match wl.error.take() {
        Some(error) => Err(error),
        None => Ok(()),
    }
}
impl Wayland {
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
        f: impl FnOnce(&mut Window) -> crate::Result<R>,
    ) -> Option<R> {
        let window = self.windows.iter_mut().find(|w| w.session.id == id)?;
        let error = match catch_unwind(AssertUnwindSafe(|| f(window))) {
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
        let window = Window::open(
            descriptor,
            &self.display.globals,
            (self.display.raw, gpu),
            self.wake.waker(),
        )?;
        self.windows.push(window);
        Ok(())
    }
    fn action(&mut self, id: WindowId, action: WindowAction) {
        let Some(window) = self.windows.iter().find(|w| w.session.id == id) else {
            return;
        };
        match action {
            WindowAction::Close => self.closed.push(id),
            WindowAction::Minimize => window.minimize(),
            WindowAction::ToggleMaximize => window.set_maximized(!window.session.maximized),
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
            WindowCommand::Exit => self.exit = true,
        }
    }
    /// Envoie `event` à la fenêtre `id` et applique sa réponse (presse-papiers, curseur).
    fn send(&mut self, id: WindowId, event: UiEvent) {
        let Some(mut result) = self.with(id, |w| w.session.input(event)) else {
            return;
        };
        match result.clipboard.take() {
            Some(crate::ui::text_input::ClipboardRequest::Write { text, cut }) => {
                let manager = self.display.globals.data_manager;
                self.clipboard.write(manager, self.seat.serial, &text);
                if cut {
                    self.with(id, |w| {
                        w.session.input(UiEvent::Edit(
                            crate::ui::text_input::EditCommand::DeleteSelection,
                        ))
                    });
                }
            }
            Some(crate::ui::text_input::ClipboardRequest::Read) => {
                if let Some(text) = self.clipboard.read(self.display.raw) {
                    self.with(id, |w| w.session.input(UiEvent::Text(text.into())));
                }
                // Les événements reçus pendant la lecture attendent leur traitement.
                self.process();
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
                drop(self.windows.remove(index));
                if self.seat.pointer_window == Some(id) {
                    self.seat.pointer_window = None;
                }
                if self.seat.keyboard_window == Some(id) {
                    self.seat.keyboard_window = None;
                    self.seat.keys.reset();
                }
            }
            if self.policy == ExitPolicy::MainWindowClosed(id) {
                self.exit = true;
            }
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
        if let Some(id) = self.seat.keyboard_window {
            for event in self.seat.keys.repeat(now) {
                self.send(id, event);
            }
        }
        while !self.pending.is_empty() {
            for command in std::mem::take(&mut self.pending) {
                self.execute(command);
            }
        }
        let (compositor, shm) = (self.display.globals.compositor, self.display.globals.shm);
        for id in self.ids() {
            self.with(id, |w| {
                if w.session.dirty {
                    w.paint(compositor, shm)
                } else {
                    Ok(())
                }
            });
        }
        self.sync_text_input();
        if let Some(bridge) = &mut self.accessibility {
            bridge.announce(&self.windows);
        }
        self.update_cursor();
        !(self.exit || self.error.is_some() || (self.windows.is_empty() && self.closed.is_empty()))
    }
    /// Délai d'attente : prochain réveil, ou aucun si une fenêtre peut être peinte.
    fn timeout(&self) -> i32 {
        if !self.closed.is_empty() || self.windows.iter().any(|w| w.session.dirty && w.ready()) {
            return 0;
        }
        let now = Instant::now();
        // Une fenêtre qui attend l'affichage de son image (ou un tampon) est reprise par
        // l'événement du compositeur : ses animations n'ont pas à réveiller la boucle
        // (fenêtre cachée ou réduite : aucune image).
        let wake = self
            .windows
            .iter()
            .filter(|w| w.ready())
            .filter_map(|w| w.session.next_wake())
            .chain(self.seat.keys.next_repeat())
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
    /// Attend un événement du compositeur, un réveil des tâches ou l'échéance, puis traite
    /// les événements reçus.
    fn wait(&mut self) -> crate::Result<()> {
        let raw = self.display.raw;
        // SAFETY : connexion valide pendant toute la boucle.
        let flushed = unsafe {
            while wl_display_prepare_read(raw) != 0 {
                if wl_display_dispatch_pending(raw) < 0 {
                    return Err(self.display.error("wl_display_dispatch_pending"));
                }
                self.process();
            }
            wl_display_flush(raw)
        };
        // Tampon d'envoi plein (EAGAIN) : des requêtes attendent encore ; la boucle se
        // réveille aussi quand la connexion redevient inscriptible, pour les envoyer au
        // prochain tour plutôt qu'au prochain événement du compositeur.
        let writable =
            flushed < 0 && std::io::Error::last_os_error().raw_os_error() == Some(libc::EAGAIN);
        let mut fds = [
            libc::pollfd {
                // SAFETY : connexion valide.
                fd: unsafe { wl_display_get_fd(raw) },
                events: libc::POLLIN | if writable { libc::POLLOUT } else { 0 },
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
        ];
        // SAFETY : trois structures valides pendant l'appel (un descripteur négatif est
        // ignoré par `poll`).
        let ready = unsafe { libc::poll(fds.as_mut_ptr(), 3, self.timeout()) };
        // SAFETY : lecture préparée ci-dessus, terminée ou annulée une fois.
        unsafe {
            if ready > 0 && fds[0].revents & libc::POLLIN != 0 {
                if wl_display_read_events(raw) < 0 {
                    return Err(self.display.error("wl_display_read_events"));
                }
            } else {
                wl_display_cancel_read(raw);
            }
            if fds[0].revents & (libc::POLLERR | libc::POLLHUP) != 0 {
                return Err(self.display.error("wayland connection"));
            }
            if wl_display_dispatch_pending(raw) < 0 {
                return Err(self.display.error("wl_display_dispatch_pending"));
            }
        }
        if fds[1].revents & libc::POLLIN != 0 {
            self.wake.clear();
        }
        self.process();
        if let Some(bridge) = &mut self.accessibility {
            // Appels des lecteurs d'écran (lus sans attendre, même sans signal du poll).
            bridge.pump(&mut self.windows);
        }
        Ok(())
    }
}
