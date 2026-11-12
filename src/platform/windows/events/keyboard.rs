//! Clavier : touches et raccourcis d'édition traduits en `UiEvent` ; avec l'option
//! `profile` seulement, touches de diagnostic (F2 renderer, F3 transparence, F5 thème,
//! F8 statistiques).
use super::{
    super::{
        ffi::{Hwnd, invalidate},
        present::{self, runtime},
        window::{dwm::apply_style, shadow},
    },
    send_ui, state,
};
use crate::ui::{
    runtime::{Key, UiEvent},
    text_input::EditCommand,
};
use windows_sys::Win32::UI::{
    Input::KeyboardAndMouse::{
        GetKeyState, VK_A, VK_BACK, VK_C, VK_CONTROL, VK_DELETE, VK_DOWN, VK_END, VK_ESCAPE, VK_F2,
        VK_F3, VK_F5, VK_F8, VK_HOME, VK_LEFT, VK_MENU, VK_NEXT, VK_PRIOR, VK_RETURN, VK_RIGHT,
        VK_SHIFT, VK_SPACE, VK_TAB, VK_UP, VK_V, VK_X, VK_Y, VK_Z,
    },
    WindowsAndMessaging::KF_REPEAT,
};

// Bit 30 de lparam (KF_REPEAT dans le mot haut) : la touche était déjà enfoncée.
const REPEATED: isize = (KF_REPEAT as isize) << 16;

/// Code de touche virtuelle porté par wparam, s'il tient sur 16 bits.
fn virtual_key(wparam: usize) -> Option<u16> {
    u16::try_from(wparam).ok()
}
fn key(wparam: usize) -> Option<Key> {
    Some(match virtual_key(wparam)? {
        VK_TAB => Key::Tab,
        VK_RETURN => Key::Enter,
        VK_SPACE => Key::Space,
        VK_LEFT => Key::Left,
        VK_UP => Key::Up,
        VK_RIGHT => Key::Right,
        VK_DOWN => Key::Down,
        VK_HOME => Key::Home,
        VK_END => Key::End,
        VK_PRIOR => Key::PageUp,
        VK_NEXT => Key::PageDown,
        VK_ESCAPE => Key::Escape,
        VK_BACK => Key::Backspace,
        VK_DELETE => Key::Delete,
        _ => return None,
    })
}
/// État d'une touche de modification au moment du message.
pub(super) fn is_pressed(key: u16) -> bool {
    // SAFETY : simple lecture de l'état clavier du thread courant.
    unsafe { GetKeyState(i32::from(key)) < 0 }
}

/// WM_KEYDOWN.
pub(super) fn key_down(
    window: Hwnd,
    wparam: usize,
    lparam: isize,
    default_proc: impl FnOnce() -> isize,
) -> isize {
    // Touches de diagnostic : jamais dans une application ordinaire, où F2, F3, F5 et F8
    // appartiennent à l'application ou à Windows.
    let first = lparam & REPEATED == 0 && state::options().profile;
    match virtual_key(wparam) {
        Some(VK_F2) if first => {
            // F2, ignorer la répétition.
            if let Err(error) = present::switch_renderer() {
                eprintln!("karbon-ui: switching the renderer failed: {error}");
            }
            invalidate(window);
            0
        }
        Some(VK_F3) if first => {
            // F3 : fond transparent.
            state::OPTIONS.with(|slot| {
                let mut value = slot.get();
                value.transparent = !value.transparent;
                slot.set(value);
            });
            if state::options().transparent {
                shadow::destroy();
            } else {
                apply_style(window);
                shadow::update(window);
            }
            invalidate(window);
            0
        }
        Some(VK_F5) if first => {
            // F5 : thème.
            state::OPTIONS.with(|slot| {
                let mut value = slot.get();
                value.light = !value.light;
                slot.set(value);
            });
            invalidate(window);
            0
        }
        Some(VK_F8) if first => {
            runtime::print_stats();
            0
        }
        _ => key_command(window, wparam, lparam, default_proc),
    }
}

/// Raccourcis d'édition (Ctrl) puis touches du runtime ; Windows traite le reste.
fn key_command(
    window: Hwnd,
    wparam: usize,
    lparam: isize,
    default_proc: impl FnOnce() -> isize,
) -> isize {
    if is_pressed(VK_CONTROL) && !is_pressed(VK_MENU) {
        let extend = is_pressed(VK_SHIFT);
        let command = match virtual_key(wparam) {
            Some(VK_A) => Some(EditCommand::SelectAll),
            Some(VK_C) => Some(EditCommand::Copy),
            Some(VK_X) => Some(EditCommand::Cut),
            Some(VK_V) => Some(EditCommand::Paste),
            Some(VK_Z) => Some(if extend {
                EditCommand::Redo
            } else {
                EditCommand::Undo
            }),
            Some(VK_Y) => Some(EditCommand::Redo),
            Some(VK_LEFT) => Some(EditCommand::WordLeft { extend }),
            Some(VK_RIGHT) => Some(EditCommand::WordRight { extend }),
            _ => None,
        };
        if let Some(command) = command
            && send_ui(window, UiEvent::Edit(command))
        {
            return 0;
        }
    }
    if let Some(key) = key(wparam) {
        let shift = is_pressed(VK_SHIFT);
        if send_ui(
            window,
            UiEvent::KeyDown {
                key,
                shift,
                repeat: lparam & REPEATED != 0,
            },
        ) {
            return 0;
        }
    }
    default_proc()
}

/// WM_KEYUP.
pub(super) fn key_up(window: Hwnd, wparam: usize, default_proc: impl FnOnce() -> isize) -> isize {
    if let Some(key) = key(wparam)
        && send_ui(window, UiEvent::KeyUp(key))
    {
        return 0;
    }
    default_proc()
}
