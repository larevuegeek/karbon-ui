//! Accessibilité du champ de saisie : valeur, sélection et révélation sans voler le
//! focus, texte affiché pendant une composition.
use super::*;

#[test]
fn accessibility_sets_values_without_stealing_focus_and_rejects_invalid_edits() {
    use crate::ui::runtime::{AccessibilityAction as Action, AccessibilityError as Error};
    let mut f = Fixture::new("");
    f.ui.accessibility_action(Action::Focus(ElementId(20)))
        .unwrap();
    f.prepare();
    f.ui.accessibility_action(Action::SetValue {
        id: ElementId(10),
        value: "Café 外部 😃".into(),
    })
    .unwrap();
    f.prepare();
    assert_eq!(&*f.value.borrow(), "Café 外部 😃");
    assert_eq!(f.ui.focused(), Some(ElementId(20)));
    let node =
        f.ui.accessibility()
            .into_iter()
            .find(|n| n.id == ElementId(10))
            .unwrap();
    assert_eq!(node.value.as_deref(), Some("Café 外部 😃"));
    assert_eq!(node.name, "Recherche");
    assert!(!node.read_only);
    assert!(!node.focused);
    assert_eq!(
        f.ui.accessibility_action(Action::SetValue {
            id: ElementId(11),
            value: "X".into()
        }),
        Err(Error::ReadOnly)
    );
    assert_eq!(
        f.ui.accessibility_action(Action::SetValue {
            id: ElementId(12),
            value: "X".into()
        }),
        Err(Error::Disabled)
    );
    assert_eq!(
        f.ui.accessibility_action(Action::Invoke(ElementId(10))),
        Err(Error::Unsupported)
    );
    assert_eq!(
        f.ui.accessibility_action(Action::Focus(ElementId(999))),
        Err(Error::Unavailable)
    );
}

#[test]
fn accessible_selection_preserves_value_focus_and_history_and_rejects_stale_text() {
    use crate::ui::runtime::{AccessibilityAction as Action, AccessibilityError as Error};
    let mut f = Fixture::new("Café 😃");
    f.text("!");
    f.ui.accessibility_action(Action::Focus(ElementId(20)))
        .unwrap();
    f.prepare();
    let version = f.ui.accessibility_version();
    let changes = f.changes.get();
    f.ui.accessibility_action(Action::SelectText {
        id: ElementId(10),
        range: 3..5,
        expected: "Café 😃!".into(),
    })
    .unwrap();
    f.prepare();
    assert_eq!(f.ui.focused(), Some(ElementId(20)));
    assert_eq!(f.state().selection, 3..5);
    assert_eq!(f.state().value, "Café 😃!");
    assert_eq!(f.changes.get(), changes);
    assert_ne!(f.ui.accessibility_version(), version);
    let snapshot =
        f.ui.accessibility()
            .into_iter()
            .find(|n| n.id == ElementId(10))
            .unwrap()
            .text
            .unwrap();
    assert_eq!(snapshot.selection, 3..5);
    assert_eq!(snapshot.value.as_ref(), "Café 😃!");
    // Champ monoligne : une seule ligne visuelle, tout le texte affiché.
    assert!(!snapshot.multiline);
    assert_eq!(snapshot.lines.len(), 1);
    assert_eq!(snapshot.lines[0].range, 0..snapshot.value.len());
    assert_eq!(snapshot.lines[0].rect.top, snapshot.clusters[0].rect.top);
    assert_eq!(
        f.ui.accessibility_action(Action::SelectText {
            id: ElementId(10),
            range: 0..1,
            expected: "Obsolète".into()
        }),
        Err(Error::Unavailable)
    );
    assert_eq!(f.state().selection, 3..5);
    f.ui.accessibility_action(Action::SelectText {
        id: ElementId(11),
        range: 0..7,
        expected: "Lecture seule".into(),
    })
    .unwrap();
    assert_eq!(
        f.ui.accessibility_action(Action::SelectText {
            id: ElementId(12),
            range: 0..1,
            expected: "Inactif".into()
        }),
        Err(Error::Disabled)
    );
    f.ui.accessibility_action(Action::Focus(ElementId(10)))
        .unwrap();
    f.prepare();
    f.edit(EditCommand::Undo);
    assert_eq!(f.state().value, "Café 😃");
    let version = f.ui.accessibility_version();
    f.event(UiEvent::CaretTick);
    assert_eq!(f.ui.accessibility_version(), version);
    f.key(Key::Left, false);
    assert_ne!(f.ui.accessibility_version(), version);
}

#[test]
fn revealing_text_preserves_selection_and_focus_and_survives_preparation() {
    use crate::ui::runtime::AccessibilityAction as Action;
    let mut f = Fixture::new("abcdefghijklmnopqrstuvwxyz012345");
    f.ui.accessibility_action(Action::Focus(ElementId(20)))
        .unwrap();
    let narrow = ViewContext {
        width: 100.,
        ..context()
    };
    f.ui.prepare(narrow, ChromeState::default(), &mut f.measure)
        .unwrap();
    let selection = f.state().selection;
    let first_x = |ui: &UiRuntime| {
        ui.accessibility()
            .into_iter()
            .find(|n| n.id == ElementId(10))
            .unwrap()
            .text
            .unwrap()
            .carets[0]
            .x
    };
    assert!(first_x(&f.ui) < f.state().bounds.left);
    f.ui.accessibility_action(Action::RevealText {
        id: ElementId(10),
        byte: 0,
    })
    .unwrap();
    for _ in 0..2 {
        f.ui.prepare(narrow, ChromeState::default(), &mut f.measure)
            .unwrap();
    }
    assert!(first_x(&f.ui) >= f.state().bounds.left);
    assert_eq!(f.state().selection, selection);
    assert_eq!(f.ui.focused(), Some(ElementId(20)));
    assert_eq!(f.changes.get(), 0);

    let mut ui = view_runtime(|| {
        Element::scroll(
            1,
            Axis::Vertical,
            Element::column(
                2,
                vec![
                    widgets::text(3, "Avant").height(Length::Px(1000.)),
                    Element::custom(4, TextInput::new("En bas", |text| Msg::Text(1, text))),
                ],
            ),
        )
    });
    let mut m = Measure::default();
    prepare(&mut ui, &mut m);
    let before = ui.focused();
    ui.accessibility_action(Action::RevealText {
        id: ElementId(4),
        byte: 0,
    })
    .unwrap();
    prepare(&mut ui, &mut m);
    assert_eq!(ui.focused(), before);
    assert!(
        !ui.accessibility()
            .iter()
            .find(|n| n.id == ElementId(4))
            .unwrap()
            .offscreen
    );
}

#[test]
fn accessible_composition_tracks_displayed_text_without_cancelling_ime() {
    use crate::ui::runtime::{AccessibilityAction as Action, AccessibilityError as Error};
    let mut f = Fixture::new("ab");
    f.event(UiEvent::CompositionStart);
    f.event(UiEvent::CompositionUpdate {
        value: "😃".into(),
        cursor_utf16: 2,
    });
    let t =
        f.ui.accessibility()
            .into_iter()
            .find(|n| n.id == ElementId(10))
            .unwrap()
            .text
            .unwrap();
    assert_eq!(t.value.as_ref(), "ab😃");
    assert_eq!(t.selection, 6..6);
    assert_eq!(f.state().value, "ab");
    assert_eq!(
        f.ui.accessibility_action(Action::SelectText {
            id: ElementId(10),
            range: 0..1,
            expected: t.value
        }),
        Err(Error::Unavailable)
    );
    assert!(f.state().composing);
    f.event(UiEvent::CompositionCommit("😃".into()));
    assert_eq!(f.state().value, "ab😃");
}
