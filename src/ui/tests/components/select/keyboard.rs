//! Liste de choix au clavier : options inactives sautées, validation explicite et
//! recherche par préfixe tapé.
use super::*;

#[test]
fn keyboard_navigation_skips_disabled_options_and_only_commits_on_confirmation() {
    let mut f = Fixture::new();
    f.focus(10);
    f.down(Key::Down);
    assert!(f.info().expanded);
    assert_eq!(f.active(), Some(1));
    f.down(Key::Down);
    assert_eq!(f.active(), Some(3));
    assert_eq!(f.model.borrow().selected, Some(1));
    f.down(Key::Enter);
    assert!(!f.info().expanded);
    assert_eq!(f.model.borrow().selected, Some(3));
    f.event(UiEvent::KeyUp(Key::Enter));
    f.down(Key::Space);
    assert!(!f.info().expanded);
    f.event(UiEvent::KeyUp(Key::Space));
    assert!(f.info().expanded);
    f.down(Key::End);
    assert_eq!(f.active(), Some(24));
    assert!(
        f.info()
            .options
            .iter()
            .find(|o| o.id == 24)
            .unwrap()
            .clip
            .bottom
            > f.info()
                .options
                .iter()
                .find(|o| o.id == 24)
                .unwrap()
                .clip
                .top
    );
    assert!(f.down(Key::Escape).handled);
    assert!(!f.info().expanded);
    assert_eq!(f.model.borrow().selected, Some(3));
    assert_eq!(f.ui.focused(), Some(ElementId(10)));
    f.expand();
    f.down(Key::Home);
    f.down(Key::Tab);
    assert!(!f.info().expanded);
    assert_eq!(f.ui.focused(), Some(ElementId(3)));
    assert_eq!(f.model.borrow().changes, &[3]);
}

#[test]
fn typed_prefix_and_repeated_letter_move_highlight_without_changing_selection() {
    let mut f = Fixture::new();
    f.focus(10);
    f.event(UiEvent::Text("B".into()));
    assert!(f.info().expanded);
    assert_eq!(f.active(), Some(3));
    f.event(UiEvent::Text("b".into()));
    assert_eq!(f.active(), Some(4));
    assert_eq!(f.model.borrow().selected, Some(1));
    f.down(Key::Home);
    f.event(UiEvent::Text("Sou".into()));
    assert_eq!(f.active(), Some(5));
    f.down(Key::Enter);
    assert_eq!(f.model.borrow().selected, Some(5));
}
