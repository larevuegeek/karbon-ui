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
    scancode,
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
// Ajout : la touche Windows, pour `meta`.
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{VK_LWIN, VK_RWIN};

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
    // Pointeur verrouillé : la place de l'appui et le mouvement propre de la souris.
    let (p, motion) = if phase == PointerPhase::Move {
        match super::pointer_lock::filter_move(window, p) {
            Some(filtered) => filtered,
            None => return,
        }
    } else {
        (p, None)
    };
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
        meta: is_pressed(VK_LWIN) || is_pressed(VK_RWIN),
        motion: motion.map(|(x, y)| (x as f32 / factor, y as f32 / factor)),
    };
    let moving = phase == PointerPhase::Move;
    // Un geste qui a son curseur (`Element::cursor_shape`) : Windows n'envoie pas
    // `WM_SETCURSOR` pendant la capture, le curseur est remis ici.
    let shaped = || runtime::pointer_gesture() && runtime::cursor_shape().is_some();
    let shaped_before = shaped();
    // Le stylet, s'il fait ce message, et la place de la fenêtre sur l'écran (`pen.rs`).
    let (pen, origin, ratio) = super::pen::extras(window);
    let result = runtime::raw_input(window, moving, |ui| {
        ui.set_pointer_extras(pen, origin, ratio);
        ui.pointer_input(input)
    });
    if result.repaint {
        invalidate(window);
    }
    if shaped_before || shaped() {
        super::pointer::update_cursor();
    }
    super::pointer_lock::sync(window, p);
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
    // Un glisser de fichiers qui vient de quitter la fenêtre passe au système.
    if moving {
        super::drag_out::after_pointer(window);
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

/// WM_XBUTTONDOWN, WM_XBUTTONUP : les boutons 4 (précédent) et 5 (suivant) de la souris
/// pour `on_pointer`, puis ce que Windows en fait d'habitude (`WM_APPCOMMAND`).
pub(super) fn extra_button(
    window: Hwnd,
    (wparam, lparam): (usize, isize),
    down: bool,
    default_proc: impl FnOnce() -> isize,
) -> isize {
    use windows_sys::Win32::UI::WindowsAndMessaging::XBUTTON1;
    let button = match ((wparam >> 16) & 0xFFFF) as u16 {
        XBUTTON1 => PointerButton::Back,
        _ => PointerButton::Forward,
    };
    side_button(window, lparam, (button, down), default_proc)
}

/// WM_MOUSEWHEEL et WM_MOUSEHWHEEL : le tour de molette, en crans, pour les gestionnaires
/// `on_pointer_wheel` sous `client` (pixels du client), avec Maj, Ctrl et Alt. `true`
/// quand l'un d'eux l'a pris : la molette ne fait alors rien d'autre.
pub(super) fn wheel(
    window: Hwnd,
    (message, wparam): (u32, usize),
    client: super::super::ffi::Point,
) -> bool {
    use windows_sys::Win32::{
        System::SystemServices::{MK_CONTROL, MK_SHIFT},
        UI::WindowsAndMessaging::WM_MOUSEHWHEEL,
    };
    let notches = f32::from((wparam >> 16) as i16) / 120.;
    // Vers soi (cran négatif de Windows) et vers la droite sont positifs.
    let amount = if message == WM_MOUSEHWHEEL {
        (notches, 0.)
    } else {
        (0., -notches)
    };
    let factor = scale(window);
    let input = PointerInput {
        phase: PointerPhase::Wheel,
        button: None,
        position: PointF {
            x: client.x as f32 / factor,
            y: client.y as f32 / factor,
        },
        shift: wparam & MK_SHIFT as usize != 0,
        control: wparam & MK_CONTROL as usize != 0,
        alt: is_pressed(VK_MENU),
        meta: is_pressed(VK_LWIN) || is_pressed(VK_RWIN),
        motion: None,
    };
    let (_, origin, ratio) = super::pen::extras(window);
    let result = runtime::raw_input(window, false, |ui| {
        ui.set_pointer_extras(None, origin, ratio);
        ui.pointer_wheel(input, amount)
    });
    if result.repaint {
        invalidate(window);
    }
    result.handled
}

/// WM_KEYDOWN et WM_KEYUP : la touche pour `Application::key`, avant son traitement
/// ordinaire, qui suit sans changement.
pub(super) fn key(window: Hwnd, wparam: usize, lparam: isize, pressed: bool) {
    // Un relâchement arrive sans lparam par `keyboard::key_up` : celui du message est
    // déposé par la table de routage.
    let lparam = if pressed || lparam != 0 {
        lparam
    } else {
        scancode::take_release()
    };
    let Some(code) = u16::try_from(wparam)
        .ok()
        .and_then(|vk| key_code(vk).or_else(|| super::more_keys::added_key_code(vk)))
    else {
        return;
    };
    let scan = scancode::scancode(wparam as u16, lparam);
    let press = KeyPress {
        repeat: pressed && lparam & REPEATED != 0,
        shift: is_pressed(VK_SHIFT),
        control: is_pressed(VK_CONTROL),
        alt: is_pressed(VK_MENU) || is_pressed(VK_LMENU),
        physical: scancode::physical_key(scan),
        scancode: scan,
        meta: is_pressed(VK_LWIN) || is_pressed(VK_RWIN),
        ..KeyPress::new(code, pressed)
    };
    let result = runtime::raw_input(window, false, |ui| ui.key_input(press));
    if result.repaint {
        invalidate(window);
    }
}

/// WM_SYSKEYDOWN et WM_SYSKEYUP (touches tenues avec Alt, et F10) : la touche pour
/// `Application::key`, puis le traitement de Windows (Alt+F4 ferme la fenêtre). Alt seul
/// et F10 n'ouvrent plus le menu de la fenêtre (`is_bare_key_menu`).
pub(super) fn system_key(
    window: Hwnd,
    (wparam, lparam): (usize, isize),
    pressed: bool,
    default_proc: impl FnOnce() -> isize,
) -> isize {
    key(window, wparam, lparam, pressed);
    default_proc()
}

/// WM_SYSCOMMAND `wparam`, `lparam` est-il le menu de la fenêtre demandé au clavier sans
/// lettre (Alt seul ou F10 relâché) ? Alt+Espace le demande avec l'espace : non.
pub(super) fn is_bare_key_menu(wparam: usize, lparam: isize) -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::SC_KEYMENU;
    (wparam & 0xFFF0) as u32 == SC_KEYMENU && lparam == 0
}

/// La touche de code virtuel `virtual_key`. Le code virtuel suit la disposition : sur un
/// clavier français, la touche marquée Z donne `VK_Z`, donc `KeyCode::Z`.
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
    fn letters_follow_the_layout() {
        assert_eq!(key_code(0x41), Some(KeyCode::A));
        assert_eq!(key_code(0x5A), Some(KeyCode::Z));
        assert_eq!(key_code(0x30), Some(KeyCode::Digit0));
        assert_eq!(key_code(VK_F12), Some(KeyCode::F12));
        assert_eq!(key_code(VK_ESCAPE), Some(KeyCode::Escape));
        // Une touche sans nom (le pavé numérique, par exemple) n'est pas donnée.
        assert_eq!(key_code(0x60), None);
    }
    /// Disposition française (AZERTY) chargée dans un fil d'essai, sans l'activer : la
    /// touche à la place du W américain (code de balayage 0x11) y donne le code virtuel
    /// `VK_Z`, donc `KeyCode::Z`, et sa place reste `PhysicalKey::KeyW`.
    #[test]
    fn on_a_french_layout_the_key_marked_z_gives_z_at_the_place_of_w() {
        std::thread::spawn(|| {
            use crate::ui::runtime::PhysicalKey;
            use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
                GetKeyboardLayoutList, HKL, KLF_NOTELLSHELL, LoadKeyboardLayoutW, MAPVK_VK_TO_VSC,
                MAPVK_VSC_TO_VK, MapVirtualKeyExW, UnloadKeyboardLayout, VK_A, VK_Q, VK_W, VK_Z,
            };
            let loaded = || {
                let mut list: [HKL; 64] = [std::ptr::null_mut(); 64];
                // SAFETY : tampon local de 64 poignées, sa taille est passée.
                let count = unsafe { GetKeyboardLayoutList(64, list.as_mut_ptr()) };
                list[..count.max(0) as usize].to_vec()
            };
            let before = loaded();
            let name: Vec<u16> = "0000040C\0".encode_utf16().collect();
            // SAFETY : nom terminé par un zéro ; la disposition n'est pas activée
            // (ni KLF_ACTIVATE ni KLF_SETFORPROCESS), le bureau n'en est pas prévenu.
            let french = unsafe { LoadKeyboardLayoutW(name.as_ptr(), KLF_NOTELLSHELL) };
            assert!(!french.is_null(), "French layout unavailable");
            // SAFETY : traductions par une disposition chargée, sans pointeur.
            let (to_vk, to_scan) = unsafe {
                (
                    |code: u32| MapVirtualKeyExW(code, MAPVK_VSC_TO_VK, french) as u16,
                    |vk: u16| MapVirtualKeyExW(u32::from(vk), MAPVK_VK_TO_VSC, french),
                )
            };
            let checks = [
                (to_vk(0x11), VK_Z, KeyCode::Z),
                (to_vk(0x2C), VK_W, KeyCode::W),
                (to_vk(0x10), VK_A, KeyCode::A),
                (to_vk(0x1E), VK_Q, KeyCode::Q),
            ];
            let places = [to_scan(VK_Z), to_scan(VK_W), to_scan(VK_A), to_scan(VK_Q)];
            // La disposition n'est rendue que si l'essai l'a ajoutée.
            if !before.contains(&french) {
                // SAFETY : poignée rendue par LoadKeyboardLayoutW ci-dessus.
                unsafe { UnloadKeyboardLayout(french) };
            }
            for (vk, expected, key) in checks {
                assert_eq!(vk, expected);
                assert_eq!(key_code(vk), Some(key));
            }
            let expected = [
                PhysicalKey::KeyW,
                PhysicalKey::KeyZ,
                PhysicalKey::KeyQ,
                PhysicalKey::KeyA,
            ];
            for (scan, place) in places.into_iter().zip(expected) {
                assert_eq!(scancode::physical_key(scan), Some(place));
            }
        })
        .join()
        .unwrap();
    }

    /// Alt seul ou F10 : le menu sans lettre est refusé. Témoins : Alt+Espace le demande
    /// avec l'espace, fermer (Alt+F4) n'est pas le menu.
    #[test]
    fn only_the_bare_key_menu_is_refused() {
        use windows_sys::Win32::UI::WindowsAndMessaging::{SC_CLOSE, SC_KEYMENU};
        assert!(is_bare_key_menu(SC_KEYMENU as usize, 0));
        assert!(is_bare_key_menu(SC_KEYMENU as usize | 0x3, 0));
        assert!(!is_bare_key_menu(SC_KEYMENU as usize, b' ' as isize));
        assert!(!is_bare_key_menu(SC_CLOSE as usize, 0));
    }
}
