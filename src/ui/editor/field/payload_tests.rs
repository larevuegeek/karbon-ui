//! Ce qu'on apporte à un champ par un glisser libre (`NumberField::on_payload`,
//! `VectorField::on_payload`) : le champ visé l'accepte et le reçoit.
use super::{view::field_element, *};
use crate::{
    platform::headless::{HeadlessApp, RawModifiers},
    ui::{
        drag::{DragEvent, DragPhase, Payload},
        prelude::*,
        runtime::{PointerButton, PointerPhase, UiEvent},
        scene::PointF,
    },
};

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Field(FieldMsg),
    Over(Option<FieldId>),
    Dropped(FieldId, f64),
}

struct Lamp {
    accepts: bool,
    over: Option<FieldId>,
    position: [f64; 3],
    fields: FieldState,
}

impl Application for Lamp {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        let thumb = column("thumb", [])
            .width(Length::Px(60.))
            .height(Length::Px(40.))
            .drag_payload(Payload::new("number", "2,5", 2.5_f64));
        let over = |axis| self.over == Some(FieldId { key: 2, axis });
        let mut position = VectorField::new(2, self.position, &self.fields, Msg::Field)
            .payload_over([over(0), over(1), over(2)]);
        if self.accepts {
            position = position.on_payload(|e: &DragEvent, field| match e.phase {
                DragPhase::Over if e.payload.is("number") => Some(Msg::Over(Some(field))),
                DragPhase::Leave => Some(Msg::Over(None)),
                DragPhase::Drop => Some(Msg::Dropped(field, *e.payload.get::<f64>()?)),
                _ => None,
            });
        }
        column("page", [thumb, position.element()]).width(Length::Px(300.))
    }
    fn update(&mut self, message: Msg) -> bool {
        match message {
            Msg::Field(m) => return self.fields.update(m).rebuild,
            Msg::Over(over) => self.over = over,
            Msg::Dropped(field, value) => {
                self.position[field.axis] = value;
                self.over = None;
            }
        }
        true
    }
}

fn mouse(app: &mut HeadlessApp<Lamp>, phase: PointerPhase, p: PointF) {
    let event = match phase {
        PointerPhase::Down => UiEvent::PointerDown(p),
        PointerPhase::Up => UiEvent::PointerUp(p),
        _ => UiEvent::PointerMove(p),
    };
    app.event(event).unwrap();
    let button = (phase != PointerPhase::Move).then_some(PointerButton::Primary);
    app.raw_pointer(phase, button, p, RawModifiers::default())
        .unwrap();
}

fn lamp(accepts: bool) -> HeadlessApp<Lamp> {
    HeadlessApp::new(Lamp {
        accepts,
        over: None,
        position: [0.; 3],
        fields: FieldState::default(),
    })
    .unwrap()
}

fn centre_of(app: &HeadlessApp<Lamp>, axis: usize) -> PointF {
    let r = app.bounds(field_element(FieldId { key: 2, axis })).unwrap();
    PointF {
        x: (r.left + r.right) / 2.,
        y: (r.top + r.bottom) / 2.,
    }
}

#[test]
fn a_number_dropped_on_the_y_field_reaches_that_field() {
    let mut app = lamp(true);
    let y = centre_of(&app, 1);
    mouse(&mut app, PointerPhase::Down, PointF { x: 20., y: 20. });
    mouse(&mut app, PointerPhase::Move, y);
    assert_eq!(app.app().over, Some(FieldId { key: 2, axis: 1 }));
    mouse(&mut app, PointerPhase::Up, y);
    assert_eq!(app.app().position, [0., 2.5, 0.]);
    assert_eq!(app.app().over, None);
    // Le champ n'a pas pris la saisie ni le glisser de sa valeur.
    assert!(!app.app().fields.is_active());
}

#[test]
fn fields_without_on_payload_are_not_targets() {
    // Témoin : sans `on_payload`, rien n'arrive au champ.
    let mut app = lamp(false);
    let y = centre_of(&app, 1);
    mouse(&mut app, PointerPhase::Down, PointF { x: 20., y: 20. });
    mouse(&mut app, PointerPhase::Move, y);
    mouse(&mut app, PointerPhase::Up, y);
    assert_eq!(app.app().position, [0.; 3]);
}
