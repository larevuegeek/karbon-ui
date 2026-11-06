//! État de la liste de choix : conservation à la reconstruction, commandes
//! d'accessibilité par clé et validation.
use super::*;

#[test]
fn rebuilding_with_same_options_preserves_state_and_external_changes_close_safely() {
    let mut f = Fixture::new();
    f.expand();
    f.down(Key::Down);
    f.ui.invalidate();
    f.prepare();
    assert!(f.info().expanded);
    assert_eq!(f.active(), Some(3));
    let mut options = f.model.borrow().options.to_vec();
    options.swap(0, 3);
    f.model.borrow_mut().options = options.into();
    f.ui.invalidate();
    f.prepare();
    assert!(!f.info().expanded);
    assert_eq!(f.model.borrow().selected, Some(1));
    f.expand();
    f.event(UiEvent::FocusChanged(false));
    assert!(!f.info().expanded);
    assert!(f.model.borrow().changes.is_empty());
    f.expand();
    f.model.borrow_mut().enabled = false;
    f.ui.invalidate();
    f.prepare();
    assert!(!f.info().expanded);
}

#[test]
fn accessible_expansion_is_idempotent_and_options_use_keys_instead_of_positions() {
    use crate::ui::runtime::{AccessibilityAction as A, AccessibilityError as E};
    let mut f = Fixture::new();
    f.expand();
    f.down(Key::Down);
    f.expand();
    assert_eq!(f.active(), Some(3));
    f.ui.accessibility_action(A::ExpandSelect {
        id: ElementId(11),
        expanded: false,
    })
    .unwrap();
    f.prepare();
    assert!(f.info().expanded);
    assert_eq!(
        f.ui.accessibility_action(A::ChooseOption {
            id: ElementId(10),
            option: 2
        }),
        Err(E::Disabled)
    );
    assert_eq!(
        f.ui.accessibility_action(A::ChooseOption {
            id: ElementId(10),
            option: 999
        }),
        Err(E::InvalidValue)
    );
    f.ui.accessibility_action(A::HighlightOption {
        id: ElementId(10),
        option: 24,
    })
    .unwrap();
    f.prepare();
    assert_eq!(f.active(), Some(24));
    assert_eq!(f.model.borrow().selected, Some(1));
    f.ui.accessibility_action(A::ChooseOption {
        id: ElementId(10),
        option: 24,
    })
    .unwrap();
    f.prepare();
    assert!(!f.info().expanded);
    assert_eq!(f.model.borrow().selected, Some(24));
    f.focus(2);
    f.ui.accessibility_action(A::ChooseOption {
        id: ElementId(10),
        option: 1,
    })
    .unwrap();
    f.prepare();
    assert_eq!(f.ui.focused(), Some(ElementId(2)));
    assert_eq!(
        f.ui.accessibility_action(A::HighlightOption {
            id: ElementId(10),
            option: 1
        }),
        Err(E::Unavailable)
    );
}

#[test]
fn select_validation_and_empty_lists_are_explicit() {
    for select in [
        Select::new(Some(7), vec![SelectOption::new(1, "Un")], |key| {
            Msg::Select(1, key)
        }),
        Select::new(
            None,
            vec![SelectOption::new(1, "Un"), SelectOption::new(1, "Doublon")],
            |key| Msg::Select(1, key),
        ),
        Select::new(None, Vec::<SelectOption>::new(), |key| Msg::Select(1, key)).visible_items(0),
    ] {
        assert!(crate::error::rejected(layout::validate(&Element::custom(
            1, select
        ))));
    }
    let mut f = Fixture::new();
    {
        let mut m = f.model.borrow_mut();
        m.selected = None;
        m.options = Vec::new().into();
    }
    f.ui.invalidate();
    f.prepare();
    f.expand();
    assert!(f.info().expanded);
    assert!(f.info().options.is_empty());
    f.down(Key::Enter);
    assert!(f.model.borrow().changes.is_empty());
    f.down(Key::Escape);
    assert!(!f.info().expanded);
    assert!(f.ui.scene().validate().is_ok());
}
