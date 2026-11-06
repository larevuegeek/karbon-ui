//! Fenêtre simulée : redimensionnement, requêtes de fenêtre collectées et absence de
//! boucle d'événements interactive.
use super::Notes;
use crate::{
    platform::headless::{Headless, HeadlessApp},
    window::{WindowAction, WindowCommand, WindowId},
};

#[test]
fn resizing_relays_out_the_view_with_the_new_context() {
    let mut app = HeadlessApp::new(Notes::default()).unwrap();
    let before = app.bounds("name").unwrap();
    // Moitié de la largeur intérieure : (800 - 2 × 20) / 2.
    assert_eq!(before.right - before.left, 380.);
    assert!(app.contains_text("Width: 800"));
    let layouts = app.runtime().stats().layouts;
    app.resize(400., 300.).unwrap();
    let after = app.bounds("name").unwrap();
    assert_eq!(after.right - after.left, 180.);
    assert!(app.contains_text("Width: 400"));
    assert!(app.runtime().stats().layouts > layouts);
    assert!(app.resize(f32::NAN, 10.).is_err());
    assert_eq!(app.options().width, 400.);
}

#[test]
fn window_requests_are_collected_not_executed() {
    let mut app = HeadlessApp::new(Notes::default()).unwrap();
    app.click("close").unwrap();
    let commands = app.take_window_commands();
    assert!(matches!(
        commands[..],
        [WindowCommand::Action(WindowId::MAIN, WindowAction::Close)]
    ));
    assert!(app.take_window_commands().is_empty());
}

#[test]
fn interactive_entry_points_are_unsupported_without_a_native_backend() {
    let result = <Headless as crate::platform::Backend>::run_windows(
        vec![crate::window::WindowDescriptor::new(
            WindowId::MAIN,
            crate::window::WindowOptions::DEFAULT,
            Notes::default(),
        )],
        crate::window::ExitPolicy::LastWindowClosed,
    );
    assert!(matches!(result, Err(crate::Error::Unsupported { .. })));
}
