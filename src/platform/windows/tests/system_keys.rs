//! Touches système (WM_SYSKEYDOWN, WM_SYSKEYUP : F10 et les touches tenues avec Alt)
//! jusqu'à `Application::key`, et place physique des touches, relâchement compris. Les
//! messages sont envoyés à une fenêtre cachée du test, rien ne passe par le bureau.
use super::super::window::{state, take_error};
use super::*;
use crate::ui::runtime::{KeyCode, KeyPress, PhysicalKey};
use windows_sys::Win32::UI::{
    Input::KeyboardAndMouse::{VK_CONTROL, VK_F10, VK_MENU, VK_Q},
    WindowsAndMessaging::{DestroyWindow, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP},
};

/// Bit 29 de lparam (KF_ALTDOWN dans le mot haut) : Alt est tenu.
const ALT_DOWN: isize = 1 << 29;
/// Bit 24 de lparam : touche étendue.
const EXTENDED: isize = 1 << 24;

/// lparam d'un message du clavier : un appui, au code de balayage `code`.
fn lparam(code: isize) -> isize {
    (code << 16) | 1
}

struct Keys(Rc<RefCell<Vec<KeyPress>>>);
impl Application for Keys {
    type Message = ();
    fn view(&self, _: &ViewContext) -> Element {
        Element::column(1, vec![widgets::text(2, "Touches système")])
    }
    fn update(&mut self, _: ()) -> bool {
        false
    }
    fn key(&mut self, press: KeyPress) -> bool {
        self.0.borrow_mut().push(press);
        false
    }
}

fn keys_window() -> (ComApartment, Session, Hwnd, Rc<RefCell<Vec<KeyPress>>>) {
    let (apartment, session) = session();
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut descriptor = descriptor(1, app());
    descriptor.application = Box::new(Keys(Rc::clone(&log)));
    let window = session.open(descriptor, false).unwrap();
    (apartment, session, window, log)
}

/// Touche, appui, place et code de balayage des touches reçues.
fn seen(log: &RefCell<Vec<KeyPress>>) -> Vec<(KeyCode, bool, Option<PhysicalKey>, u32)> {
    log.borrow()
        .iter()
        .map(|p| (p.key, p.pressed, p.physical, p.scancode))
        .collect()
}

#[test]
fn f10_and_keys_held_with_alt_reach_the_application() {
    let (_apartment, _session, window, log) = keys_window();
    let _scope = state::enter(window);
    // F10 seul, Alt+Q enfoncé et relâché, puis Alt de droite (touche étendue). Ni F10 ni
    // Alt ne sont relâchés : leur relâchement ouvrirait le menu de la fenêtre, une boucle
    // modale qui suspendrait le test.
    // SAFETY : fenêtre de test de ce thread ; aucun pointeur dans wparam/lparam.
    unsafe {
        SendMessageW(window, WM_SYSKEYDOWN, VK_F10 as usize, lparam(0x44));
        let q = lparam(0x10) | ALT_DOWN;
        SendMessageW(window, WM_SYSKEYDOWN, VK_Q as usize, q);
        SendMessageW(window, WM_SYSKEYUP, VK_Q as usize, q);
        let right_alt = lparam(0x38) | EXTENDED | ALT_DOWN;
        SendMessageW(window, WM_SYSKEYDOWN, VK_MENU as usize, right_alt);
    }
    assert_eq!(
        seen(&log),
        [
            (KeyCode::F10, true, Some(PhysicalKey::F10), 0x44),
            (KeyCode::Q, true, Some(PhysicalKey::KeyQ), 0x10),
            (KeyCode::Q, false, Some(PhysicalKey::KeyQ), 0x10),
            (KeyCode::Alt, true, Some(PhysicalKey::AltRight), 0xE038),
        ]
    );
    assert!(take_error().is_none());
    // SAFETY : fenêtre de test créée sur ce thread ; Windows valide le handle.
    assert_ne!(unsafe { DestroyWindow(window) }, 0);
}

/// Le relâchement passe par `keyboard::key_up`, qui ne transmet pas lparam : sa place
/// vient quand même du message (Ctrl de droite, et non celui de gauche que donnerait le
/// seul code virtuel).
#[test]
fn a_released_key_keeps_its_place() {
    let (_apartment, _session, window, log) = keys_window();
    let _scope = state::enter(window);
    let right_control = lparam(0x1D) | EXTENDED;
    // SAFETY : fenêtre de test de ce thread ; aucun pointeur dans wparam/lparam.
    unsafe {
        SendMessageW(window, WM_KEYDOWN, VK_CONTROL as usize, right_control);
        SendMessageW(window, WM_KEYUP, VK_CONTROL as usize, right_control);
    }
    let right = Some(PhysicalKey::ControlRight);
    assert_eq!(
        seen(&log),
        [
            (KeyCode::Control, true, right, 0xE01D),
            (KeyCode::Control, false, right, 0xE01D),
        ]
    );
    assert!(take_error().is_none());
    // SAFETY : fenêtre de test créée sur ce thread ; Windows valide le handle.
    assert_ne!(unsafe { DestroyWindow(window) }, 0);
}
