pub(super) use super::super::{
    accessibility,
    ffi::{Hwnd, invalidate},
    present::runtime,
    window::{self, shadow, state, tasks, timer, tray},
};
use super::*;
pub(super) use crate::ui::runtime::UiEvent;
pub(super) use std::cell::Cell;
pub(super) use std::panic::{AssertUnwindSafe, catch_unwind};
pub(super) use windows_sys::Win32::UI::{
    Input::KeyboardAndMouse::{ReleaseCapture, SetCapture},
    WindowsAndMessaging::{DefWindowProcW, DestroyWindow, WM_DESTROY, WM_NCDESTROY},
};

pub(super) static POISONED: state::WindowLocal<Cell<bool>> =
    state::WindowLocal::new(|state| &state.poisoned);

pub(crate) unsafe extern "system" fn window_proc(
    window: Hwnd,
    message: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    let _scope = state::enter(window);
    if POISONED.with(Cell::get) {
        // Après une panique, l'état applicatif n'est plus fiable : seul Windows traite la fenêtre.
        if message == WM_DESTROY {
            teardown_guarded(window);
        } else if message == WM_NCDESTROY {
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
            window::defer_commands(window);
            tasks::defer_commands(window);
        }
        result
    }));
    outcome.unwrap_or_else(|payload| {
        POISONED.with(|poisoned| poisoned.set(true));
        window::record_error(panic_error(payload.as_ref()));
        if message == WM_NCDESTROY {
            state::remove(window);
            window::wake();
        } else if message == WM_DESTROY {
            // Panique pendant le démontage : ses étapes restantes, chacune une seule fois.
            teardown_guarded(window);
        } else {
            // SAFETY : la fenêtre est valide pendant le traitement de son message.
            unsafe { DestroyWindow(window) };
        }
        0
    })
}

/// Démontage de `WM_DESTROY` : ce que la fenêtre possède hors de son état et que Windows
/// ne libère pas avec elle (ombre sans propriétaire, fournisseur d'accessibilité, tâches).
/// `run` exécute chaque étape ; toutes sont sans effet la seconde fois.
pub(super) fn teardown(window: Hwnd, run: impl Fn(&dyn Fn())) {
    run(&runtime::finish_profile);
    run(&tasks::close);
    run(&|| tray::remove(window));
    run(&|| timer::shutdown(window));
    run(&|| text::shutdown(window));
    run(&accessibility::shutdown);
    run(&shadow::destroy);
}

/// Le même démontage après une panique : le message interrompu peut encore emprunter un
/// état, donc chaque étape est isolée pour que son échec n'empêche pas les suivantes.
fn teardown_guarded(window: Hwnd) {
    teardown(window, |step| {
        let _ = catch_unwind(AssertUnwindSafe(step));
    });
}

/// Appel de l'application hors de toute procédure de fenêtre (chargement) : une panique
/// devient `Error::Panicked`, rendu à l'appelant au lieu de sortir de `platform::run`.
pub(crate) fn catch_panic<R>(f: impl FnOnce() -> R) -> crate::Result<R> {
    catch_unwind(AssertUnwindSafe(f)).map_err(|payload| panic_error(payload.as_ref()))
}

/// Appel de l'application pour `window` depuis la boucle de messages, hors de
/// `window_proc` (commandes de fenêtre, tâches, ouverture) : une panique a la même suite
/// que dans la procédure. La fenêtre est empoisonnée puis détruite sans rappeler
/// l'application, et l'erreur `Error::Panicked` est rendue à l'appelant, qui la remonte
/// à la boucle.
pub(crate) fn guard<R>(window: Hwnd, f: impl FnOnce() -> R) -> crate::Result<R> {
    let _scope = state::enter(window);
    catch_panic(f).inspect_err(|_| {
        // Fenêtre déjà détruite pendant l'appel : rien à empoisonner ni à détruire.
        if state::windows().iter().any(|(handle, _)| *handle == window) {
            POISONED.with(|poisoned| poisoned.set(true));
            // SAFETY : fenêtre enregistrée, donc vivante sur ce thread.
            unsafe { DestroyWindow(window) };
        }
    })
}

pub(super) fn panic_error(payload: &(dyn std::any::Any + Send)) -> crate::Error {
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
pub(super) fn send_ui(window: Hwnd, event: UiEvent) -> bool {
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
