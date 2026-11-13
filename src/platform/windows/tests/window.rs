//! Cycle de vie des fenêtres natives : identifiants, commandes différées, état isolé
//! par fenêtre, panique applicative (procédure de fenêtre, boucle, chargement), peinture
//! imbriquée dans un rappel, politique de sortie et réveil de la boucle.
use super::super::{
    events::text,
    present::runtime,
    window::{shadow, state, take_error},
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
    // L'ombre n'a pas de fenêtre propriétaire : seul le démontage la détruit.
    let shade = {
        let _scope = state::enter(window);
        shadow::create_hidden()
    };
    armed.set(true);
    // SAFETY : fenêtre de test de ce thread ; aucun pointeur dans wparam/lparam.
    unsafe {
        SendMessageW(window, WM_NULL, 0, 0); // Seuls les rappels communs s'exécutent.
    }
    // SAFETY : simple test de validité d'un handle.
    assert_eq!(unsafe { IsWindow(window) }, 0);
    // SAFETY : simple test de validité d'un handle.
    assert_eq!(unsafe { IsWindow(shade) }, 0);
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

/// Application qui panique dans le rappel nommé par `armed`.
struct PanicIn(Rc<Cell<&'static str>>);
impl PanicIn {
    fn check(&self, callback: &'static str) {
        if self.0.get() == callback {
            panic!("{callback} en échec");
        }
    }
}
impl Application for PanicIn {
    type Message = ();
    fn load(&mut self, _: &dyn Resources) -> crate::Result<()> {
        self.check("load");
        Ok(())
    }
    fn update(&mut self, _: ()) -> bool {
        false
    }
    fn view(&self, _: &ViewContext) -> Element {
        widgets::text(1, "Panique armée")
    }
    fn take_window_commands(&mut self) -> Vec<WindowCommand> {
        self.check("window");
        Vec::new()
    }
    fn take_task_commands(&mut self) -> Vec<crate::task::TaskCommand> {
        self.check("task");
        Vec::new()
    }
}
fn panicking(id: u64, armed: &Rc<Cell<&'static str>>) -> WindowDescriptor {
    WindowDescriptor {
        application: Box::new(PanicIn(Rc::clone(armed))),
        ..descriptor(id, app())
    }
}
fn is_panic(error: &crate::Error, expected: &str) -> bool {
    matches!(error, crate::Error::Panicked { message } if message == expected)
}

#[test]
fn application_panic_while_loading_is_returned_by_open() {
    let (_apartment, session) = session();
    let other = session.open(descriptor(1, app()), false).unwrap();
    let armed = Rc::new(Cell::new("load"));
    let error = session.open(panicking(2, &armed), false).unwrap_err();
    assert!(is_panic(&error, "load en échec"), "{error}");
    assert_eq!(state::windows().len(), 1);
    // SAFETY : simple test de validité d'un handle.
    assert_ne!(unsafe { IsWindow(other) }, 0);
    assert!(take_error().is_none());
}

#[test]
fn application_panic_in_loop_callbacks_closes_its_window_and_is_returned() {
    let (_apartment, session) = session();
    let armed = Rc::new(Cell::new(""));
    // Commandes de fenêtre lues par la boucle (hors de `window_proc`).
    let window = session.open(panicking(1, &armed), false).unwrap();
    let other = session.open(descriptor(2, app()), false).unwrap();
    armed.set("window");
    let error = session.commands(false).unwrap_err();
    assert!(is_panic(&error, "window en échec"), "{error}");
    // SAFETY : simple test de validité d'un handle.
    assert_eq!(unsafe { IsWindow(window) }, 0);
    // SAFETY : simple test de validité d'un handle.
    assert_ne!(unsafe { IsWindow(other) }, 0);
    assert!(take_error().is_none());
    // Commandes de tâches lues par la boucle : l'erreur est rendue au tour suivant.
    armed.set("");
    let window = session.open(panicking(3, &armed), false).unwrap();
    armed.set("task");
    assert!(session.tasks.process());
    // SAFETY : simple test de validité d'un handle.
    assert_eq!(unsafe { IsWindow(window) }, 0);
    let error = session
        .run_message_loop(ExitPolicy::LastWindowClosed)
        .unwrap_err();
    assert!(is_panic(&error, "task en échec"), "{error}");
    // SAFETY : simple test de validité d'un handle.
    assert_ne!(unsafe { IsWindow(other) }, 0);
    // SAFETY : fenêtre de test créée sur ce thread ; Windows valide le handle.
    unsafe {
        DestroyWindow(other);
    }
}

#[test]
fn paint_nested_in_an_application_callback_is_deferred_instead_of_panicking() {
    let (_apartment, session) = session();
    let application = app();
    let count = Rc::clone(&application.count);
    let nested = Rc::clone(&application.nested_target);
    let window = session.open(descriptor(1, application), false).unwrap();
    // `update` envoie WM_KEYDOWN puis WM_PAINT à sa propre fenêtre pendant que son état
    // de dessin est emprunté, comme la boucle modale d'un `MessageBoxW` ouvert par elle.
    nested.set(window);
    let _scope = state::enter(window);
    assert_eq!(
        runtime::accessibility_action(window, AccessibilityAction::Invoke(ElementId(3))),
        0
    );
    assert_eq!(count.get(), 1);
    assert!(take_error().is_none());
    // SAFETY : simple test de validité d'un handle.
    assert_eq!(unsafe { IsWindow(window) }, 1);
    // La fenêtre reste utilisable : peinture et rappels suivants.
    nested.set(null_mut());
    super::super::present::paint::paint_window(window);
    assert!(take_error().is_none());
    assert_eq!(
        runtime::accessibility_action(window, AccessibilityAction::Invoke(ElementId(3))),
        0
    );
    assert_eq!(count.get(), 2);
    // SAFETY : fenêtre de test créée sur ce thread ; Windows valide le handle.
    assert_ne!(unsafe { DestroyWindow(window) }, 0);
}
