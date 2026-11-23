//! Ajouts aux entrées brutes : le clavier adressé au panneau qui a le focus (`on_key`),
//! le pointeur verrouillé (`pointer_lock`) et le mouvement relatif (`delta`).
use crate::ui::{runtime::raw_input::PointerInput, tests::support::*, widgets::TextInput};
use std::cell::RefCell;

/// Ce que l'application reçoit.
#[derive(Clone, Debug, PartialEq)]
enum Got {
    /// Une touche prise par un panneau (1 : la vue, 2 : l'arbre).
    Panel(u64, KeyCode),
    /// Une touche arrivée à `Application::key`.
    App(KeyCode),
    /// Un événement du pointeur sur la vue : phase, bouton, écart.
    Pointer(PointerPhase, Option<PointerButton>, (f32, f32)),
    Text,
}

type Log = Rc<RefCell<Vec<Got>>>;

/// Deux panneaux côte à côte : la vue (1) prend Suppr et les gestes du bouton droit au
/// pointeur verrouillé ; l'arbre (2) prend Suppr et F2, et contient un champ (3) et un
/// bloc (4). Les autres touches remontent à l'application.
struct Panels {
    log: Log,
    lock: bool,
}
impl Application for Panels {
    type Message = Got;
    fn view(&self, _: &ViewContext) -> Element {
        let view = box_node(1, Length::Px(200.), Length::Px(200.))
            .on_key(|press: KeyPress| {
                (press.pressed && press.key == KeyCode::Delete).then_some(Got::Panel(1, press.key))
            })
            .on_pointer(|e: PointerEvent| {
                let right = e.button == Some(PointerButton::Secondary);
                (right || e.phase == PointerPhase::Move)
                    .then_some(Got::Pointer(e.phase, e.button, e.delta))
            })
            .pointer_lock(self.lock);
        let field = Element::custom(3, TextInput::new("", |_: String| Got::Text));
        let inner = box_node(4, Length::Px(100.), Length::Px(50.));
        let tree = Element::column(2, vec![field, inner]).on_key(|press: KeyPress| {
            let taken = press.pressed && matches!(press.key, KeyCode::Delete | KeyCode::F2);
            taken.then_some(Got::Panel(2, press.key))
        });
        Element::row(10, vec![view, tree])
    }
    fn update(&mut self, message: Got) -> bool {
        self.log.borrow_mut().push(message);
        false
    }
    fn key(&mut self, press: KeyPress) -> bool {
        if press.pressed {
            self.log.borrow_mut().push(Got::App(press.key));
        }
        false
    }
}

fn panels(lock: bool) -> (UiRuntime, Log) {
    let log = Rc::new(RefCell::new(Vec::new()));
    let app = Panels {
        log: log.clone(),
        lock,
    };
    let mut ui = UiRuntime::new(app, &NoResources).unwrap();
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
        meta: false,
        motion: None,
    }
}

fn press(ui: &mut UiRuntime, code: KeyCode) {
    ui.key_input(KeyPress::new(code, true));
    prepare(ui, &mut Measure::default());
}

#[test]
fn a_key_goes_to_the_panel_last_pressed_then_up_to_the_application() {
    let (mut ui, log) = panels(false);
    // Rien d'appuyé : Suppr va à l'application, comme avant.
    press(&mut ui, KeyCode::Delete);
    assert_eq!(log.borrow().last(), Some(&Got::App(KeyCode::Delete)));
    // Un appui dans la vue : Suppr est à elle, et l'application ne la voit pas.
    let left = Some(PointerButton::Primary);
    ui.pointer_input(input(PointerPhase::Down, left, center(&ui, 1)));
    ui.pointer_input(input(PointerPhase::Up, left, center(&ui, 1)));
    press(&mut ui, KeyCode::Delete);
    assert_eq!(log.borrow().last(), Some(&Got::Panel(1, KeyCode::Delete)));
    // F2 n'est pas à la vue : elle remonte à l'application.
    press(&mut ui, KeyCode::F2);
    assert_eq!(log.borrow().last(), Some(&Got::App(KeyCode::F2)));
    // Un appui dans un bloc de l'arbre : la touche part de lui et remonte à l'arbre.
    ui.pointer_input(input(PointerPhase::Down, left, center(&ui, 4)));
    press(&mut ui, KeyCode::Delete);
    assert_eq!(log.borrow().last(), Some(&Got::Panel(2, KeyCode::Delete)));
    press(&mut ui, KeyCode::F2);
    assert_eq!(log.borrow().last(), Some(&Got::Panel(2, KeyCode::F2)));
    // Une touche relâchée n'est prise par personne ici : elle arrive à l'application,
    // qui n'en note que les appuis.
    let before = log.borrow().len();
    ui.key_input(KeyPress::new(KeyCode::Delete, false));
    assert_eq!(log.borrow().len(), before);
}

#[test]
fn the_keyboard_focus_moved_by_tab_says_where_keys_go_and_a_field_keeps_its_keys() {
    let (mut ui, log) = panels(false);
    let left = Some(PointerButton::Primary);
    ui.pointer_input(input(PointerPhase::Down, left, center(&ui, 1)));
    // Tab met le champ de l'arbre au clavier : ce qu'on tape est à lui, aucun panneau
    // ni l'application ne le voit.
    key(&mut ui, Key::Tab);
    prepare(&mut ui, &mut Measure::default());
    assert_eq!(ui.focused(), Some(ElementId(3)));
    let before = log.borrow().len();
    press(&mut ui, KeyCode::Delete);
    assert_eq!(log.borrow().len(), before);
}

#[test]
fn without_on_key_nothing_changes() {
    let log = Rc::new(RefCell::new(Vec::new()));
    // Témoin : une vue sans `on_key`, appuyée, laisse tout à l'application.
    struct Plain(Log);
    impl Application for Plain {
        type Message = Got;
        fn view(&self, _: &ViewContext) -> Element {
            box_node(1, Length::Px(100.), Length::Px(100.)).on_pointer(|_| Some(Got::Text))
        }
        fn update(&mut self, _: Got) -> bool {
            false
        }
        fn key(&mut self, press: KeyPress) -> bool {
            self.0.borrow_mut().push(Got::App(press.key));
            false
        }
    }
    let mut ui = UiRuntime::new(Plain(log.clone()), &NoResources).unwrap();
    prepare(&mut ui, &mut Measure::default());
    let left = Some(PointerButton::Primary);
    ui.pointer_input(input(PointerPhase::Down, left, center(&ui, 1)));
    let result = ui.key_input(KeyPress::new(KeyCode::Delete, true));
    assert!(!result.handled);
    assert_eq!(*log.borrow(), [Got::App(KeyCode::Delete)]);
}

#[test]
fn moves_carry_their_delta_and_a_locked_gesture_takes_the_mouse_motion() {
    let (mut ui, log) = panels(true);
    let at = center(&ui, 1);
    let right = Some(PointerButton::Secondary);
    // Sans geste : l'écart des positions.
    ui.pointer_input(input(PointerPhase::Move, None, at));
    let moved = PointF {
        x: at.x + 7.,
        y: at.y - 3.,
    };
    ui.pointer_input(input(PointerPhase::Move, None, moved));
    assert_eq!(
        log.borrow().last(),
        Some(&Got::Pointer(PointerPhase::Move, None, (7., -3.)))
    );
    assert!(!ui.pointer_lock_wanted());
    // L'appui du bouton droit pris par la vue verrouille le pointeur.
    ui.pointer_input(input(PointerPhase::Down, right, at));
    assert!(ui.pointer_lock_wanted());
    // Verrouillé, la position ne bouge pas et l'écart est celui de la souris : 50 tours
    // de 40 pixels, bien plus que l'écran, sans butée.
    for _ in 0..50 {
        let locked = PointerInput {
            motion: Some((40., 0.)),
            ..input(PointerPhase::Move, None, at)
        };
        ui.pointer_input(locked);
    }
    let turned: f32 = log
        .borrow()
        .iter()
        .filter_map(|got| match got {
            Got::Pointer(PointerPhase::Move, None, (dx, _)) if *dx == 40. => Some(*dx),
            _ => None,
        })
        .sum();
    assert_eq!(turned, 2000.);
    ui.pointer_input(input(PointerPhase::Up, right, at));
    assert!(!ui.pointer_lock_wanted());
}

#[test]
fn losing_the_lock_releases_the_held_buttons_and_ends_the_gesture() {
    let (mut ui, log) = panels(true);
    let at = center(&ui, 1);
    let right = Some(PointerButton::Secondary);
    ui.pointer_input(input(PointerPhase::Down, right, at));
    assert!(ui.pointer_gesture());
    let result = ui.pointer_lock_lost();
    assert!(result.handled && !ui.pointer_gesture() && !ui.pointer_lock_wanted());
    assert_eq!(
        log.borrow().last(),
        Some(&Got::Pointer(PointerPhase::Up, right, (0., 0.)))
    );
    // Témoin : sans verrou, la perte ne touche pas au geste en cours.
    let (mut ui, _) = panels(false);
    ui.pointer_input(input(PointerPhase::Down, right, center(&ui, 1)));
    assert!(!ui.pointer_lock_lost().handled);
    assert!(ui.pointer_gesture());
}
