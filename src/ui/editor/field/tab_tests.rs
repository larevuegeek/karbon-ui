//! Champs qui s'arrêtent à Tab (`NumberField::tab_stop`) : Tab ouvre la saisie du champ
//! atteint, Tab dans la saisie valide et passe au suivant ; un clic reste un clic.
use super::{view::field_element, *};
use crate::{
    platform::headless::RawModifiers,
    ui::{
        editor::testing::window,
        element::Element,
        prelude::{Application, ViewContext, column},
        runtime::{Key, PointerButton, PointerPhase, UiEvent},
        scene::PointF,
    },
};

/// Une intensité et une position, arrêts de Tab ou non.
struct Lamp {
    tab_stop: bool,
    intensity: f64,
    position: [f64; 3],
    fields: FieldState,
    events: Vec<FieldEvent>,
}

impl Lamp {
    fn new(tab_stop: bool) -> Self {
        Self {
            tab_stop,
            intensity: 50.,
            position: [0.; 3],
            fields: FieldState::default(),
            events: Vec::new(),
        }
    }
}

impl Application for Lamp {
    type Message = FieldMsg;
    fn view(&self, _: &ViewContext) -> Element {
        let intensity = NumberField::new(1, self.intensity, &self.fields, |m| m)
            .range(0., 100.)
            .tab_stop(self.tab_stop)
            .element();
        let position = VectorField::new(2, self.position, &self.fields, |m| m)
            .step(0.01)
            .tab_stop(self.tab_stop)
            .element();
        column("page", [intensity, position]).gap(4.).padding(10.)
    }
    fn update(&mut self, message: FieldMsg) -> bool {
        let reply = self.fields.update(message);
        if let Some(event) = reply.event {
            match event.field.key {
                1 => self.intensity = event.value,
                _ => self.position[event.field.axis] = event.value,
            }
            self.events.push(event);
        }
        reply.rebuild
    }
}

fn axis(axis: usize) -> FieldId {
    FieldId { key: 2, axis }
}

#[test]
fn tab_opens_the_typing_box_of_each_field_in_turn() {
    let mut w = window(Lamp::new(true), 400., 200.);
    let intensity = FieldId { key: 1, axis: 0 };
    w.press(Key::Tab).unwrap();
    assert_eq!(w.app().fields.editing(), Some(intensity));
    let input = field_element(intensity).child("input");
    assert_eq!(w.focused(), Some(input));
    assert_eq!(w.input_value(input).as_deref(), Some("50.0"));
    w.type_text("12").unwrap();
    // Tab valide et passe à X, puis Y.
    w.press(Key::Tab).unwrap();
    assert_eq!(w.app().intensity, 12.);
    assert_eq!(w.app().fields.editing(), Some(axis(0)));
    assert_eq!(w.focused(), Some(field_element(axis(0)).child("input")));
    w.type_text("1,5").unwrap();
    w.press(Key::Tab).unwrap();
    assert_eq!(w.app().position, [1.5, 0., 0.]);
    assert_eq!(w.app().fields.editing(), Some(axis(1)));
    // Maj+Tab revient à X, sans rien changer à Y.
    w.press_with_shift(Key::Tab).unwrap();
    assert_eq!(w.app().fields.editing(), Some(axis(0)));
    assert_eq!(w.app().position, [1.5, 0., 0.]);
    // Échap ferme la saisie ; la boîte garde le clavier, son anneau se dessine.
    w.press(Key::Escape).unwrap();
    assert!(w.app().fields.editing().is_none());
    let changes: Vec<_> = w.app().events.iter().map(|e| e.change).collect();
    assert!(changes.iter().all(|c| *c == FieldChange::Commit));
}

#[test]
fn without_tab_stop_tab_skips_the_fields() {
    // Témoin : le comportement d'avant, Tab ne s'arrête sur aucun champ.
    let mut w = window(Lamp::new(false), 400., 200.);
    w.press(Key::Tab).unwrap();
    assert!(w.app().fields.editing().is_none());
    assert!(w.focused().is_none());
}

#[test]
fn a_press_and_drag_on_a_tab_stop_field_is_still_a_drag() {
    let mut w = window(Lamp::new(true), 400., 200.);
    let r = w
        .bounds(field_element(FieldId { key: 1, axis: 0 }))
        .unwrap();
    let start = PointF {
        x: r.left + 100.,
        y: r.top + 10.,
    };
    let end = PointF {
        x: start.x + 20.,
        ..start
    };
    let left = Some(PointerButton::Primary);
    let none = RawModifiers::default();
    // Comme une fenêtre : l'entrée brute, puis l'événement ordinaire (qui donne le focus).
    w.raw_pointer(PointerPhase::Down, left, start, none)
        .unwrap();
    w.event(UiEvent::PointerDown(start)).unwrap();
    w.raw_pointer(PointerPhase::Move, None, end, none).unwrap();
    w.event(UiEvent::PointerMove(end)).unwrap();
    w.raw_pointer(PointerPhase::Up, left, end, none).unwrap();
    w.event(UiEvent::PointerUp(end)).unwrap();
    assert_eq!(w.app().intensity, 52.);
    assert!(w.app().fields.editing().is_none());
    let commits = w
        .app()
        .events
        .iter()
        .filter(|e| e.change == FieldChange::Commit);
    assert_eq!(commits.count(), 1);
}
