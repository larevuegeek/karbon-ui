//! Champ multiligne avec le mesureur déterministe (0,5 em par caractère, lignes de
//! 1,4 em) : police de 10 px sans padding, 52 px de large (coupure à 50 px, soit
//! 10 caractères) et 42 px de haut (3 lignes de 14 px).
use super::*;

struct AreaApp {
    value: Rc<RefCell<String>>,
    changes: Rc<Cell<usize>>,
    read_only: bool,
}
impl Application for AreaApp {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        let mut input = TextInput::new(self.value.borrow().clone(), |text| Msg::Text(30, text))
            .multiline()
            .label("Notes")
            .placeholder("Écrire ici")
            .on_submit(Msg::Click(99))
            .style(InputStyle {
                font_size: 10.,
                padding: 0.,
                ..InputStyle::default()
            });
        if self.read_only {
            input = input.read_only();
        }
        Element::column(
            1,
            vec![
                Element::custom(30, input)
                    .width(Length::Px(52.))
                    .height(Length::Px(42.)),
                widgets::button(31, "Suite", Msg::Click(31)),
            ],
        )
    }
    fn update(&mut self, message: Msg) -> bool {
        match message {
            Msg::Text(30, value) => {
                *self.value.borrow_mut() = value;
                self.changes.set(self.changes.get() + 1);
                true
            }
            Msg::Click(99) => panic!("Entrée ne valide pas un champ multiligne"),
            _ => false,
        }
    }
}
struct Area {
    ui: UiRuntime,
    measure: Measure,
    value: Rc<RefCell<String>>,
    changes: Rc<Cell<usize>>,
}
impl Area {
    fn new(initial: &str) -> Self {
        Self::with(initial, false)
    }
    fn with(initial: &str, read_only: bool) -> Self {
        let value = Rc::new(RefCell::new(initial.to_owned()));
        let changes = Rc::new(Cell::new(0));
        let ui = UiRuntime::new(
            AreaApp {
                value: value.clone(),
                changes: changes.clone(),
                read_only,
            },
            &NoResources,
        )
        .unwrap();
        let mut area = Self {
            ui,
            measure: Measure::default(),
            value,
            changes,
        };
        area.prepare();
        area.key(Key::Tab, false);
        assert_eq!(area.ui.focused(), Some(ElementId(30)));
        area
    }
    fn prepare(&mut self) {
        prepare(&mut self.ui, &mut self.measure);
    }
    fn event(&mut self, event: UiEvent) -> EventResult {
        let result = self.ui.event(event);
        self.prepare();
        result
    }
    fn key(&mut self, key: Key, shift: bool) {
        self.event(UiEvent::KeyDown {
            key,
            shift,
            repeat: false,
        });
    }
    fn text(&mut self, value: &str) {
        self.event(UiEvent::Text(value.into()));
    }
    fn state(&self) -> crate::ui::text_input::InputSnapshot {
        self.ui.input_snapshot(ElementId(30)).unwrap()
    }
    fn cursor(&self) -> usize {
        self.state().selection.end
    }
    fn rect(&self) -> RectF {
        self.ui
            .root()
            .unwrap()
            .find(ElementId(30))
            .unwrap()
            .bounds()
    }
    fn text_info(&self) -> crate::ui::text_input::AccessibleText {
        self.ui
            .accessibility()
            .into_iter()
            .find(|n| n.id == ElementId(30))
            .and_then(|n| n.text)
            .unwrap()
    }
    fn lines(&self) -> Vec<std::ops::Range<usize>> {
        self.text_info()
            .lines
            .into_iter()
            .map(|l| l.range)
            .collect()
    }
}

#[test]
fn enter_inserts_line_breaks_and_text_wraps_at_the_field_width() {
    let mut a = Area::new("");
    a.text("hello");
    a.key(Key::Enter, false);
    a.text("world");
    assert_eq!(&*a.value.borrow(), "hello\nworld");
    assert_eq!(a.lines(), [0..5, 6..11]);
    let caret = a.state().caret;
    near(caret.top, a.rect().top + 14.);
    near(caret.left, a.rect().left + 25.);
    // Coupure automatique après l'espace : l'espace reste en fin de première ligne.
    a.ui.event(UiEvent::Edit(EditCommand::SelectAll));
    a.text("aaaa bbbb cccc");
    assert_eq!(a.lines(), [0..10, 10..14]);
    // Collage : CR LF et CR deviennent des retours, la tabulation une espace.
    a.text("\r\nx\ty\rz");
    assert_eq!(&*a.value.borrow(), "aaaa bbbb cccc\nx y\nz");
    a.ui.event(UiEvent::Edit(EditCommand::Undo));
    a.prepare();
    assert_eq!(a.state().value, "aaaa bbbb cccc");
    a.ui.event(UiEvent::Edit(EditCommand::Redo));
    a.prepare();
    assert_eq!(a.state().value, "aaaa bbbb cccc\nx y\nz");
    // Retour arrière au début d'une ligne : supprime le retour à la ligne.
    a.key(Key::Home, false);
    a.key(Key::Backspace, false);
    assert_eq!(a.state().value, "aaaa bbbb cccc\nx yz");
    // Tab garde la navigation au focus.
    a.key(Key::Tab, false);
    assert_eq!(a.ui.focused(), Some(ElementId(31)));
    assert_eq!(a.state().value, "aaaa bbbb cccc\nx yz");
}

#[test]
fn vertical_moves_keep_the_visual_column_and_line_ends_keep_their_affinity() {
    let mut a = Area::new("abcdefgh\nab\nabcdefgh");
    assert_eq!(a.cursor(), 20);
    a.key(Key::Up, false);
    assert_eq!(a.cursor(), 11);
    a.key(Key::Up, false);
    assert_eq!(a.cursor(), 8, "la colonne visée est gardée");
    a.key(Key::Down, true);
    a.key(Key::Down, true);
    assert_eq!(a.state().selection, 8..20);
    a.key(Key::Up, false);
    a.key(Key::Up, false);
    a.key(Key::Up, false);
    assert_eq!(a.cursor(), 0, "Haut sur la première ligne va au début");
    a.key(Key::End, false);
    assert_eq!(a.cursor(), 8);
    // Ligne coupée automatiquement : Fin garde le caret en fin de ligne.
    let mut a = Area::new("aaaa bbbb cccc");
    a.key(Key::Home, false);
    assert_eq!(a.cursor(), 10);
    a.key(Key::Up, false);
    assert_eq!(a.cursor(), 0);
    a.key(Key::End, false);
    assert_eq!(a.cursor(), 10);
    let caret = a.state().caret;
    near(caret.top, a.rect().top);
    near(caret.left, a.rect().left + 50.);
    a.key(Key::Right, false);
    a.key(Key::Left, false);
    near(a.state().caret.top, a.rect().top + 14.);
    a.key(Key::Down, false);
    assert_eq!(a.cursor(), 14);
    // Mots : les retours à la ligne sont des séparateurs.
    a.ui.event(UiEvent::Edit(EditCommand::WordLeft { extend: false }));
    a.prepare();
    assert_eq!(a.cursor(), 10);
}

#[test]
fn pointer_selects_across_lines_and_clipboard_keeps_line_breaks() {
    let mut a = Area::new("abc\ndefgh\nij");
    let r = a.rect();
    let point = |x: f32, y: f32| PointF {
        x: r.left + x,
        y: r.top + y,
    };
    assert!(a.event(UiEvent::PointerDown(point(11., 3.))).capture);
    assert_eq!(a.state().selection, 2..2);
    a.event(UiEvent::PointerMove(point(14., 17.)));
    assert_eq!(a.state().selection, 2..7);
    assert!(a.event(UiEvent::PointerUp(point(14., 17.))).release);
    let copied = a.event(UiEvent::Edit(EditCommand::Copy));
    assert!(
        matches!(copied.clipboard, Some(ClipboardRequest::Write { ref text, cut: false }) if text == "c\ndef")
    );
    // Clic sous la dernière ligne : fin du texte ; Maj+clic étend la sélection.
    a.event(UiEvent::PointerDown(point(40., 41.)));
    a.event(UiEvent::PointerUp(point(40., 41.)));
    assert_eq!(a.cursor(), 12);
    a.event(UiEvent::PointerDownExtended(point(0., 15.)));
    a.event(UiEvent::PointerUp(point(0., 15.)));
    assert_eq!(a.state().selection, 4..12);
    // La sélection peint un rectangle par ligne touchée.
    let fills = a
        .ui
        .scene()
        .commands
        .iter()
        .filter(|c| matches!(c, crate::ui::scene::DrawCommand::Fill { color, .. } if *color == Theme::DARK.pressed))
        .count();
    assert!(fills >= 2, "{fills}");
}

#[test]
fn double_and_triple_clicks_select_a_word_and_a_logical_line() {
    let mut a = Area::new("abc\nde fg\nij");
    let r = a.rect();
    let point = |x: f32, y: f32| PointF {
        x: r.left + x,
        y: r.top + y,
    };
    // Deuxième ligne (14 à 28 px), sur « de » : double clic = « de », triple = la ligne.
    for _ in 0..2 {
        a.event(UiEvent::PointerDown(point(2., 17.)));
        a.event(UiEvent::PointerUp(point(2., 17.)));
    }
    assert_eq!(a.state().selection, 4..6);
    a.event(UiEvent::PointerDown(point(2., 17.)));
    a.event(UiEvent::PointerUp(point(2., 17.)));
    assert_eq!(a.state().selection, 4..9);
    let copied = a.event(UiEvent::Edit(EditCommand::Copy));
    assert!(
        matches!(copied.clipboard, Some(ClipboardRequest::Write { ref text, .. }) if text == "de fg")
    );
}

#[test]
fn content_scrolls_vertically_and_keeps_the_caret_visible() {
    let mut a = Area::new("1\n2\n3\n4\n5\n6");
    let r = a.rect();
    // Valeur initiale : le haut du texte est affiché.
    near(a.text_info().lines[0].rect.top, r.top);
    a.text("7");
    let caret = a.state().caret;
    near(caret.top, r.top + 28.);
    near(a.text_info().lines[0].rect.top, r.top - 42.);
    // Molette sur le champ : son texte défile d'abord, sans dépasser ses bords.
    let centre = center(&a.ui, 30);
    assert!(wheel(&mut a.ui, centre, 0., -100.).handled);
    a.prepare();
    near(a.text_info().lines[0].rect.top, r.top);
    assert!(!wheel(&mut a.ui, centre, 0., -10.).handled);
    a.prepare();
    near(a.text_info().lines[0].rect.top, r.top);
    // Le défilement à la molette ne ramène pas le caret tant qu'il ne bouge pas.
    a.event(UiEvent::CaretTick);
    near(a.text_info().lines[0].rect.top, r.top);
    a.key(Key::Up, false);
    let caret = a.state().caret;
    assert!(caret.top >= r.top - 0.01 && caret.bottom <= r.bottom + 0.01);
    a.key(Key::PageUp, false);
    a.key(Key::PageUp, false);
    assert_eq!(a.cursor(), 0);
    near(a.text_info().lines[0].rect.top, r.top);
    // La colonne visée (fin de « 67 ») est gardée d'une page à l'autre.
    a.key(Key::PageDown, false);
    assert_eq!(a.cursor(), 7);
    // Le texte dépasse : un indicateur de défilement est peint au bord droit.
    let thumb = a.ui.scene().commands.iter().any(|c| {
        matches!(c, crate::ui::scene::DrawCommand::Fill { rect, color, .. }
            if color.alpha == 110 && layout::width(*rect) == 4. && rect.right <= r.right)
    });
    assert!(thumb);
    assert!(a.ui.scene().validate().is_ok());
}

#[test]
fn composition_read_only_and_accessibility_follow_the_single_line_field() {
    let mut a = Area::new("ab\ncd");
    a.event(UiEvent::CompositionStart);
    a.event(UiEvent::CompositionUpdate {
        value: "にほん".into(),
        cursor_utf16: 3,
    });
    assert!(a.state().composing);
    assert_eq!(a.changes.get(), 0);
    let text = a.text_info();
    assert_eq!(&*text.value, "ab\ncdにほん");
    assert_eq!(text.selection, 14..14);
    a.event(UiEvent::CompositionCommit("日本".into()));
    assert_eq!(&*a.value.borrow(), "ab\ncd日本");
    assert_eq!(a.changes.get(), 1);
    let text = a.text_info();
    assert!(text.multiline);
    assert_eq!(text.lines.len(), 2);
    assert_eq!(text.lines[1].range, 3..11);
    // Un octet par arrêt de caret, ligne par ligne, avec des clusters sur chaque ligne.
    assert!(text.carets.iter().any(|c| c.byte == 2));
    assert!(
        text.clusters
            .iter()
            .any(|c| c.start == 3 && c.rect.top > text.clusters[0].rect.top)
    );
    let node =
        a.ui.accessibility()
            .into_iter()
            .find(|n| n.id == ElementId(30))
            .unwrap();
    assert_eq!(node.role, Role::TextInput);
    assert_eq!(node.value.as_deref(), Some("ab\ncd日本"));
    assert_eq!(node.help, "Écrire ici");
    // L'accessibilité sélectionne une plage sur deux lignes.
    a.ui.accessibility_action(AccessibilityAction::SelectText {
        id: ElementId(30),
        range: 1..4,
        expected: "ab\ncd日本".into(),
    })
    .unwrap();
    a.prepare();
    assert_eq!(a.state().selection, 1..4);
    // Lecture seule : ni saisie ni retour à la ligne, la sélection reste possible.
    let mut r = Area::with("x\ny", true);
    r.key(Key::Enter, false);
    r.text("z");
    assert_eq!(r.state().value, "x\ny");
    r.key(Key::Up, true);
    assert_eq!(r.state().selection, 1..3);
}
