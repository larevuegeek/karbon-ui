//! Cases à cocher, interrupteurs et boutons radio : état contrôlé, clavier,
//! accessibilité et dessin.
use crate::ui::choices::{Checkbox, ChoiceStyle, RadioButton, Switch, radio_group};
use crate::ui::tests::support::*;
use crate::ui::theme::Color;
use std::cell::RefCell;

struct Values {
    checked: CheckState,
    switched: bool,
    mixed: CheckState,
    quality: usize,
    other: Option<usize>,
    enabled: bool,
    actions: Vec<Msg>,
}
struct App(Rc<RefCell<Values>>);
impl Application for App {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        let s = self.0.borrow();
        let check = Checkbox::new("Case", false, |v| Msg::Toggle(1, v)).state(s.checked);
        Element::column(
            1,
            vec![
                Element::custom(10, if s.enabled { check } else { check.disabled() }),
                Element::custom(11, Switch::new("Switch", s.switched, |v| Msg::Toggle(2, v))),
                Element::custom(
                    12,
                    Checkbox::new("Partiel", false, |v| Msg::Toggle(3, v)).state(s.mixed),
                ),
                Element::custom(
                    13,
                    Checkbox::new("Inactif", true, |v| Msg::Toggle(4, v)).disabled(),
                ),
                widgets::button(14, "Avant", Msg::Click(5)),
                radio_group(
                    20,
                    "Qualité",
                    vec![
                        Element::custom(
                            21,
                            RadioButton::new("Originale", s.quality == 0, Msg::Click(6)),
                        ),
                        Element::custom(
                            22,
                            RadioButton::new("Inactif", false, Msg::Click(7)).disabled(),
                        ),
                        Element::custom(
                            23,
                            RadioButton::new("Économique", s.quality == 1, Msg::Click(8)),
                        ),
                    ],
                ),
                radio_group(
                    30,
                    "Autre groupe",
                    vec![
                        Element::custom(
                            31,
                            RadioButton::new("Premier", s.other == Some(0), Msg::Click(9)),
                        ),
                        Element::custom(
                            32,
                            RadioButton::new("Second", s.other == Some(1), Msg::Click(10)),
                        ),
                    ],
                ),
                widgets::button(40, "Après", Msg::Click(11)),
            ],
        )
    }
    fn update(&mut self, message: Msg) -> bool {
        let mut s = self.0.borrow_mut();
        s.actions.push(message.clone());
        let state = |on| if on { CheckState::On } else { CheckState::Off };
        match message {
            Msg::Toggle(1, on) => s.checked = state(on),
            Msg::Toggle(2, on) => s.switched = on,
            Msg::Toggle(3, on) => s.mixed = state(on),
            Msg::Click(6) => s.quality = 0,
            Msg::Click(8) => s.quality = 1,
            Msg::Click(9) => s.other = Some(0),
            Msg::Click(10) => s.other = Some(1),
            _ => {}
        }
        true
    }
}
fn setup() -> (UiRuntime, Rc<RefCell<Values>>, Measure) {
    let values = Rc::new(RefCell::new(Values {
        checked: CheckState::Off,
        switched: false,
        mixed: CheckState::Mixed,
        quality: 0,
        other: None,
        enabled: true,
        actions: Vec::new(),
    }));
    let mut ui = UiRuntime::new(App(values.clone()), &NoResources).unwrap();
    let mut m = Measure::default();
    prepare(&mut ui, &mut m);
    (ui, values, m)
}
fn click(ui: &mut UiRuntime, id: u64, m: &mut Measure) {
    let p = center(ui, id);
    assert!(ui.event(UiEvent::PointerDown(p)).capture);
    assert!(ui.event(UiEvent::PointerUp(p)).release);
    prepare(ui, m);
}
fn press(ui: &mut UiRuntime, key: Key, shift: bool, m: &mut Measure) {
    ui.event(UiEvent::KeyDown {
        key,
        shift,
        repeat: false,
    });
    prepare(ui, m);
}
#[test]
fn choices_share_capture_cancellation_keyboard_and_controlled_state() {
    let (mut ui, values, mut m) = setup();
    let p = center(&ui, 10);
    ui.event(UiEvent::PointerDown(p));
    ui.event(UiEvent::PointerUp(PointF { x: -1., y: -1. }));
    prepare(&mut ui, &mut m);
    assert!(values.borrow().actions.is_empty());
    click(&mut ui, 10, &mut m);
    assert_eq!(values.borrow().checked, CheckState::On);
    assert_eq!(values.borrow().actions, [Msg::Toggle(1, true)]);
    press(&mut ui, Key::Space, false, &mut m);
    ui.event(UiEvent::KeyDown {
        key: Key::Space,
        shift: false,
        repeat: true,
    });
    assert_eq!(values.borrow().checked, CheckState::On);
    ui.event(UiEvent::KeyUp(Key::Space));
    prepare(&mut ui, &mut m);
    assert_eq!(values.borrow().checked, CheckState::Off);
    click(&mut ui, 11, &mut m);
    assert!(values.borrow().switched);
    click(&mut ui, 12, &mut m);
    assert_eq!(values.borrow().mixed, CheckState::On);
    click(&mut ui, 12, &mut m);
    assert_eq!(values.borrow().mixed, CheckState::Off);
    // Valeur suivante transmise : Off après la case, On pour l'interrupteur, Mixed devient On.
    assert_eq!(
        values.borrow().actions[1..],
        [
            Msg::Toggle(1, false),
            Msg::Toggle(2, true),
            Msg::Toggle(3, true),
            Msg::Toggle(3, false)
        ]
    );
    let count = values.borrow().actions.len();
    let p = center(&ui, 13);
    assert!(!ui.event(UiEvent::PointerDown(p)).capture);
    ui.event(UiEvent::PointerUp(p));
    assert_eq!(values.borrow().actions.len(), count);
    let p = center(&ui, 10);
    ui.event(UiEvent::PointerDown(p));
    values.borrow_mut().enabled = false;
    ui.invalidate();
    prepare(&mut ui, &mut m);
    assert!(ui.event(UiEvent::PointerUp(p)).release);
    assert_eq!(values.borrow().actions.len(), count);
}

#[test]
fn radio_groups_have_one_tab_stop_and_arrow_navigation_skips_disabled_members() {
    use crate::ui::runtime::AccessibilityAction as A;
    let (mut ui, values, mut m) = setup();
    ui.accessibility_action(A::Focus(ElementId(14))).unwrap();
    prepare(&mut ui, &mut m);
    press(&mut ui, Key::Tab, false, &mut m);
    assert_eq!(ui.focused(), Some(ElementId(21)));
    press(&mut ui, Key::Right, false, &mut m);
    assert_eq!(ui.focused(), Some(ElementId(23)));
    assert_eq!(values.borrow().quality, 1);
    press(&mut ui, Key::Right, false, &mut m);
    assert_eq!(ui.focused(), Some(ElementId(21)));
    press(&mut ui, Key::End, false, &mut m);
    assert_eq!(ui.focused(), Some(ElementId(23)));
    press(&mut ui, Key::Home, false, &mut m);
    assert_eq!(ui.focused(), Some(ElementId(21)));
    let count = values.borrow().actions.len();
    press(&mut ui, Key::Enter, false, &mut m);
    ui.event(UiEvent::KeyUp(Key::Space));
    prepare(&mut ui, &mut m);
    assert_eq!(values.borrow().actions.len(), count);
    press(&mut ui, Key::Tab, false, &mut m);
    assert_eq!(ui.focused(), Some(ElementId(31)));
    press(&mut ui, Key::Left, false, &mut m);
    assert_eq!(ui.focused(), Some(ElementId(32)));
    assert_eq!(values.borrow().other, Some(1));
    assert_eq!(values.borrow().quality, 0);
    press(&mut ui, Key::Tab, false, &mut m);
    assert_eq!(ui.focused(), Some(ElementId(40)));
    press(&mut ui, Key::Tab, true, &mut m);
    assert_eq!(ui.focused(), Some(ElementId(32)));
    ui.accessibility_action(A::Focus(ElementId(23))).unwrap();
    prepare(&mut ui, &mut m);
    press(&mut ui, Key::Tab, false, &mut m);
    assert_eq!(ui.focused(), Some(ElementId(32)));
}

#[test]
fn choice_accessibility_uses_toggle_selection_and_explicit_containers() {
    use crate::ui::runtime::{AccessibilityAction as A, AccessibilityError as E};
    let (mut ui, values, mut m) = setup();
    ui.accessibility_action(A::Focus(ElementId(14))).unwrap();
    prepare(&mut ui, &mut m);
    ui.accessibility_action(A::Toggle(ElementId(10))).unwrap();
    ui.accessibility_action(A::SelectChoice(ElementId(23)))
        .unwrap();
    prepare(&mut ui, &mut m);
    assert_eq!(ui.focused(), Some(ElementId(14)));
    assert_eq!(values.borrow().quality, 1);
    let snapshot = ui.accessibility();
    let check = snapshot.iter().find(|n| n.id == ElementId(10)).unwrap();
    assert_eq!(check.choice, Some(ChoiceState::Toggle(CheckState::On)));
    assert!(!check.invokable);
    let radio = snapshot.iter().find(|n| n.id == ElementId(23)).unwrap();
    assert_eq!(radio.choice, Some(ChoiceState::Radio(true)));
    assert_eq!(radio.selection_container, Some(ElementId(20)));
    assert_eq!(
        ui.accessibility_action(A::Invoke(ElementId(10))),
        Err(E::Unsupported)
    );
    assert_eq!(
        ui.accessibility_action(A::Toggle(ElementId(23))),
        Err(E::Unsupported)
    );
    assert_eq!(
        ui.accessibility_action(A::SelectChoice(ElementId(22))),
        Err(E::Disabled)
    );
    let count = values.borrow().actions.len();
    ui.accessibility_action(A::SelectChoice(ElementId(23)))
        .unwrap();
    assert_eq!(values.borrow().actions.len(), count);
}

#[test]
fn radio_validation_rejects_orphans_and_multiple_selected_but_accepts_nested_groups() {
    let radio = || Element::custom(2, RadioButton::new("Seul", true, Msg::Click(1)));
    assert!(crate::error::rejected(layout::validate(&radio())));
    assert!(crate::error::rejected(layout::validate(&radio_group(
        1,
        "Groupe",
        vec![
            radio(),
            Element::custom(3, RadioButton::new("Autre", true, Msg::Click(2)))
        ]
    ))));
    assert!(
        layout::validate(&radio_group(
            1,
            "Groupe",
            vec![
                radio(),
                radio_group(
                    4,
                    "Imbriqué",
                    vec![Element::custom(
                        5,
                        RadioButton::new("Autre", true, Msg::Click(2))
                    )]
                )
            ]
        ))
        .is_ok()
    );
}

#[test]
fn choice_drawing_respects_styles_themes_focus_and_mixed_state() {
    let accent = Color::rgb(123, 45, 67);
    let foreground = Color::rgb(21, 43, 65);
    let checkbox = Checkbox::new("Case", false, |v| Msg::Toggle(1, v))
        .state(CheckState::Mixed)
        .style(ChoiceStyle {
            accent: Some(accent),
            foreground: Some(foreground),
            ..ChoiceStyle::default()
        });
    for theme in [Theme::LIGHT, Theme::DARK] {
        let ctx = PaintContext {
            rect: RectF::new(0., 0., 240., 40.),
            intrinsic: SizeF {
                width: 240.,
                height: 40.,
            },
            theme,
            state: VisualState {
                focused: true,
                ..VisualState::default()
            },
        };
        let mut scene = Scene::new(theme.background);
        checkbox.paint(&ctx, &mut scene);
        scene.validate().unwrap();
        assert!(
            scene.commands.iter().any(
                |c| matches!(c,crate::ui::scene::DrawCommand::Fill {color,..} if *color==accent)
            )
        );
        assert!(scene.commands.iter().any(
            |c| matches!(c,crate::ui::scene::DrawCommand::Text {color,..} if *color==foreground)
        ));
        assert!(
            scene.commands.iter().any(
                |c| matches!(c,crate::ui::scene::DrawCommand::Stroke {width,..} if *width==2.)
            )
        );
    }
}
