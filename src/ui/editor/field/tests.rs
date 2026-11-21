//! Champs numériques : pas, bornes, vitesses (Maj, Ctrl), annulation par Échap, un seul
//! message de fin de geste, saisie au clavier posée sur le champ, vecteurs.
use super::{view::field_element, *};
use crate::ui::{
    editor::testing::{CONTROL, NONE, SHIFT, at, pointer, window},
    element::Element,
    prelude::{Application, ViewContext, column},
    runtime::{Key, KeyCode, KeyPress, PointerButton, PointerPhase},
    scene::PointF,
};

/// Une lampe : intensité bornée de 0 à 100, position en vecteur. Note les événements.
struct Lamp {
    intensity: f64,
    position: [f64; 3],
    fields: FieldState,
    events: Vec<FieldEvent>,
}

impl Default for Lamp {
    fn default() -> Self {
        Self {
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
            .step(0.5)
            .unit(" lx")
            .element();
        let position = VectorField::new(2, self.position, &self.fields, |m| m)
            .step(0.01)
            .element();
        column("page", [intensity, position]).gap(4.).padding(10.)
    }
    fn update(&mut self, message: FieldMsg) -> bool {
        let reply = self.fields.update(message);
        self.store(reply)
    }
    fn key(&mut self, press: KeyPress) -> bool {
        let reply = self.fields.key(press);
        self.store(reply)
    }
}

impl Lamp {
    fn store(&mut self, reply: FieldReply) -> bool {
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

fn intensity() -> crate::ui::element::ElementId {
    field_element(FieldId { key: 1, axis: 0 })
}

fn changes(lamp: &Lamp) -> Vec<FieldChange> {
    lamp.events.iter().map(|e| e.change).collect()
}

fn shifted(point: PointF, dx: f32) -> PointF {
    PointF {
        x: point.x + dx,
        ..point
    }
}

/// Clic simple (appui et relâchement sans bouger) sur l'intensité : la saisie s'ouvre.
fn open_typing(w: &mut crate::platform::headless::HeadlessApp<Lamp>) {
    let left = Some(PointerButton::Primary);
    let point = at(w, intensity(), 100., 10.);
    pointer(w, PointerPhase::Down, left, point, NONE);
    pointer(w, PointerPhase::Up, left, point, NONE);
}

#[test]
fn dragging_changes_the_value_by_its_step_within_its_bounds() {
    let mut w = window(Lamp::default(), 400., 200.);
    let left = Some(PointerButton::Primary);
    let start = at(&w, intensity(), 100., 10.);
    pointer(&mut w, PointerPhase::Down, left, start, NONE);
    // Sous le seuil : rien ne bouge.
    pointer(&mut w, PointerPhase::Move, None, shifted(start, 2.), NONE);
    assert_eq!(w.app().intensity, 50.);
    // 0,5 par pixel : 20 pixels de plus font +10.
    pointer(&mut w, PointerPhase::Move, None, shifted(start, 22.), NONE);
    assert_eq!(w.app().intensity, 60.);
    // Maj : dix fois plus vite ; la borne haute arrête la valeur.
    pointer(&mut w, PointerPhase::Move, None, shifted(start, 42.), SHIFT);
    assert_eq!(w.app().intensity, 100.);
    // En revenant, la valeur repart de la borne, pas d'un excès caché.
    pointer(&mut w, PointerPhase::Move, None, shifted(start, 32.), NONE);
    assert_eq!(w.app().intensity, 95.);
    // Ctrl : dix fois plus fin (0,05 par pixel), arrondi à une décimale.
    pointer(
        &mut w,
        PointerPhase::Move,
        None,
        shifted(start, 22.),
        CONTROL,
    );
    assert_eq!(w.app().intensity, 94.5);
    pointer(&mut w, PointerPhase::Up, left, shifted(start, 22.), NONE);
    // Des aperçus pendant le geste, une seule validation à la fin.
    let all = changes(w.app());
    assert_eq!(all.last(), Some(&FieldChange::Commit));
    assert_eq!(all.iter().filter(|c| **c == FieldChange::Commit).count(), 1);
    assert!(
        all[..all.len() - 1]
            .iter()
            .all(|c| *c == FieldChange::Preview)
    );
    assert!(!w.app().fields.is_active());
}

#[test]
fn escape_during_a_drag_puts_the_value_back() {
    let mut w = window(Lamp::default(), 400., 200.);
    let left = Some(PointerButton::Primary);
    let start = at(&w, intensity(), 100., 10.);
    pointer(&mut w, PointerPhase::Down, left, start, NONE);
    pointer(&mut w, PointerPhase::Move, None, shifted(start, -60.), NONE);
    assert_eq!(w.app().intensity, 20.);
    w.prepare().unwrap();
    w.runtime_mut()
        .key_input(KeyPress::new(KeyCode::Escape, true));
    w.prepare().unwrap();
    assert_eq!(w.app().intensity, 50.);
    assert_eq!(changes(w.app()).last(), Some(&FieldChange::Cancel));
    // Le relâchement qui suit ne valide rien.
    pointer(&mut w, PointerPhase::Up, left, shifted(start, -60.), NONE);
    assert_eq!(changes(w.app()).last(), Some(&FieldChange::Cancel));
}

#[test]
fn a_click_opens_typing_over_the_field_and_enter_confirms_an_expression() {
    let mut w = window(Lamp::default(), 400., 200.);
    let field = FieldId { key: 1, axis: 0 };
    let id = field_element(field);
    let left = Some(PointerButton::Primary);
    let point = at(&w, id, 100., 10.);
    pointer(&mut w, PointerPhase::Down, left, point, NONE);
    pointer(&mut w, PointerPhase::Up, left, point, NONE);
    assert_eq!(w.app().fields.editing(), Some(field));
    // La zone de saisie remplace le champ, a le clavier et toute la valeur (sans unité)
    // choisie : la frappe la remplace.
    let input = id.child("input");
    assert!(w.find(id).is_none());
    assert_eq!(w.focused(), Some(input));
    assert_eq!(w.input_value(input).as_deref(), Some("50.0"));
    w.type_text("2*3,5").unwrap();
    w.press(Key::Enter).unwrap();
    assert_eq!(w.app().intensity, 7.);
    assert_eq!(changes(w.app()), [FieldChange::Commit]);
    assert!(w.find(input).is_none() && w.find(id).is_some());
}

#[test]
fn escape_cancels_typing_and_a_click_elsewhere_confirms_it() {
    let mut w = window(Lamp::default(), 400., 200.);
    open_typing(&mut w);
    w.type_text("12").unwrap();
    w.press(Key::Escape).unwrap();
    assert_eq!(w.app().intensity, 50.);
    assert!(w.app().events.is_empty());
    assert!(w.app().fields.editing().is_none());
    // Saisie de nouveau, puis clic hors de la zone : la valeur tapée est gardée.
    open_typing(&mut w);
    w.type_text("12").unwrap();
    w.click_at(PointF { x: 390., y: 190. }).unwrap();
    assert_eq!(w.app().intensity, 12.);
    // Une saisie illisible garde la valeur d'avant.
    open_typing(&mut w);
    w.type_text("douze").unwrap();
    w.press(Key::Enter).unwrap();
    assert_eq!(w.app().intensity, 12.);
}

#[test]
fn the_axes_of_a_vector_are_separate_fields() {
    let mut w = window(Lamp::default(), 400., 200.);
    let left = Some(PointerButton::Primary);
    let y = field_element(FieldId { key: 2, axis: 1 });
    let start = at(&w, y, 40., 10.);
    pointer(&mut w, PointerPhase::Down, left, start, NONE);
    pointer(&mut w, PointerPhase::Move, None, shifted(start, 53.), NONE);
    pointer(&mut w, PointerPhase::Up, left, shifted(start, 53.), NONE);
    assert_eq!(w.app().position, [0., 0.53, 0.]);
    let last = w.app().events.last().unwrap();
    assert_eq!(last.field, FieldId { key: 2, axis: 1 });
    // Les trois champs se partagent la largeur, dans l'ordre X, Y, Z.
    let boxes: Vec<_> = (0..3)
        .map(|axis| w.bounds(field_element(FieldId { key: 2, axis })).unwrap())
        .collect();
    assert!(boxes[0].right <= boxes[1].left && boxes[1].right <= boxes[2].left);
}
