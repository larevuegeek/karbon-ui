//! Actions de fenêtre demandées par l'application ou la barre de titre.
use super::super::ffi::{Hwnd, last_error};
use crate::window::WindowAction;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DestroyWindow, IsZoomed, SW_MAXIMIZE, SW_MINIMIZE, SW_RESTORE, ShowWindow,
};

pub(crate) fn execute_action(window: Hwnd, action: WindowAction) {
    match action {
        WindowAction::Minimize => {
            // SAFETY : commande d'affichage sur un handle ; Windows valide la fenêtre.
            unsafe { ShowWindow(window, SW_MINIMIZE) };
        }
        WindowAction::ToggleMaximize => {
            // SAFETY : lecture d'état puis commande d'affichage, sans pointeur transmis.
            let command = if unsafe { IsZoomed(window) } != 0 {
                SW_RESTORE
            } else {
                SW_MAXIMIZE
            };
            // SAFETY : voir ci-dessus.
            unsafe { ShowWindow(window, command) };
        }
        WindowAction::Close => {
            // SAFETY : destruction demandée sur le thread propriétaire de la fenêtre.
            if unsafe { DestroyWindow(window) } == 0 {
                eprintln!(
                    "karbon-ui: closing the window failed: {}",
                    last_error("DestroyWindow")
                );
            }
        }
    }
}
