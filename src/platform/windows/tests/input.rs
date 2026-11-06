//! Entrées natives traduites en `UiEvent` : clic souris (WM_MOUSEMOVE, WM_LBUTTONDOWN/UP)
//! et clavier (WM_KEYDOWN) jusqu'à l'application.
use super::super::{
    ffi::scale,
    present::runtime,
    window::{state, take_error},
};
use super::*;
use crate::{
    ui::{element::ElementId, runtime::AccessibilityAction},
    window::ExitPolicy,
};
use windows_sys::Win32::UI::{
    Input::KeyboardAndMouse::{VK_DOWN, VK_ESCAPE, VK_RETURN},
    WindowsAndMessaging::{
        DestroyWindow, WM_CONTEXTMENU, WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE,
    },
};
// Bouton gauche enfoncé dans wparam de WM_LBUTTONDOWN.
const MK_LBUTTON: usize = windows_sys::Win32::System::SystemServices::MK_LBUTTON as usize;

#[test]
fn native_click_reaches_a_button_inside_an_animated_card() {
    use crate::ui::{
        animation::{Effect, Transition},
        runtime::UiEvent,
    };
    use std::time::{Duration, Instant};
    struct Animated(Rc<Cell<u64>>);
    impl Application for Animated {
        type Message = Msg;
        fn view(&self, _: &ViewContext) -> Element {
            Element::column(
                1,
                vec![
                    Element::column(
                        2,
                        vec![widgets::button(3, "Tester le clic", Msg::Click(3)).height(40)],
                    )
                    .width(240)
                    .height(120)
                    .padding(20)
                    .hover(Effect::new().translate(0., -4.).scale(1.015))
                    .pressed(Effect::new().scale(0.98))
                    .transition(Transition::new().duration(180)),
                ],
            )
            .padding(20)
        }
        fn update(&mut self, message: Msg) -> bool {
            assert_eq!(message, Msg::Click(3));
            self.0.set(self.0.get() + 1);
            true
        }
    }
    let (_apartment, session) = session();
    let count = Rc::new(Cell::new(0));
    let mut descriptor = descriptor(1, app());
    descriptor.application = Box::new(Animated(Rc::clone(&count)));
    let window = session.open(descriptor, false).unwrap();
    let _scope = state::enter(window);
    let position = || {
        let button = runtime::accessibility_snapshot()
            .unwrap()
            .into_iter()
            .find(|n| n.id == ElementId(3))
            .unwrap();
        let scale = scale(window);
        let x = ((button.clipped_bounds.left + button.clipped_bounds.right) * 0.5 * scale).round()
            as u32;
        let y = ((button.clipped_bounds.top + button.clipped_bounds.bottom) * 0.5 * scale).round()
            as u32;
        ((y << 16) | x) as isize
    };
    // SAFETY : fenêtre de test de ce thread ; aucun pointeur dans wparam/lparam.
    unsafe {
        SendMessageW(window, WM_MOUSEMOVE, 0, position());
    }
    runtime::input(
        window,
        UiEvent::Timer(Instant::now() + Duration::from_millis(200)),
    );
    let p = position();
    // SAFETY : fenêtre de test de ce thread ; aucun pointeur dans wparam/lparam.
    unsafe {
        SendMessageW(window, WM_LBUTTONDOWN, MK_LBUTTON, p);
    }
    assert!(runtime::has_capture());
    runtime::input(
        window,
        UiEvent::Timer(Instant::now() + Duration::from_millis(400)),
    );
    // SAFETY : fenêtre de test de ce thread ; aucun pointeur dans wparam/lparam.
    unsafe {
        SendMessageW(window, WM_LBUTTONUP, 0, p);
    }
    assert!(!runtime::has_capture());
    assert_eq!(count.get(), 1);
    assert!(take_error().is_none());
    // SAFETY : fenêtre de test créée sur ce thread ; Windows valide le handle.
    assert_ne!(unsafe { DestroyWindow(window) }, 0);
}

/// WM_CONTEXTMENU (clic droit ou touche Menu) ouvre le menu intégré d'un champ de saisie,
/// que les touches natives pilotent : Bas puis Entrée choisissent « Tout sélectionner »,
/// Échap ferme.
#[test]
fn native_context_menu_opens_the_field_menu_and_keys_choose_an_entry() {
    use crate::ui::text_input::TextInput;
    use windows_sys::Win32::{Foundation::POINT, Graphics::Gdi::ClientToScreen};
    #[derive(Debug, Clone, PartialEq)]
    enum FieldMsg {
        Changed(String),
    }
    struct FieldApp;
    impl Application for FieldApp {
        type Message = FieldMsg;
        fn view(&self, _: &ViewContext) -> Element {
            Element::column(
                1,
                vec![Element::custom(
                    10,
                    TextInput::new("abc", FieldMsg::Changed),
                )],
            )
            .padding(20)
        }
        fn update(&mut self, _: FieldMsg) -> bool {
            false
        }
    }
    let (_apartment, session) = session();
    let mut descriptor = descriptor(1, app());
    descriptor.application = Box::new(FieldApp);
    let window = session.open(descriptor, false).unwrap();
    let _scope = state::enter(window);
    let selection = || {
        runtime::accessibility_snapshot()
            .unwrap()
            .into_iter()
            .find(|n| n.id == ElementId(10))
            .unwrap()
            .text
            .unwrap()
            .selection
    };
    // Depuis le clavier (lparam = -1) : le champ focalisé ouvre son menu sous le caret.
    assert_eq!(
        runtime::accessibility_action(window, AccessibilityAction::Focus(ElementId(10))),
        0
    );
    // SAFETY : fenêtre de test de ce thread ; aucun pointeur dans wparam/lparam.
    unsafe { SendMessageW(window, WM_CONTEXTMENU, window as usize, -1) };
    assert!(runtime::edit_menu_open());
    // Sans sélection, la première entrée possible est Coller ; Bas → Tout sélectionner.
    // SAFETY : idem.
    unsafe {
        SendMessageW(window, WM_KEYDOWN, VK_DOWN as usize, 0);
        SendMessageW(window, WM_KEYDOWN, VK_RETURN as usize, 0);
    }
    assert!(!runtime::edit_menu_open());
    assert_eq!(selection(), 0..3);
    // Au pointeur : coordonnées d'écran dans lparam, menu ouvert sous le point ; Échap.
    let field = runtime::accessibility_snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.id == ElementId(10))
        .unwrap();
    let scale = scale(window);
    let mut point = POINT {
        x: ((field.clipped_bounds.left + field.clipped_bounds.right) * 0.5 * scale).round() as i32,
        y: ((field.clipped_bounds.top + field.clipped_bounds.bottom) * 0.5 * scale).round() as i32,
    };
    let client = ((point.y as u32) << 16 | point.x as u32) as isize;
    // SAFETY : fenêtre de test de ce thread ; structure locale.
    unsafe {
        SendMessageW(window, WM_MOUSEMOVE, 0, client);
        assert_ne!(ClientToScreen(window, &mut point), 0);
    }
    let screen = ((point.y as u16 as u32) << 16 | point.x as u16 as u32) as isize;
    // SAFETY : fenêtre de test de ce thread ; aucun pointeur dans wparam/lparam.
    unsafe { SendMessageW(window, WM_CONTEXTMENU, window as usize, screen) };
    assert!(runtime::edit_menu_open());
    // SAFETY : idem.
    unsafe { SendMessageW(window, WM_KEYDOWN, VK_ESCAPE as usize, 0) };
    assert!(!runtime::edit_menu_open());
    assert!(!runtime::has_capture());
    assert!(take_error().is_none());
    // SAFETY : fenêtre de test créée sur ce thread ; Windows valide le handle.
    assert_ne!(unsafe { DestroyWindow(window) }, 0);
}

#[test]
fn sent_input_defers_exit_without_requiring_an_application_rebuild() {
    let (_apartment, session) = session();
    let mut application = app();
    application.exit_on_update = true;
    let count = Rc::clone(&application.count);
    let window = session.open(descriptor(1, application), false).unwrap();
    {
        let _scope = state::enter(window);
        assert_eq!(
            runtime::accessibility_action(window, AccessibilityAction::Focus(ElementId(3))),
            0
        );
    }
    sent_message_while_waiting(window, WM_KEYDOWN, VK_RETURN as usize, || {
        session
            .run_message_loop(ExitPolicy::LastWindowClosed)
            .unwrap();
    });
    assert_eq!(count.get(), 1);
    assert!(state::windows().is_empty());
}
