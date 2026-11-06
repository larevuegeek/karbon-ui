//! Menu contextuel intégré du champ : ouverture au clic droit et au clavier, entrées
//! disponibles selon l'état du champ, choix au pointeur et au clavier, fermetures, et
//! priorité d'un gestionnaire applicatif.
use super::*;
use crate::ui::scene::DrawCommand;

/// Centre de la rangée `index` du menu ouvert, et sa disponibilité.
fn row(f: &Fixture, index: usize) -> (PointF, bool) {
    let (rect, enabled) = f.ui.edit_menu_rows()[index];
    (
        PointF {
            x: (rect.left + rect.right) / 2.,
            y: (rect.top + rect.bottom) / 2.,
        },
        enabled,
    )
}
fn open(f: &Fixture) -> bool {
    !f.ui.edit_menu_rows().is_empty()
}
fn available(f: &Fixture) -> [bool; 4] {
    let rows = f.ui.edit_menu_rows();
    [rows[0].1, rows[1].1, rows[2].1, rows[3].1]
}
fn drawn_labels(f: &Fixture) -> Vec<String> {
    f.ui.scene()
        .commands
        .iter()
        .filter_map(|c| match c {
            DrawCommand::Text { text, .. } => Some(text.to_string()),
            _ => None,
        })
        .collect()
}
fn press(f: &mut Fixture, key: Key) -> EventResult {
    f.event(UiEvent::KeyDown {
        key,
        shift: false,
        repeat: false,
    })
}

#[test]
fn right_click_in_a_field_opens_the_menu_and_paste_asks_the_clipboard() {
    let mut f = Fixture::new("abc");
    // Focus sur le bouton : le clic droit le ramène au champ visé.
    f.key(Key::Tab, false);
    f.key(Key::Tab, false);
    assert_eq!(f.ui.focused(), Some(ElementId(20)));
    let p = center(&f.ui, 10);
    let r = f.event(UiEvent::ContextMenu(Some(p)));
    assert!(r.handled && r.repaint);
    assert_eq!(f.ui.focused(), Some(ElementId(10)));
    assert!(open(&f));
    assert!(f.ui.scene().validate().is_ok());
    let labels = drawn_labels(&f);
    for label in ["Cut", "Copy", "Paste", "Select all"] {
        assert!(labels.iter().any(|l| l == label), "{label} absent");
    }
    // Sans sélection : couper et copier indisponibles ; coller et tout sélectionner
    // possibles. Le menu est posé sous le pointeur.
    assert_eq!(available(&f), [false, false, true, true]);
    let (paste, _) = row(&f, 2);
    assert!(paste.y > p.y);
    // Une entrée grisée ne répond pas.
    let (cut, _) = row(&f, 0);
    f.event(UiEvent::PointerDown(cut));
    assert!(open(&f));
    f.event(UiEvent::PointerUp(cut));
    assert!(open(&f));
    // Coller : le champ demande le presse-papiers au backend, le menu se ferme.
    assert!(f.event(UiEvent::PointerDown(paste)).capture);
    let up = f.event(UiEvent::PointerUp(paste));
    assert!(matches!(up.clipboard, Some(ClipboardRequest::Read)));
    assert!(up.release);
    assert!(!open(&f));
    assert!(!f.ui.has_capture());
    assert_eq!(f.ui.focused(), Some(ElementId(10)));
}

#[test]
fn keyboard_opens_below_the_caret_and_arrows_choose_an_entry() {
    let mut f = Fixture::new("abc");
    let caret = f.state().caret;
    assert!(f.event(UiEvent::ContextMenu(None)).handled);
    assert!(open(&f));
    let (first, _) = row(&f, 0);
    assert!(first.y >= caret.bottom);
    // Première entrée possible active (Coller) ; Bas → Tout sélectionner ; Entrée.
    press(&mut f, Key::Down);
    assert!(press(&mut f, Key::Enter).clipboard.is_none());
    assert!(!open(&f));
    assert_eq!(f.state().selection, 0..3);
    // Avec une sélection : couper et copier possibles, tout sélectionner inutile ;
    // Entrée sur la première entrée coupe.
    f.event(UiEvent::ContextMenu(None));
    assert_eq!(available(&f), [true, true, true, false]);
    let r = press(&mut f, Key::Enter);
    assert!(matches!(
        r.clipboard,
        Some(ClipboardRequest::Write { cut: true, ref text }) if text == "abc"
    ));
    assert!(!open(&f));
    // Haut depuis la première entrée revient à la dernière possible ; Fin et Début.
    f.event(UiEvent::ContextMenu(None));
    press(&mut f, Key::Up);
    let r = press(&mut f, Key::Enter);
    assert!(matches!(r.clipboard, Some(ClipboardRequest::Read)));
}

#[test]
fn escape_closes_and_an_outside_click_is_swallowed_once() {
    let mut f = Fixture::new("abc");
    let p = center(&f.ui, 10);
    f.event(UiEvent::ContextMenu(Some(p)));
    assert!(open(&f));
    assert!(press(&mut f, Key::Escape).handled);
    assert!(!open(&f));
    // Un second clic droit sur un menu ouvert le ferme.
    f.event(UiEvent::ContextMenu(Some(p)));
    f.event(UiEvent::ContextMenu(Some(p)));
    assert!(!open(&f));
    // Un clic hors du menu (au-dessus du champ : le menu, posé sous le pointeur, couvre
    // les champs du dessous) le ferme et retient le pointeur jusqu'au relâchement, sans
    // atteindre la page.
    f.event(UiEvent::ContextMenu(Some(p)));
    let outside = PointF {
        x: p.x,
        y: p.y - 30.,
    };
    assert!(f.ui.edit_menu_rows().iter().all(|(r, _)| r.top > outside.y));
    let down = f.event(UiEvent::PointerDown(outside));
    assert!(down.handled && down.capture);
    assert!(!open(&f));
    assert!(f.ui.has_capture());
    assert!(f.event(UiEvent::PointerUp(outside)).release);
    assert_eq!(f.ui.focused(), Some(ElementId(10)));
    assert!(!f.ui.has_capture());
    // Le menu fermé, un clic atteint de nouveau le champ du dessous.
    let other = center(&f.ui, 11);
    f.event(UiEvent::PointerDown(other));
    f.event(UiEvent::PointerUp(other));
    assert_eq!(f.ui.focused(), Some(ElementId(11)));
    // Perte du focus de la fenêtre ou Tab : le menu se ferme, l'événement continue.
    f.event(UiEvent::ContextMenu(None));
    assert!(open(&f));
    f.event(UiEvent::FocusChanged(false));
    assert!(!open(&f));
    f.event(UiEvent::FocusChanged(true));
    f.event(UiEvent::ContextMenu(None));
    f.key(Key::Tab, false);
    assert!(!open(&f));
    assert_eq!(f.ui.focused(), Some(ElementId(20)));
}

#[test]
fn read_only_and_disabled_fields_limit_the_entries() {
    let mut f = Fixture::new("abc");
    // Champ désactivé : rien ne s'ouvre, l'événement n'est pas pris.
    let r = f.event(UiEvent::ContextMenu(Some(center(&f.ui, 12))));
    assert!(!r.handled);
    assert!(!open(&f));
    // Lecture seule : ni couper ni coller ; tout sélectionner puis copier.
    f.key(Key::Tab, false);
    assert_eq!(f.ui.focused(), Some(ElementId(11)));
    f.event(UiEvent::ContextMenu(None));
    assert_eq!(available(&f), [false, false, false, true]);
    press(&mut f, Key::Enter);
    assert_eq!(
        f.ui.input_snapshot(ElementId(11)).unwrap().selection,
        0.."Lecture seule".len()
    );
    f.event(UiEvent::ContextMenu(None));
    assert_eq!(available(&f), [false, true, false, false]);
    let r = press(&mut f, Key::Enter);
    assert!(matches!(
        r.clipboard,
        Some(ClipboardRequest::Write { cut: false, .. })
    ));
}

#[test]
fn an_application_handler_replaces_the_built_in_menu() {
    let mut ui = view_runtime(|| {
        Element::column(
            1,
            vec![
                Element::custom(10, TextInput::new("x", |t| Msg::Text(1, t)))
                    .context_menu(|request| Msg::Menu(10, request)),
            ],
        )
    });
    let mut measure = Measure::default();
    prepare(&mut ui, &mut measure);
    let p = center(&ui, 10);
    let r = ui.event(UiEvent::ContextMenu(Some(p)));
    prepare(&mut ui, &mut measure);
    assert!(r.handled);
    assert!(ui.edit_menu_rows().is_empty());
}
