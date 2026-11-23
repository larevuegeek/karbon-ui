//! Déplacement libre (`onpan`) : seuil, clic supprimé, phases, capture gardée malgré les
//! reconstructions, annulation.
use crate::ui::drag_drop::{PanEvent, PanPhase};
use crate::ui::tests::support::*;
use std::cell::RefCell;
use std::time::Instant;
#[derive(Default)]
struct Data {
    clicks: usize,
    pans: Vec<PanEvent>,
}
struct App(Rc<RefCell<Data>>);
impl Application for App {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        Element::column(
            1,
            vec![
                widgets::button(2, "Poignée", Msg::Click(2))
                    .height(Length::Px(60.))
                    .on_pan(|event| Msg::Pan(2, event)),
            ],
        )
    }
    fn update(&mut self, message: Msg) -> bool {
        match message {
            Msg::Click(_) => self.0.borrow_mut().clicks += 1,
            Msg::Pan(_, event) => self.0.borrow_mut().pans.push(event),
            other => panic!("unexpected message {other:?}"),
        }
        // Chaque message reconstruit la vue : le geste doit survivre.
        true
    }
}
fn setup() -> (UiRuntime, Measure, Rc<RefCell<Data>>, PointF) {
    let data = Rc::new(RefCell::new(Data::default()));
    let mut ui = UiRuntime::new(App(Rc::clone(&data)), &NoResources).unwrap();
    let mut m = Measure::default();
    prepare(&mut ui, &mut m);
    let r = ui.root().unwrap().find(ElementId(2)).unwrap().rect;
    let p = PointF {
        x: r.left + 20.,
        y: r.top + 20.,
    };
    (ui, m, data, p)
}
fn at(p: PointF, dx: f32, dy: f32) -> PointF {
    PointF {
        x: p.x + dx,
        y: p.y + dy,
    }
}
#[test]
fn a_press_under_the_threshold_stays_a_click() {
    let (mut ui, _, data, p) = setup();
    assert!(ui.event(UiEvent::PointerDown(p)).capture);
    ui.event(UiEvent::PointerMove(at(p, 2., 1.)));
    assert!(ui.event(UiEvent::PointerUp(at(p, 2., 1.))).release);
    assert_eq!(data.borrow().clicks, 1);
    assert!(data.borrow().pans.is_empty());
}
#[test]
fn a_drag_reports_start_moves_and_end_without_a_click_across_rebuilds() {
    let (mut ui, mut m, data, p) = setup();
    ui.event(UiEvent::PointerDown(p));
    ui.event(UiEvent::PointerMove(at(p, 10., 0.)));
    prepare(&mut ui, &mut m);
    assert!(ui.has_capture());
    ui.event(UiEvent::PointerMove(at(p, 30., -40.)));
    prepare(&mut ui, &mut m);
    assert!(ui.event(UiEvent::PointerUp(at(p, 30., -40.))).release);
    assert!(!ui.has_capture());
    let pans = data.borrow().pans.clone();
    let phases: Vec<PanPhase> = pans.iter().map(|e| e.phase).collect();
    assert_eq!(phases, [PanPhase::Start, PanPhase::Move, PanPhase::End]);
    assert_eq!((pans[1].dx, pans[1].dy), (30., -40.));
    assert_eq!((pans[2].x, pans[2].y), (p.x + 30., p.y - 40.));
    // Le parent du bouton : la colonne, mesurée à l'appui.
    let column = ui.root().unwrap().rect;
    assert_eq!(pans[0].parent_width, column.right - column.left);
    assert_eq!(pans[0].parent_height, column.bottom - column.top);
    assert_eq!(data.borrow().clicks, 0);
}
#[test]
fn a_cancelled_drag_still_ends() {
    let (mut ui, _, data, p) = setup();
    ui.event(UiEvent::PointerDown(p));
    ui.event(UiEvent::PointerMove(at(p, 0., 12.)));
    assert!(ui.event(UiEvent::Cancel).release);
    let phases: Vec<PanPhase> = data.borrow().pans.iter().map(|e| e.phase).collect();
    assert_eq!(phases, [PanPhase::Start, PanPhase::End]);
    assert_eq!(data.borrow().clicks, 0);
}

/// Un bloc `pan_frame` placé par l'application (marge gauche) ; lâché à moins de 50
/// unités du bord, l'application le colle à gauche.
/// Marge gauche du bloc, événements reçus.
type FrameData = Rc<RefCell<(f32, Vec<PanEvent>)>>;
struct FrameApp(FrameData);
impl Application for FrameApp {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        let left = self.0.borrow().0;
        Element::column(
            1,
            vec![
                Element::column(
                    3,
                    vec![
                        widgets::button(2, "Poignée", Msg::Click(2))
                            .height(Length::Px(40.))
                            .on_pan(|event| Msg::Pan(2, event)),
                    ],
                )
                .width(Length::Px(100.))
                .height(Length::Px(60.))
                .margin(Insets::edges(20., 0., 0., left))
                .pan_frame(),
            ],
        )
    }
    fn update(&mut self, message: Msg) -> bool {
        let Msg::Pan(_, event) = message else {
            return false;
        };
        let mut data = self.0.borrow_mut();
        data.1.push(event);
        if event.phase != PanPhase::End {
            // Le moteur déplace le bloc : rien à reconstruire pendant le geste.
            return false;
        }
        let dropped = data.0 + event.dx;
        data.0 = if dropped < 50. { 0. } else { dropped };
        true
    }
}
fn frame_setup(left: f32) -> (UiRuntime, Measure, FrameData, PointF) {
    let data = Rc::new(RefCell::new((left, Vec::new())));
    let mut ui = UiRuntime::new(FrameApp(Rc::clone(&data)), &NoResources).unwrap();
    let mut m = Measure::default();
    prepare(&mut ui, &mut m);
    let r = ui.root().unwrap().find(ElementId(2)).unwrap().rect;
    let p = PointF {
        x: r.left + 20.,
        y: r.top + 20.,
    };
    (ui, m, data, p)
}
fn frame_left(ui: &UiRuntime) -> f32 {
    ui.root().unwrap().find(ElementId(3)).unwrap().rect.left
}
#[test]
fn the_frame_follows_the_pointer_without_rebuild_or_layout_and_stays_in_the_window() {
    let (mut ui, mut m, data, p) = frame_setup(100.);
    let before = ui.stats();
    ui.event(UiEvent::PointerDown(p));
    ui.event(UiEvent::PointerMove(at(p, 150., 0.)));
    prepare(&mut ui, &mut m);
    assert_eq!(frame_left(&ui), 250.);
    // La poignée suit le bloc : le pointeur reste dessus.
    let handle = ui.root().unwrap().find(ElementId(2)).unwrap().rect;
    assert!(handle.left <= p.x + 150. && p.x + 150. <= handle.right);
    assert_eq!(ui.stats().views, before.views);
    assert_eq!(ui.stats().layouts, before.layouts);
    // Tiré bien au-delà du bord gauche : le bloc s'arrête au bord de la fenêtre.
    ui.event(UiEvent::PointerMove(at(p, -500., 0.)));
    assert_eq!(frame_left(&ui), 0.);
    assert_eq!(data.borrow().1.last().unwrap().dx, -100.);
    ui.event(UiEvent::PointerUp(at(p, -500., 0.)));
    prepare(&mut ui, &mut m);
    assert_eq!(frame_left(&ui), 0.);
}
#[test]
fn after_release_the_frame_glides_from_the_drop_point_to_its_new_place() {
    let (mut ui, mut m, data, p) = frame_setup(100.);
    ui.event(UiEvent::PointerDown(p));
    ui.event(UiEvent::PointerMove(at(p, -70., 0.)));
    ui.event(UiEvent::PointerUp(at(p, -70., 0.)));
    let start = Instant::now();
    prepare(&mut ui, &mut m);
    // L'application l'a collé au bord (0) ; il part du point de lâcher (30).
    assert_eq!(data.borrow().0, 0.);
    assert!((frame_left(&ui) - 30.).abs() < 1.);
    assert!(ui.next_wake().is_some());
    ui.event(UiEvent::Timer(start + std::time::Duration::from_millis(60)));
    let halfway = frame_left(&ui);
    assert!(halfway > 0. && halfway < 30., "{halfway}");
    ui.event(UiEvent::Timer(
        start + std::time::Duration::from_millis(400),
    ));
    assert_eq!(frame_left(&ui), 0.);
    assert!(ui.next_wake().is_none());
}

/// Le dernier événement dit si le geste a été lâché ou annulé : un dépôt n'a lieu que
/// dans le premier cas.
#[test]
fn the_end_of_a_pan_tells_a_release_from_a_cancellation() {
    let (mut ui, _, data, p) = setup();
    ui.event(UiEvent::PointerDown(p));
    ui.event(UiEvent::PointerMove(at(p, 0., 12.)));
    ui.event(UiEvent::PointerUp(at(p, 0., 12.)));
    ui.event(UiEvent::PointerDown(p));
    ui.event(UiEvent::PointerMove(at(p, 0., 12.)));
    ui.event(UiEvent::Cancel);
    let ends: Vec<(PanPhase, bool)> = data
        .borrow()
        .pans
        .iter()
        .map(|e| (e.phase, e.cancelled))
        .collect();
    assert_eq!(
        ends,
        [
            (PanPhase::Start, false),
            (PanPhase::End, false),
            (PanPhase::Start, false),
            (PanPhase::End, true),
        ]
    );
}
