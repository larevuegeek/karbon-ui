//! Ajouts aux entrées brutes (KUI-018, KUI-024) : boutons 4 et 5 de la souris, pression
//! et inclinaison d'un stylet, place du pointeur sur l'écran.
use crate::{
    platform::headless::{HeadlessApp, Pen, RawModifiers},
    ui::{
        prelude::*,
        runtime::{CursorShape, PointerEvent},
        tests::support::*,
    },
};

/// Une zone qui note chaque appui : le bouton, la pression, l'inclinaison et la place
/// sur l'écran.
#[derive(Default)]
struct Pad {
    seen: Vec<Seen>,
}

/// Le bouton, la pression, l'inclinaison et la place sur l'écran d'un appui.
type Seen = (Option<PointerButton>, f32, (f32, f32), (f32, f32));

impl Application for Pad {
    type Message = Seen;
    fn view(&self, _: &ViewContext) -> Element {
        column("pad", [])
            .width(Length::Px(300.))
            .height(Length::Px(200.))
            .on_pointer(|e: PointerEvent| {
                (e.phase == PointerPhase::Down).then_some((e.button, e.pressure, e.tilt, e.screen))
            })
    }
    fn update(&mut self, seen: Self::Message) -> bool {
        self.seen.push(seen);
        false
    }
}

fn at(x: f32, y: f32) -> PointF {
    PointF { x, y }
}

#[test]
fn the_back_and_forward_buttons_reach_the_element_and_end_their_gesture() {
    let mut app = HeadlessApp::new(Pad::default()).unwrap();
    let none = RawModifiers::default();
    for button in [PointerButton::Back, PointerButton::Forward] {
        app.raw_pointer(PointerPhase::Down, Some(button), at(20., 20.), none)
            .unwrap();
        app.raw_pointer(PointerPhase::Up, Some(button), at(20., 20.), none)
            .unwrap();
    }
    let buttons: Vec<_> = app.app().seen.iter().map(|s| s.0).collect();
    assert_eq!(
        buttons,
        [Some(PointerButton::Back), Some(PointerButton::Forward)]
    );
    // Relâché, le bouton 5 rend le geste : un appui suivant au bouton gauche est neuf.
    app.raw_pointer(
        PointerPhase::Down,
        Some(PointerButton::Primary),
        at(5., 5.),
        none,
    )
    .unwrap();
    assert_eq!(app.app().seen.len(), 3);
}

#[test]
fn a_pen_gives_its_pressure_and_tilt_and_a_mouse_presses_fully() {
    let mut app = HeadlessApp::new(Pad::default()).unwrap();
    let left = Some(PointerButton::Primary);
    let pen = Pen {
        pressure: 0.5,
        tilt: (12., -30.),
    };
    app.raw_pen(PointerPhase::Down, left, at(10., 10.), pen)
        .unwrap();
    app.raw_pen(PointerPhase::Up, left, at(10., 10.), pen)
        .unwrap();
    // Témoin : la souris qui suit n'hérite pas du stylet.
    let none = RawModifiers::default();
    app.raw_pointer(PointerPhase::Down, left, at(10., 10.), none)
        .unwrap();
    let seen = &app.app().seen;
    assert_eq!((seen[0].1, seen[0].2), (0.5, (12., -30.)));
    assert_eq!((seen[1].1, seen[1].2), (1., (0., 0.)));
}

#[test]
fn the_place_on_the_screen_follows_the_window_and_the_scale_of_its_screen() {
    let mut app = HeadlessApp::new(Pad::default()).unwrap();
    let left = Some(PointerButton::Primary);
    let none = RawModifiers::default();
    // Au coin de l'écran, la place sur l'écran est celle dans la fenêtre.
    app.raw_pointer(PointerPhase::Down, left, at(40., 30.), none)
        .unwrap();
    app.raw_pointer(PointerPhase::Up, left, at(40., 30.), none)
        .unwrap();
    // Sur un second écran à 150 % quand le principal est à 100 %.
    app.place_on_screen((1920., 100.), 1.5);
    app.raw_pointer(PointerPhase::Down, left, at(40., 30.), none)
        .unwrap();
    let seen = &app.app().seen;
    assert_eq!(seen[0].3, (40., 30.));
    assert_eq!(seen[1].3, (1980., 145.));
}

#[test]
fn the_pressure_of_windows_ink_goes_from_zero_to_one() {
    use crate::ui::runtime::raw_input::pen_pressure;
    assert_eq!(pen_pressure(0), 0.);
    assert_eq!(pen_pressure(512), 0.5);
    assert_eq!(pen_pressure(1024), 1.);
    assert_eq!(pen_pressure(4096), 1.);
}

/// Trois zones : une toile au curseur fait d'une image (sa main fermée en geste), une
/// vue au curseur caché qui contient un champ de texte, et une zone sans curseur choisi.
struct Shapes {
    hand: CursorShape,
}

impl Application for Shapes {
    type Message = ();
    fn view(&self, _: &ViewContext) -> Element {
        let size = |e: Element| e.width(Length::Px(100.)).height(Length::Px(100.));
        let canvas = size(column("canvas", []))
            .cursor_shape(self.hand.clone())
            .on_pointer(|e: PointerEvent| (e.phase == PointerPhase::Down).then_some(()));
        let field = input("field", "", |_: String| ());
        let view = size(column("view", [field])).cursor_shape(CursorShape::Hidden);
        let plain = size(column("plain", []));
        row("row", [canvas, view, plain])
    }
    fn update(&mut self, _: ()) -> bool {
        false
    }
}

#[test]
fn an_element_shows_its_cursor_and_keeps_it_during_its_gesture() {
    let dot = Arc::new(Image::new(2, 2, vec![255; 16]).unwrap());
    let hand = CursorShape::Image(crate::ui::runtime::CursorImage::new(dot, (9, 1)));
    let mut app = HeadlessApp::new(Shapes { hand: hand.clone() }).unwrap();
    let over = |app: &mut HeadlessApp<Shapes>, x: f32, y: f32| {
        app.event(UiEvent::PointerMove(at(x, y))).unwrap();
        app.runtime().cursor_shape()
    };
    // Le point de visée est ramené dans l'image.
    let CursorShape::Image(image) = &hand else {
        unreachable!()
    };
    assert_eq!(image.hotspot, (1, 1));
    assert_eq!(over(&mut app, 50., 50.), Some(hand.clone()));
    assert_eq!(over(&mut app, 150., 90.), Some(CursorShape::Hidden));
    // Sur le champ de texte, le runtime garde la barre de texte.
    assert_eq!(over(&mut app, 150., 8.), None);
    // Témoin : sans curseur choisi, le runtime garde le sien.
    assert_eq!(over(&mut app, 250., 50.), None);
    // Pendant le geste pris sur la toile, sa main vaut partout.
    let none = RawModifiers::default();
    let left = Some(PointerButton::Primary);
    app.raw_pointer(PointerPhase::Down, left, at(50., 50.), none)
        .unwrap();
    assert_eq!(over(&mut app, 250., 50.), Some(hand.clone()));
    app.raw_pointer(PointerPhase::Up, left, at(250., 50.), none)
        .unwrap();
    assert_eq!(over(&mut app, 250., 50.), None);
}

/// Une toile dessinée par l'application qui prend le texte tapé, à côté d'un champ de
/// texte du runtime.
#[derive(Default)]
struct Typing {
    field: String,
    typed: Vec<crate::ui::runtime::TextEvent>,
}

#[derive(Clone)]
enum TypingMsg {
    Field(String),
    Typed(crate::ui::runtime::TextEvent),
}

impl Application for Typing {
    type Message = TypingMsg;
    fn view(&self, _: &ViewContext) -> Element {
        let canvas = column("canvas", [])
            .width(Length::Px(200.))
            .height(Length::Px(100.))
            .on_text(|event: &crate::ui::runtime::TextEvent| Some(TypingMsg::Typed(event.clone())))
            .text_caret(RectF {
                left: 30.,
                top: 10.,
                right: 31.,
                bottom: 30.,
            });
        let field = input("field", self.field.as_str(), TypingMsg::Field);
        column("page", [canvas, field])
    }
    fn update(&mut self, message: TypingMsg) -> bool {
        match message {
            TypingMsg::Field(text) => self.field = text,
            TypingMsg::Typed(event) => self.typed.push(event),
        }
        true
    }
}

#[test]
fn an_element_drawn_by_the_application_receives_typed_text_and_compositions() {
    use crate::ui::runtime::TextEvent;
    let mut app = HeadlessApp::new(Typing::default()).unwrap();
    // Avant tout appui, rien n'a le clavier : le texte ne va nulle part.
    app.type_text("x").unwrap();
    assert!(app.app().typed.is_empty());
    let none = RawModifiers::default();
    let left = Some(PointerButton::Primary);
    app.raw_pointer(PointerPhase::Down, left, at(50., 50.), none)
        .unwrap();
    app.raw_pointer(PointerPhase::Up, left, at(50., 50.), none)
        .unwrap();
    app.type_text("é").unwrap();
    app.event(UiEvent::CompositionStart).unwrap();
    let value: std::sync::Arc<str> = "にほ".into();
    app.event(UiEvent::CompositionUpdate {
        value,
        cursor_utf16: 1,
    })
    .unwrap();
    let geometry = app.runtime().text_target_geometry().unwrap();
    assert!(geometry.composing);
    assert_eq!((geometry.caret.left, geometry.caret.top), (30., 10.));
    app.event(UiEvent::CompositionCommit("日本".into()))
        .unwrap();
    app.event(UiEvent::CompositionEnd).unwrap();
    assert_eq!(
        app.app().typed,
        [
            TextEvent::Insert("é".into()),
            TextEvent::Composition {
                text: "にほ".into(),
                cursor: "に".len()
            },
            TextEvent::Insert("日本".into()),
            TextEvent::CompositionEnd,
        ]
    );
    // Témoin : un champ du runtime qui a le focus garde son texte.
    app.click("field").unwrap();
    app.type_text("ê").unwrap();
    assert_eq!(app.app().field, "ê");
    assert_eq!(app.app().typed.len(), 4);
    assert!(app.runtime().text_target_geometry().is_none());
}
