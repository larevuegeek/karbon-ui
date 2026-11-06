//! Boucle d'événements Win32 et fenêtres natives : ouverture, commandes différées,
//! erreur de la boucle et réveil du thread UI.
pub(crate) mod actions;
mod create;
pub(crate) mod dwm;
mod session;
pub(crate) mod shadow;
pub(crate) mod startup;
pub(crate) mod state;
pub(crate) mod tasks;
pub(crate) mod timer;

pub(crate) use session::Session;

use super::ffi::{hresult_from_win32, last_error};
use crate::window::{ExitPolicy, WindowCommand, WindowDescriptor};
use std::{
    cell::{Cell, RefCell},
    ptr::null_mut,
};
use windows_sys::Win32::{
    Foundation::ERROR_ACCESS_DENIED,
    UI::{
        HiDpi::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext},
        WindowsAndMessaging::{PostMessageW, WM_APP},
    },
};

thread_local! { static RUNNING: Cell<bool> = const { Cell::new(false) }; }
thread_local! { static PENDING: RefCell<Vec<WindowCommand>> = const { RefCell::new(Vec::new()) }; }
// Message privé de thread : des commandes de fenêtre attendent d'être traitées.
const WINDOW_WAKE: u32 = WM_APP + 1;

pub(crate) fn wake() {
    // NULL cible la file du thread courant, même si la dernière fenêtre vient de fermer.
    // SAFETY : message sans pointeur, déposé dans la file du thread courant.
    if unsafe { PostMessageW(null_mut(), WINDOW_WAKE, 0, 0) } == 0 {
        record_error(last_error("PostMessageW"));
    }
}
pub(crate) fn defer_commands() {
    let commands = super::present::runtime::take_window_commands();
    if !commands.is_empty() {
        PENDING.with(|pending| pending.borrow_mut().extend(commands));
        wake();
    }
}
fn take_pending() -> Vec<WindowCommand> {
    PENDING.with(|pending| std::mem::take(&mut *pending.borrow_mut()))
}

thread_local! { static LAST_ERROR: RefCell<Option<crate::Error>> = const { RefCell::new(None) }; }
/// Conserve la première erreur : les suivantes sont souvent des conséquences de la fermeture.
pub(crate) fn record_error(error: crate::Error) {
    LAST_ERROR.with(|slot| {
        slot.borrow_mut().get_or_insert(error);
    });
}
pub(crate) fn take_error() -> Option<crate::Error> {
    LAST_ERROR.with(|slot| slot.borrow_mut().take())
}

struct RunGuard;
impl RunGuard {
    fn enter() -> crate::Result<Self> {
        if RUNNING.with(|running| running.replace(true)) {
            return Err(crate::Error::AlreadyRunning);
        }
        Ok(Self)
    }
}
impl Drop for RunGuard {
    fn drop(&mut self) {
        RUNNING.with(|running| running.set(false));
    }
}

/// Boucle d'événements Win32 ; erreurs documentées par `platform::run_windows`.
pub(crate) fn run_windows(
    windows: Vec<WindowDescriptor>,
    exit_policy: ExitPolicy,
) -> crate::Result<()> {
    startup::mark("run");
    crate::window::validation::validate(&windows, exit_policy)?;
    let _running = RunGuard::enter()?;
    let _apartment = super::ffi::ComApartment::initialize()?;
    startup::mark("COM initialized");
    // SAFETY : réglage du processus par une constante prédéfinie de Windows.
    if unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) } == 0 {
        let error = last_error("SetProcessDpiAwarenessContext");
        // Déjà réglé (manifeste ou appel précédent) : la configuration existante convient.
        if !matches!(error, crate::Error::Platform { code, .. } if code == hresult_from_win32(ERROR_ACCESS_DENIED))
        {
            return Err(error);
        }
    }
    take_error();
    let session = Session::new()?;
    startup::mark("GDI+ initialized");
    for window in windows {
        session.open(window, true)?;
    }
    session.run_message_loop(exit_policy)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nested_run_is_rejected_and_guard_is_released() {
        let running = RunGuard::enter().unwrap();
        assert!(matches!(
            RunGuard::enter(),
            Err(crate::Error::AlreadyRunning)
        ));
        drop(running);
        assert!(RunGuard::enter().is_ok());
    }
}
