//! Cycle de vie des fenêtres natives : identifiants, commandes différées, état isolé
//! par fenêtre, panique applicative, politique de sortie et réveil de la boucle.
use super::super::{
    events::text,
    present::runtime,
    window::{state, take_error},
};
use super::*;
use crate::{
    ui::{element::ElementId, runtime::AccessibilityAction},
    window::ExitPolicy,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{DestroyWindow, IsWindow, WM_CLOSE, WM_NULL};

#[test]
fn custom_title_bar_command_closes_only_the_requested_native_window() {
    let (_apartment, session) = session();
    let first = app();
    let commands = Rc::clone(&first.commands);
    let first = session.open(descriptor(1, first), false).unwrap();
    let second = session.open(descriptor(2, app()), false).unwrap();
    commands.borrow_mut().extend([
        WindowCommand::Action(WindowId(404), crate::window::WindowAction::Close),
        WindowCommand::Action(WindowId(1), crate::window::WindowAction::Close),
    ]);
    assert!(session.commands(false).unwrap());
    // SAFETY : simple test de validité d'un handle.
    assert_eq!(unsafe { IsWindow(first) }, 0);
    // SAFETY : simple test de validité d'un handle.
    assert_ne!(unsafe { IsWindow(second) }, 0);
    assert!(take_error().is_none());
    // SAFETY : fenêtre de test créée sur ce thread ; Windows valide le handle.
    unsafe {
        DestroyWindow(second);
    }
}

#[test]
fn native_windows_isolate_runtime_theme_decoder_and_reentrant_callbacks() {
    let (_apartment, session) = session();
    let first = app();
    let count_a = Rc::clone(&first.count);
    let drops_a = Rc::clone(&first.drops);
    let nested = Rc::clone(&first.nested_target);
    let second = app();
    let count_b = Rc::clone(&second.count);
    let a = session.open(descriptor(1, first), false).unwrap();
    // F5 (bascule du thème, envoyée à B depuis un rappel de A) est une touche de diagnostic :
    // elle ne répond qu'avec l'option `profile`.
    let mut second = descriptor(2, second);
    second.options.profile = true;
    let b = session.open(second, false).unwrap();
    nested.set(b);
    let a_scope = state::enter(a);
    assert_eq!(
        runtime::accessibility_action(a, AccessibilityAction::Invoke(ElementId(3))),
        0
    );
    assert_eq!(count_a.get(), 1);
    assert_eq!(count_b.get(), 0);
    assert!(!state::options().light);
    assert!(text::character(0xD83D).is_none());
    {
        let _b_scope = state::enter(b);
        assert!(state::options().light);
        assert!(text::character(0xDE03).is_some());
        assert_eq!(
            runtime::accessibility_action(b, AccessibilityAction::Invoke(ElementId(3))),
            0
        );
    }
    assert!(
        matches!(text::character(0xDE03), Some(crate::ui::runtime::UiEvent::Text(value)) if &*value == "😃")
    );
    assert_eq!(count_b.get(), 1);
    drop(a_scope);
    // SAFETY : fenêtre de test créée sur ce thread ; Windows valide le handle.
    unsafe {
        DestroyWindow(a);
    }
    assert_eq!(drops_a.get(), 1);
    // SAFETY : simple test de validité d'un handle.
    assert_eq!(unsafe { IsWindow(b) }, 1);
    assert!(!session.should_exit(ExitPolicy::LastWindowClosed));
    assert!(session.should_exit(ExitPolicy::MainWindowClosed(WindowId(1))));
    drop(session);
    assert!(state::windows().is_empty());
    // SAFETY : simple test de validité d'un handle.
    assert_eq!(unsafe { IsWindow(b) }, 0);
}

#[test]
fn deferred_commands_open_close_reopen_and_exit_without_dropping_other_windows() {
    let (_apartment, session) = session();
    let main = app();
    let commands = Rc::clone(&main.commands);
    let main_window = session.open(descriptor(1, main), false).unwrap();
    let secondary = app();
    let drops = Rc::clone(&secondary.drops);
    commands
        .borrow_mut()
        .push(WindowCommand::Open(descriptor(2, secondary)));
    assert!(session.commands(false).unwrap());
    assert_eq!(state::windows().len(), 2);
    assert!(!session.commands(false).unwrap());
    commands.borrow_mut().extend([
        WindowCommand::Close(WindowId(2)),
        WindowCommand::Close(WindowId(2)),
    ]);
    session.commands(false).unwrap();
    assert_eq!(drops.get(), 1);
    // SAFETY : simple test de validité d'un handle.
    assert_eq!(unsafe { IsWindow(main_window) }, 1);
    commands
        .borrow_mut()
        .push(WindowCommand::Open(descriptor(2, app())));
    session.commands(false).unwrap();
    assert_eq!(state::windows().len(), 2);
    commands.borrow_mut().push(WindowCommand::Exit);
    session.commands(false).unwrap();
    assert!(state::windows().is_empty());
}

#[test]
fn failed_load_and_duplicate_ids_release_resources_and_preserve_existing_window() {
    let (_apartment, session) = session();
    let a = session.open(descriptor(1, app()), false).unwrap();
    assert!(matches!(
        session.open(descriptor(1, app()), false),
        Err(crate::Error::DuplicateWindowId(WindowId(1)))
    ));
    let mut failing = app();
    failing.fail_load = true;
    let drops = Rc::clone(&failing.drops);
    assert!(matches!(
        session.open(descriptor(2, failing), false),
        Err(crate::Error::Application(error)) if error.to_string() == "load rejected by the test"
    ));
    assert_eq!(drops.get(), 1);
    assert_eq!(state::windows().len(), 1);
    // SAFETY : simple test de validité d'un handle.
    assert_eq!(unsafe { IsWindow(a) }, 1);
    assert!(!session.should_exit(ExitPolicy::LastWindowClosed));
    drop(session);
    assert!(state::windows().is_empty());
}

#[test]
fn application_panic_closes_its_window_and_is_returned_as_an_error() {
    struct Panicking(Rc<Cell<bool>>);
    impl Application for Panicking {
        type Message = ();
        fn update(&mut self, _: ()) -> bool {
            false
        }
        fn view(&self, _: &ViewContext) -> Element {
            widgets::text(1, "Panique armée")
        }
        fn take_window_commands(&mut self) -> Vec<WindowCommand> {
            if self.0.get() {
                panic!("callback applicatif en échec");
            }
            Vec::new()
        }
    }
    let (_apartment, session) = session();
    let armed = Rc::new(Cell::new(false));
    let panicking = WindowDescriptor {
        application: Box::new(Panicking(Rc::clone(&armed))),
        ..descriptor(1, app())
    };
    let window = session.open(panicking, false).unwrap();
    let other = session.open(descriptor(2, app()), false).unwrap();
    armed.set(true);
    // SAFETY : fenêtre de test de ce thread ; aucun pointeur dans wparam/lparam.
    unsafe {
        SendMessageW(window, WM_NULL, 0, 0); // Seuls les rappels communs s'exécutent.
    }
    // SAFETY : simple test de validité d'un handle.
    assert_eq!(unsafe { IsWindow(window) }, 0);
    // SAFETY : simple test de validité d'un handle.
    assert_ne!(unsafe { IsWindow(other) }, 0);
    let error = session
        .run_message_loop(ExitPolicy::LastWindowClosed)
        .unwrap_err();
    assert!(
        matches!(&error, crate::Error::Panicked { message } if message == "callback applicatif en échec"),
        "{error}"
    );
    assert!(take_error().is_none());
    // SAFETY : fenêtre de test créée sur ce thread ; Windows valide le handle.
    unsafe {
        DestroyWindow(other);
    }
}

#[test]
fn sent_close_wakes_get_message_after_the_last_native_window_disappears() {
    let (_apartment, session) = session();
    let window = session.open(descriptor(1, app()), false).unwrap();
    sent_message_while_waiting(window, WM_CLOSE, 0, || {
        session
            .run_message_loop(ExitPolicy::LastWindowClosed)
            .unwrap();
    });
    assert!(state::windows().is_empty());
}

#[test]
fn main_window_policy_returns_then_session_closes_remaining_windows() {
    let (_apartment, session) = session();
    let main = app();
    let commands = Rc::clone(&main.commands);
    session.open(descriptor(1, main), false).unwrap();
    let remaining = app();
    let drops = Rc::clone(&remaining.drops);
    let other = session.open(descriptor(2, remaining), false).unwrap();
    commands
        .borrow_mut()
        .push(WindowCommand::Close(WindowId(1)));
    session
        .run_message_loop(ExitPolicy::MainWindowClosed(WindowId(1)))
        .unwrap();
    drop(session);
    // SAFETY : simple test de validité d'un handle.
    assert_eq!(unsafe { IsWindow(other) }, 0);
    assert_eq!(drops.get(), 1);
}
