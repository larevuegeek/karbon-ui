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

#[test]
fn window_states_reach_the_application() {
    use crate::{
        application::{Application, ViewContext},
        ui::{element::Element, widgets},
        window::WindowState,
    };
    #[derive(Default)]
    struct Player {
        state: WindowState,
        released: u32,
    }
    impl Application for Player {
        type Message = ();
        fn view(&self, _: &ViewContext) -> Element {
            widgets::text(1, format!("{}x{}", self.state.width, self.state.height))
        }
        fn update(&mut self, _: ()) -> bool {
            false
        }
        fn window_changed(&mut self, state: WindowState) -> bool {
            // Fin d'un geste : la taille est retenue une fois.
            if self.state.dragging && !state.dragging {
                self.released += 1;
            }
            self.state = state;
            true
        }
    }
    let mut app = HeadlessApp::new(Player::default()).unwrap();
    let dragging = WindowState {
        width: 300,
        height: 300,
        scale: 1.,
        dragging: true,
        ..Default::default()
    };
    assert!(app.window_changed(dragging).unwrap());
    assert!(app.contains_text("300x300"));
    let released = WindowState {
        dragging: false,
        ..dragging
    };
    app.window_changed(released).unwrap();
    assert_eq!(app.app().released, 1);
}
