//! Procédure de fenêtre : entrée des messages Win32, garde contre les paniques et envoi
//! des `UiEvent` au runtime avec les suites qu'il demande (capture, presse-papiers…).
pub(crate) mod caption;
mod clipboard;
mod dispatch;
mod keyboard;
mod nonclient;
mod pointer;
pub(crate) mod text;

use super::{
    accessibility,
    ffi::{Hwnd, invalidate},
    present::runtime,
    window::{self, state, tasks, timer},
};
use crate::ui::runtime::UiEvent;
use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use windows_sys::Win32::UI::{
    Input::KeyboardAndMouse::{ReleaseCapture, SetCapture},
    WindowsAndMessaging::{DefWindowProcW, DestroyWindow, WM_DESTROY, WM_NCDESTROY},
};

static POISONED: state::WindowLocal<Cell<bool>> = state::WindowLocal::new(|state| &state.poisoned);

pub(crate) unsafe extern "system" fn window_proc(
    window: Hwnd,
    message: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    let _scope = state::enter(window);
    if POISONED.with(Cell::get) {
        // Après une panique, l'état applicatif n'est plus fiable : seul Windows traite la fenêtre.
        if message == WM_NCDESTROY {
            state::remove(window);
            window::wake();
        }
        // SAFETY : relais des arguments reçus de Windows pour cette même fenêtre.
        return unsafe { DefWindowProcW(window, message, wparam, lparam) };
    }
    // Une panique ne doit pas traverser la frontière extern "system" : Rust arrêterait le
    // processus. Elle devient l'erreur renvoyée par la boucle de messages.
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY : arguments transmis tels quels par Windows pendant ce message.
        let result = unsafe { dispatch::window_message(window, message, wparam, lparam) };
        if message == WM_NCDESTROY {
            state::remove(window);
            window::wake();
        } else if message != WM_DESTROY {
            window::defer_commands();
            tasks::defer_commands();
        }
        result
    }));
    outcome.unwrap_or_else(|payload| {
        POISONED.with(|poisoned| poisoned.set(true));
        window::record_error(panic_error(payload.as_ref()));
        if message == WM_NCDESTROY {
            state::remove(window);
            window::wake();
        } else if message != WM_DESTROY {
            // SAFETY : la fenêtre est valide pendant le traitement de son message.
            unsafe { DestroyWindow(window) };
        }
        0
    })
}

fn panic_error(payload: &(dyn std::any::Any + Send)) -> crate::Error {
    let message = payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("non-string panic payload");
    crate::Error::Panicked {
        message: message.to_owned(),
    }
}

/// Envoie `event` au runtime puis applique sa réponse ; vrai si l'événement est traité.
fn send_ui(window: Hwnd, event: UiEvent) -> bool {
    let result = runtime::input(window, event);
    if result.capture {
        // SAFETY : capture demandée pour la fenêtre qui traite le message.
        unsafe { SetCapture(window) };
    }
    if result.release {
        // SAFETY : sans argument ; libère une capture détenue par ce thread.
        unsafe { ReleaseCapture() };
    }
    if result.repaint {
        invalidate(window);
    }
    if let Some(request) = result.clipboard {
        match clipboard::clipboard(window, request) {
            Ok(Some(event)) => {
                send_ui(window, event);
            }
            Ok(None) => {}
            Err(error) => eprintln!("karbon-ui: clipboard access failed: {error}"),
        }
    }
    text::sync(window);
    accessibility::sync(window);
    timer::sync(window);
    result.handled
}
