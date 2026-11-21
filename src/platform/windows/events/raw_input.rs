//! Entrées brutes sous Windows (`ui::runtime::raw_input`) : les trois boutons de la
//! souris pour les gestionnaires `on_pointer`, toutes les touches pour
//! `Application::key`. Elles s'ajoutent aux événements ordinaires sans les remplacer :
//! chaque message garde le traitement qu'il avait.
use super::{
    super::{
        ffi::{Hwnd, invalidate, scale},
        present::runtime,
    },
    keyboard::is_pressed,
    pointer::point_from_message,
};
use crate::ui::{
    runtime::{KeyCode, KeyPress, PointerButton, PointerPhase, raw_input::PointerInput},
    scene::PointF,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetCapture, ReleaseCapture, SetCapture, VK_BACK, VK_CONTROL, VK_DELETE, VK_DOWN, VK_END,
    VK_ESCAPE, VK_F1, VK_F12, VK_HOME, VK_INSERT, VK_LEFT, VK_LMENU, VK_MENU, VK_NEXT, VK_PRIOR,
    VK_RETURN, VK_RIGHT, VK_SHIFT, VK_SPACE, VK_TAB, VK_UP,
};

/// Bit 30 de lparam : la touche était déjà enfoncée (répétition).
const REPEATED: isize = 1 << 30;

/// Donne aux gestionnaires `on_pointer` un appui (`Down`), un mouvement ou un
/// relâchement à la position de `lparam`. Pendant le geste d'un élément, la fenêtre garde
/// la souris, qu'elle rend au dernier bouton relâché si rien d'autre ne la tient.
pub(super) fn pointer(
    window: Hwnd,
    lparam: isize,
    phase: PointerPhase,
    button: Option<PointerButton>,
) {
    let p = point_from_message(lparam);
    let factor = scale(window);
    let input = PointerInput {
        phase,
        button,
        position: PointF {
            x: p.x as f32 / factor,
            y: p.y as f32 / factor,
        },
        shift: is_pressed(VK_SHIFT),
        control: is_pressed(VK_CONTROL),
        alt: is_pressed(VK_MENU),
    };
    let moving = phase == PointerPhase::Move;
    let result = runtime::raw_input(window, moving, |ui| ui.pointer_input(input));
    if result.repaint {
        invalidate(window);
    }
    // SAFETY : sans argument, ou la fenêtre qui traite le message ; capture prise et
    // rendue sur le thread de la fenêtre.
    unsafe {
        let held = GetCapture() == window;
        if runtime::pointer_gesture() && !held {
            SetCapture(window);
        } else if !moving && held && !runtime::pointer_gesture() && !runtime::has_capture() {
            ReleaseCapture();
        }
    }
}

/// WM_RBUTTONDOWN, WM_RBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP : le bouton pour
/// `on_pointer`, puis ce que Windows en fait d'habitude (le bouton droit relâché amène
/// le menu contextuel).
pub(super) fn side_button(
    window: Hwnd,
    lparam: isize,
    (button, down): (PointerButton, bool),
    default_proc: impl FnOnce() -> isize,
) -> isize {
    let phase = if down {
        PointerPhase::Down
    } else {
        PointerPhase::Up
    };
    pointer(window, lparam, phase, Some(button));
    default_proc()
}

/// WM_KEYDOWN et WM_KEYUP : la touche pour `Application::key`, avant son traitement
/// ordinaire, qui suit sans changement.
pub(super) fn key(window: Hwnd, wparam: usize, lparam: isize, pressed: bool) {
    let Some(code) = u16::try_from(wparam).ok().and_then(key_code) else {
        return;
    };
    let press = KeyPress {
        repeat: pressed && lparam & REPEATED != 0,
        shift: is_pressed(VK_SHIFT),
        control: is_pressed(VK_CONTROL),
        alt: is_pressed(VK_MENU) || is_pressed(VK_LMENU),
        ..KeyPress::new(code, pressed)
    };
    let result = runtime::raw_input(window, false, |ui| ui.key_input(press));
    if result.repaint {
        invalidate(window);
    }
}

/// La touche de code virtuel `virtual_key`, par sa place sur un clavier américain.
fn key_code(virtual_key: u16) -> Option<KeyCode> {
    use KeyCode::*;
    const LETTERS: [KeyCode; 26] = [
        A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    ];
    const DIGITS: [KeyCode; 10] = [
        Digit0, Digit1, Digit2, Digit3, Digit4, Digit5, Digit6, Digit7, Digit8, Digit9,
    ];
    const FUNCTIONS: [KeyCode; 12] = [F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12];
    // Lettres et chiffres : leurs codes ASCII en capitales.
    let at = |table: &[KeyCode], first: u16| table.get(usize::from(virtual_key - first)).copied();
    Some(match virtual_key {
        0x41..=0x5A => at(&LETTERS, 0x41)?,
        0x30..=0x39 => at(&DIGITS, 0x30)?,
        VK_F1..=VK_F12 => at(&FUNCTIONS, VK_F1)?,
        VK_ESCAPE => Escape,
        VK_RETURN => Enter,
        VK_TAB => Tab,
        VK_SPACE => Space,
        VK_BACK => Backspace,
        VK_DELETE => Delete,
        VK_INSERT => Insert,
        VK_HOME => Home,
        VK_END => End,
        VK_PRIOR => PageUp,
        VK_NEXT => PageDown,
        VK_LEFT => Left,
        VK_RIGHT => Right,
        VK_UP => Up,
        VK_DOWN => Down,
        VK_SHIFT => Shift,
        VK_CONTROL => Control,
        VK_MENU => Alt,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_named_by_their_place() {
        assert_eq!(key_code(0x41), Some(KeyCode::A));
        assert_eq!(key_code(0x5A), Some(KeyCode::Z));
        assert_eq!(key_code(0x30), Some(KeyCode::Digit0));
        assert_eq!(key_code(VK_F12), Some(KeyCode::F12));
        assert_eq!(key_code(VK_ESCAPE), Some(KeyCode::Escape));
        // Une touche sans nom (le pavé numérique, par exemple) n'est pas donnée.
        assert_eq!(key_code(0x60), None);
    }
}
