//! Édition du champ de saisie : valeur, sélection, presse-papiers, annulation,
//! composition IME, sélection au pointeur et navigation par mot.
use super::*;

// Porté sur le harnais headless : saisie, touches et relecture sans Fixture dédiée.
#[test]
fn editing_preserves_value_selection_focus_and_external_resets() {
    use crate::platform::headless::HeadlessApp;
    let value = Rc::new(RefCell::new(String::new()));
    let changes = Rc::new(Cell::new(0));
    let mut app = HeadlessApp::new(InputApp {
        value: value.clone(),
        changes: changes.clone(),
        submits: Rc::new(Cell::new(0)),
    })
    .unwrap();
    app.press(Key::Tab).unwrap();
    assert_eq!(app.focused(), Some(ElementId(10)));
    // Un seul événement texte (collage ou IME) : une seule notification.
    app.event(UiEvent::Text("Café 😃".into())).unwrap();
    assert_eq!(&*value.borrow(), "Café 😃");
    assert_eq!(changes.get(), 1);
    app.press(Key::Left).unwrap();
    app.type_text("!").unwrap();
    assert_eq!(&*value.borrow(), "Café !😃");
    app.press(Key::Backspace).unwrap();
    assert_eq!(&*value.borrow(), "Café 😃");
    app.press(Key::Delete).unwrap();
    assert_eq!(&*value.borrow(), "Café ");
    app.press(Key::Home).unwrap();
    app.press_with_shift(Key::Right).unwrap();
    let selection = |app: &HeadlessApp<InputApp>| {
        app.runtime()
            .input_snapshot(ElementId(10))
            .unwrap()
            .selection
    };
    assert_eq!(selection(&app), 0..1);
    app.type_text("B").unwrap();
    assert_eq!(&*value.borrow(), "Bafé ");
    app.runtime_mut().invalidate();
    app.prepare().unwrap();
    assert_eq!(selection(&app), 1..1);
    app.update_app(|input| *input.value.borrow_mut() = "Autre".into())
        .unwrap();
    assert_eq!(app.input_value(10).as_deref(), Some("Autre"));
    assert_eq!(app.focused(), Some(ElementId(10)));
    assert!(app.scene().validate().is_ok());
}

#[test]
fn clipboard_requests_cut_confirmation_read_only_and_disabled_focus() {
    let mut f = Fixture::new("Café 😃");
    f.edit(EditCommand::SelectAll);
    let r = f.edit(EditCommand::Cut);
    assert!(
        matches!(r.clipboard,Some(ClipboardRequest::Write{ref text,cut:true}) if text=="Café 😃")
    );
    // La suppression n'arrive qu'après la confirmation de réussite du presse-papiers natif.
    assert_eq!(f.state().value, "Café 😃");
    f.edit(EditCommand::DeleteSelection);
    assert_eq!(f.state().value, "");
    assert!(matches!(
        f.edit(EditCommand::Paste).clipboard,
        Some(ClipboardRequest::Read)
    ));
    f.text("a\nb\tc");
    assert_eq!(f.state().value, "a b c");
    f.key(Key::Tab, false);
    assert_eq!(f.ui.focused(), Some(ElementId(11)));
    f.text("MODIFIER");
    assert_eq!(
        f.ui.input_snapshot(ElementId(11)).unwrap().value,
        "Lecture seule"
    );
    assert!(f.edit(EditCommand::Paste).clipboard.is_none());
    f.edit(EditCommand::SelectAll);
    assert!(matches!(
        f.edit(EditCommand::Cut).clipboard,
        Some(ClipboardRequest::Write { cut: false, .. })
    ));
    f.key(Key::Tab, false);
    assert_eq!(f.ui.focused(), Some(ElementId(20)));
}

#[test]
fn undo_redo_limits_submission_and_caret_blink_reuse_layout() {
    let mut f = Fixture::new("");
    f.text("abc");
    f.text("é");
    f.edit(EditCommand::Undo);
    assert_eq!(f.state().value, "abc");
    f.edit(EditCommand::Redo);
    assert_eq!(f.state().value, "abcé");
    f.edit(EditCommand::SelectAll);
    f.text(&"😃".repeat(100));
    assert_eq!(f.state().value.chars().count(), 32);
    f.key(Key::Enter, false);
    assert_eq!(f.submits.get(), 1);
    f.event(UiEvent::KeyDown {
        key: Key::Enter,
        shift: false,
        repeat: true,
    });
    assert_eq!(f.submits.get(), 1);
    let stats = f.ui.stats();
    let calls = f.measure.calls;
    for _ in 0..10 {
        f.event(UiEvent::CaretTick);
    }
    assert_eq!(f.ui.stats().views, stats.views);
    assert_eq!(f.ui.stats().layouts, stats.layouts);
    assert_eq!(f.measure.calls, calls);
    f.key(Key::Escape, false);
    assert_eq!(f.ui.focused(), Some(ElementId(10)));
}

#[test]
fn composition_preview_is_not_committed_and_cancel_preserves_selection() {
    let mut f = Fixture::new("abc");
    f.edit(EditCommand::SelectAll);
    f.event(UiEvent::CompositionStart);
    f.event(UiEvent::CompositionUpdate {
        value: "にほん".into(),
        cursor_utf16: 2,
    });
    assert!(f.state().composing);
    assert_eq!(f.state().value, "abc");
    assert_eq!(f.changes.get(), 0);
    f.event(UiEvent::CompositionEnd);
    assert!(!f.state().composing);
    assert_eq!(f.state().selection, 0..3);
    f.event(UiEvent::CompositionStart);
    f.event(UiEvent::CompositionUpdate {
        value: "日本".into(),
        cursor_utf16: 2,
    });
    f.event(UiEvent::CompositionCommit("日本".into()));
    assert_eq!(&*f.value.borrow(), "日本");
    assert_eq!(f.changes.get(), 1);
    f.edit(EditCommand::Undo);
    assert_eq!(f.state().value, "abc");
}

#[test]
fn pointer_selection_shift_click_scroll_and_focus_loss_release_capture() {
    let mut f = Fixture::new("abcdefghijklmnopqrstuvwxyz");
    f.key(Key::Home, false);
    let start = f.state().caret;
    let p = PointF {
        x: start.left + 1.,
        y: start.top + 4.,
    };
    assert!(f.event(UiEvent::PointerDown(p)).capture);
    f.event(UiEvent::PointerMove(PointF {
        x: p.x + 45.,
        y: p.y,
    }));
    assert!(f.state().selection.end > 0);
    assert!(f.event(UiEvent::PointerUp(p)).release);
    f.key(Key::End, false);
    let end = f.state().caret;
    assert!(end.left < 600.);
    f.event(UiEvent::PointerDownExtended(p));
    assert!(f.state().selection.end > f.state().selection.start);
    assert!(f.event(UiEvent::FocusChanged(false)).release);
    assert!(!f.ui.has_capture());
    f.event(UiEvent::FocusChanged(true));
    f.ui.prepare(
        ViewContext {
            width: 100.,
            ..context()
        },
        ChromeState::default(),
        &mut f.measure,
    )
    .unwrap();
    // Le texte dépasse vraiment la largeur intérieure : le caret reste dans le viewport.
    key(&mut f.ui, Key::End);
    f.ui.prepare(
        ViewContext {
            width: 100.,
            ..context()
        },
        ChromeState::default(),
        &mut f.measure,
    )
    .unwrap();
    let caret = f.state();
    assert!(caret.caret.left >= caret.bounds.left && caret.caret.right <= caret.bounds.right + 0.1);
    assert!(f.ui.scene().validate().is_ok());
}

#[test]
fn word_navigation_and_style_validation() {
    let mut f = Fixture::new("hello café 😃");
    f.edit(EditCommand::WordLeft { extend: false });
    assert_eq!(f.state().selection, 12..12);
    f.edit(EditCommand::WordLeft { extend: true });
    assert_eq!(f.state().selection, 6..12);
    f.text("thé ");
    assert_eq!(f.state().value, "hello thé 😃");
    let bad = Element::custom(
        1,
        TextInput::new("", |text| Msg::Text(1, text)).style(InputStyle {
            font_size: f32::NAN,
            ..InputStyle::default()
        }),
    );
    assert!(crate::error::rejected(layout::validate(&bad)));
}
