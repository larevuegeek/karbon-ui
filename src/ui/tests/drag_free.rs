//! Glisser libre entre panneaux (`ui::drag`, KUI-045) : une vignette qui porte une
//! texture, tirée sur une vue qui l'accepte et sur un arbre qui la refuse ; seuil, clic
//! évité, lâcher, Échap, fenêtre qui perd la souris.
use crate::{
    platform::headless::{HeadlessApp, RawModifiers},
    ui::{
        drag::{DragEvent, DragPhase, Payload},
        prelude::*,
        runtime::{KeyCode, KeyPress, PointerButton, PointerPhase, UiEvent},
        scene::PointF,
    },
};

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Clicked,
    Source(DragPhase, bool, bool, bool),
    Aim(Option<(f32, f32)>),
    Placed(u64, (f32, f32)),
    TreeOver,
}

#[derive(Default)]
struct Board {
    log: Vec<Msg>,
}

impl Application for Board {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        let brick = button("brick", "Brique", Msg::Clicked)
            .width(Length::Px(100.))
            .height(Length::Px(40.))
            .drag_payload(Payload::new("texture", "Brique", 42_u64))
            .on_drag(|e: &DragEvent| {
                (e.phase != DragPhase::Move).then_some(Msg::Source(
                    e.phase,
                    e.accepted,
                    e.dropped,
                    e.cancelled,
                ))
            });
        let view = column("view", [])
            .width(Length::Px(200.))
            .height(Length::Px(100.))
            .on_drag_over(|e: &DragEvent| match e.phase {
                DragPhase::Over if e.payload.is("texture") => Some(Msg::Aim(e.fraction())),
                DragPhase::Leave => Some(Msg::Aim(None)),
                _ => None,
            })
            .on_drop(|e: &DragEvent| Some(Msg::Placed(*e.payload.get::<u64>()?, e.fraction()?)));
        let tree = column("tree", [])
            .width(Length::Px(200.))
            .height(Length::Px(100.))
            .on_drag_over(|e: &DragEvent| e.payload.is("model").then_some(Msg::TreeOver));
        column("page", [brick, view, tree])
    }
    fn update(&mut self, message: Msg) -> bool {
        self.log.push(message);
        true
    }
}

fn at(x: f32, y: f32) -> PointF {
    PointF { x, y }
}

/// Ce que fait une fenêtre de Windows : l'événement du runtime, puis l'entrée brute.
fn mouse(app: &mut HeadlessApp<Board>, phase: PointerPhase, p: PointF) {
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

#[test]
fn a_dragged_texture_is_accepted_by_the_view_refused_by_the_tree_and_dropped() {
    let mut app = HeadlessApp::new(Board::default()).unwrap();
    mouse(&mut app, PointerPhase::Down, at(20., 20.));
    // Sous le seuil : rien ne commence.
    mouse(&mut app, PointerPhase::Move, at(22., 22.));
    assert!(app.app().log.is_empty());
    // Sur la vue (rangée 40 à 140) : elle accepte, au quart de sa largeur.
    mouse(&mut app, PointerPhase::Move, at(50., 90.));
    // Sur l'arbre : il refuse une texture, la vue apprend que le pointeur est parti.
    mouse(&mut app, PointerPhase::Move, at(50., 190.));
    mouse(&mut app, PointerPhase::Move, at(150., 115.));
    mouse(&mut app, PointerPhase::Up, at(150., 115.));
    assert_eq!(
        app.app().log,
        [
            Msg::Source(DragPhase::Start, false, false, false),
            Msg::Aim(Some((0.25, 0.5))),
            Msg::Aim(None),
            Msg::Aim(Some((0.75, 0.75))),
            Msg::Placed(42, (0.75, 0.75)),
            Msg::Source(DragPhase::End, true, true, false),
        ]
    );
    // Le clic sur la vignette n'a pas eu lieu.
    assert!(!app.app().log.contains(&Msg::Clicked));
}

#[test]
fn escape_cancels_a_drag_without_a_drop_and_a_plain_press_still_clicks() {
    let mut app = HeadlessApp::new(Board::default()).unwrap();
    mouse(&mut app, PointerPhase::Down, at(20., 20.));
    mouse(&mut app, PointerPhase::Move, at(50., 90.));
    app.raw_key(KeyPress::new(KeyCode::Escape, true)).unwrap();
    mouse(&mut app, PointerPhase::Up, at(50., 90.));
    let log = &app.app().log;
    assert_eq!(log[log.len() - 2], Msg::Aim(None));
    assert_eq!(
        log[log.len() - 1],
        Msg::Source(DragPhase::End, false, false, true)
    );
    assert!(!log.iter().any(|m| matches!(m, Msg::Placed(..))));
    // Témoin : un appui sans mouvement reste un clic.
    mouse(&mut app, PointerPhase::Down, at(20., 20.));
    mouse(&mut app, PointerPhase::Up, at(20., 20.));
    assert_eq!(app.app().log.last(), Some(&Msg::Clicked));
}

#[test]
fn a_window_that_loses_the_mouse_cancels_the_drag_and_a_release_outside_drops_nothing() {
    let mut app = HeadlessApp::new(Board::default()).unwrap();
    mouse(&mut app, PointerPhase::Down, at(20., 20.));
    mouse(&mut app, PointerPhase::Move, at(50., 90.));
    app.event(UiEvent::Cancel).unwrap();
    assert_eq!(
        app.app().log.last(),
        Some(&Msg::Source(DragPhase::End, false, false, true))
    );
    // Lâché hors de toute cible (sous l'arbre) : rien de posé, la source le sait.
    mouse(&mut app, PointerPhase::Down, at(20., 20.));
    mouse(&mut app, PointerPhase::Move, at(50., 400.));
    mouse(&mut app, PointerPhase::Up, at(50., 400.));
    assert_eq!(
        app.app().log.last(),
        Some(&Msg::Source(DragPhase::End, false, false, false))
    );
    assert!(!app.app().log.iter().any(|m| matches!(m, Msg::Placed(..))));
}

#[test]
fn a_payload_keeps_its_kind_label_and_typed_value() {
    let files = Payload::files(vec!["C:/maisons/chalet.glb".into()]);
    assert!(files.is(Payload::FILES));
    assert_eq!(files.label(), "chalet.glb");
    assert_eq!(
        files.get::<Vec<std::path::PathBuf>>().map(Vec::len),
        Some(1)
    );
    let copy = files.clone();
    assert_eq!(copy, files);
    assert_ne!(Payload::files(vec![]), files);
}

#[test]
fn a_drag_that_comes_back_to_its_source_does_not_click_it() {
    let mut app = HeadlessApp::new(Board::default()).unwrap();
    mouse(&mut app, PointerPhase::Down, at(20., 20.));
    mouse(&mut app, PointerPhase::Move, at(50., 90.));
    mouse(&mut app, PointerPhase::Move, at(30., 20.));
    mouse(&mut app, PointerPhase::Up, at(30., 20.));
    assert_eq!(
        app.app().log.last(),
        Some(&Msg::Source(DragPhase::End, false, false, false))
    );
    assert!(!app.app().log.contains(&Msg::Clicked));
}
