//! Entrées brutes : `on_pointer` (les trois boutons, le geste gardé par l'élément qui a
//! pris l'appui, le passage aux ancêtres) et `Application::key` (toutes les touches, sauf
//! quand un champ de saisie a le clavier).
use crate::ui::{runtime::raw_input::PointerInput, tests::support::*, widgets::TextInput};
use std::cell::RefCell;

/// Messages reçus : (élément, phase, bouton, position dans l'élément).
type Log = Rc<RefCell<Vec<(u64, PointerPhase, Option<PointerButton>, f32, f32)>>>;

/// Une colonne 1 qui prend tout ce qui lui arrive, et deux blocs de 100 × 100 : le bloc 2
/// prend les appuis du bouton droit et la suite de leurs gestes, le bloc 3 ne prend rien.
struct Pointers {
    log: Log,
}
impl Application for Pointers {
    type Message = (u64, PointerPhase, Option<PointerButton>, f32, f32);
    fn view(&self, _: &ViewContext) -> Element {
        let record = |id: u64| move |e: PointerEvent| (id, e.phase, e.button, e.local_x, e.local_y);
        let taker = box_node(2, Length::Px(100.), Length::Px(100.)).on_pointer(move |e| {
            let right = e.button == Some(PointerButton::Secondary);
            (right || e.phase != PointerPhase::Down).then(|| record(2)(e))
        });
        let ignorer =
            box_node(3, Length::Px(100.), Length::Px(100.)).on_pointer(|_| None::<Self::Message>);
        Element::column(1, vec![taker, ignorer]).on_pointer(move |e| Some(record(1)(e)))
    }
    fn update(&mut self, message: Self::Message) -> bool {
        self.log.borrow_mut().push(message);
        false
    }
}

fn pointers() -> (UiRuntime, Log) {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut ui = UiRuntime::new(Pointers { log: log.clone() }, &NoResources).unwrap();
    prepare(&mut ui, &mut Measure::default());
    (ui, log)
}

fn input(phase: PointerPhase, button: Option<PointerButton>, position: PointF) -> PointerInput {
    PointerInput {
        phase,
        button,
        position,
        shift: false,
        control: false,
        alt: false,
    }
}

#[test]
fn an_element_that_takes_a_press_keeps_the_gesture_until_the_last_button_is_released() {
    let (mut ui, log) = pointers();
    let taker = center(&ui, 2);
    let right = Some(PointerButton::Secondary);
    // Le bloc 2 prend l'appui du bouton droit, avec la position dans le bloc.
    let result = ui.pointer_input(input(PointerPhase::Down, right, taker));
    assert!(result.handled && ui.pointer_gesture());
    assert_eq!(log.borrow()[0], (2, PointerPhase::Down, right, 50., 50.));
    // Hors de lui, sur le bloc 3 et plus loin, la suite du geste lui revient.
    let far = PointF {
        x: taker.x + 400.,
        y: taker.y + 150.,
    };
    ui.pointer_input(input(PointerPhase::Move, None, far));
    assert_eq!(log.borrow()[1], (2, PointerPhase::Move, None, 450., 200.));
    ui.pointer_input(input(PointerPhase::Up, right, far));
    assert_eq!(log.borrow()[2].1, PointerPhase::Up);
    assert!(!ui.pointer_gesture());
    // Le geste fini, un mouvement va à ce qui est sous le pointeur : le bloc 3 laisse
    // passer, la colonne prend.
    ui.pointer_input(input(PointerPhase::Move, None, center(&ui, 3)));
    assert_eq!(log.borrow()[3].0, 1);
}

#[test]
fn a_press_the_element_does_not_take_goes_to_its_ancestors() {
    let (mut ui, log) = pointers();
    // Le bloc 2 ne prend pas le bouton gauche : la colonne le prend, et garde le geste.
    let left = Some(PointerButton::Primary);
    ui.pointer_input(input(PointerPhase::Down, left, center(&ui, 2)));
    assert_eq!(log.borrow()[0].0, 1);
    ui.pointer_input(input(PointerPhase::Move, None, center(&ui, 2)));
    assert_eq!(log.borrow()[1].0, 1);
    // Deux boutons : le geste dure jusqu'au dernier relâché.
    let middle = Some(PointerButton::Middle);
    ui.pointer_input(input(PointerPhase::Down, middle, center(&ui, 2)));
    ui.pointer_input(input(PointerPhase::Up, left, center(&ui, 2)));
    assert!(ui.pointer_gesture());
    ui.pointer_input(input(PointerPhase::Up, middle, center(&ui, 2)));
    assert!(!ui.pointer_gesture());
    // Sans gestionnaire nulle part, rien n'est consommé.
    let mut plain = view_runtime(|| box_node(1, Length::Px(100.), Length::Px(100.)));
    prepare(&mut plain, &mut Measure::default());
    let result = plain.pointer_input(input(PointerPhase::Down, left, center(&plain, 1)));
    assert!(!result.handled && !plain.pointer_gesture());
}

/// Une application qui note les touches reçues, avec un champ de saisie 2.
struct Keys {
    log: Rc<RefCell<Vec<KeyPress>>>,
}
impl Application for Keys {
    type Message = String;
    fn view(&self, _: &ViewContext) -> Element {
        Element::column(
            1,
            vec![Element::custom(2, TextInput::new("", |text: String| text))],
        )
    }
    fn update(&mut self, _: String) -> bool {
        false
    }
    fn key(&mut self, press: KeyPress) -> bool {
        self.log.borrow_mut().push(press);
        press.key == KeyCode::R
    }
}

#[test]
fn every_key_goes_to_the_application_unless_a_text_field_has_the_keyboard() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut ui = UiRuntime::new(Keys { log: log.clone() }, &NoResources).unwrap();
    prepare(&mut ui, &mut Measure::default());
    let ctrl_z = KeyPress {
        control: true,
        ..KeyPress::new(KeyCode::Z, true)
    };
    assert!(!ui.key_input(ctrl_z).repaint);
    // Une touche qui fait reconstruire la vue demande une nouvelle image.
    assert!(ui.key_input(KeyPress::new(KeyCode::R, true)).repaint);
    assert_eq!(*log.borrow(), [ctrl_z, KeyPress::new(KeyCode::R, true)]);
    // Le champ a le clavier : ce qu'on tape est à lui.
    key(&mut ui, Key::Tab);
    prepare(&mut ui, &mut Measure::default());
    assert_eq!(ui.focused(), Some(ElementId(2)));
    ui.key_input(KeyPress::new(KeyCode::A, true));
    assert_eq!(log.borrow().len(), 2);
}

#[test]
fn an_application_that_ignores_keys_sees_nothing_change() {
    let mut ui = view_runtime(|| box_node(1, Length::Px(10.), Length::Px(10.)));
    prepare(&mut ui, &mut Measure::default());
    let result = ui.key_input(KeyPress::new(KeyCode::A, true));
    assert!(!result.repaint && !result.handled);
}
