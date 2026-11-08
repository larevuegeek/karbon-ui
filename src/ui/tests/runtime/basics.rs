//! Bases du runtime : capture du pointeur, clavier, focus, réutilisation du layout,
//! séparateurs et zones de déplacement de fenêtre.
use crate::platform::headless::HeadlessApp;
use crate::ui::tests::support::*;
// Porté sur le harnais headless : mesure déterministe, préparation automatique après
// chaque événement, et lecture de l'état via l'application elle-même.
#[test]
fn button_capture_keyboard_repeat_focus_and_disabled_state() {
    let count = Rc::new(Cell::new(0));
    let mut app = HeadlessApp::new(Counter {
        value: Rc::clone(&count),
    })
    .unwrap();
    let middle = |app: &HeadlessApp<Counter>, id: u64| {
        let r = app.bounds(id).unwrap();
        PointF {
            x: (r.left + r.right) / 2.,
            y: (r.top + r.bottom) / 2.,
        }
    };
    let p = middle(&app, 10);
    assert!(app.event(UiEvent::PointerDown(p)).unwrap().capture);
    let outside = PointF { x: 799., y: 599. };
    assert!(app.event(UiEvent::PointerUp(outside)).unwrap().release);
    assert_eq!(count.get(), 0);
    app.click(10).unwrap();
    assert_eq!(count.get(), 1);
    assert!(app.contains_text("Count: 1"));
    assert_eq!(app.focused(), Some(ElementId(10)));
    app.press(Key::Enter).unwrap();
    app.event(UiEvent::KeyDown {
        key: Key::Enter,
        shift: false,
        repeat: true,
    })
    .unwrap();
    assert_eq!(count.get(), 2);
    // Espace active au relâchement.
    app.event(UiEvent::KeyDown {
        key: Key::Space,
        shift: false,
        repeat: false,
    })
    .unwrap();
    assert_eq!(count.get(), 2);
    app.event(UiEvent::KeyUp(Key::Space)).unwrap();
    assert_eq!(count.get(), 3);
    app.press(Key::Tab).unwrap();
    assert_eq!(app.focused(), Some(ElementId(12)));
    app.press_with_shift(Key::Tab).unwrap();
    assert_eq!(app.focused(), Some(ElementId(10)));
    let disabled = middle(&app, 11);
    assert!(!app.event(UiEvent::PointerDown(disabled)).unwrap().capture);
    app.event(UiEvent::PointerUp(disabled)).unwrap();
    assert_eq!(count.get(), 3);
    assert!(app.accessible(11).is_some_and(|node| !node.enabled));
}

#[test]
fn hovering_reuses_tree_layout_and_measurements() {
    let count = Rc::new(Cell::new(0));
    let mut ui = UiRuntime::new(Counter { value: count }, &NoResources).unwrap();
    let mut measure = Measure::default();
    prepare(&mut ui, &mut measure);
    let before = ui.stats();
    let calls = measure.calls;
    let capacity = ui.scene().commands.capacity();
    let p = center(&ui, 10);
    for _ in 0..100 {
        ui.event(UiEvent::PointerMove(p));
        prepare(&mut ui, &mut measure);
        ui.event(UiEvent::PointerLeave);
        prepare(&mut ui, &mut measure);
    }
    assert_eq!(ui.stats().views, before.views);
    assert_eq!(ui.stats().layouts, before.layouts);
    assert_eq!(measure.calls, calls);
    assert_eq!(ui.scene().commands.capacity(), capacity);
    assert!(ui.scene().validate().is_ok());
}

struct SplitApp {
    resizable: bool,
}
impl Application for SplitApp {
    type Message = ();
    fn update(&mut self, _: ()) -> bool {
        false
    }
    fn view(&self, _: &ViewContext) -> Element {
        Element::split(
            1,
            Split {
                axis: Axis::Horizontal,
                fraction: 0.4,
                min_first: 100.,
                min_second: 120.,
                resizable: self.resizable,
            },
            Element::column(2, vec![]),
            Element::column(3, vec![]),
        )
        .padding(10.)
    }
}

#[test]
fn split_drag_clamps_cancels_and_preserves_ratio_after_rebuild() {
    let mut ui = UiRuntime::new(SplitApp { resizable: true }, &NoResources).unwrap();
    let mut measure = Measure::default();
    prepare(&mut ui, &mut measure);
    let root = ui.root().unwrap();
    let bar = root.divider.unwrap();
    let original = root.children[0].bounds().right;
    let p = PointF {
        x: (bar.left + bar.right) / 2.,
        y: bar.top + 20.,
    };
    assert!(ui.event(UiEvent::PointerDown(p)).capture);
    ui.event(UiEvent::PointerMove(PointF { x: 1000., y: p.y }));
    prepare(&mut ui, &mut measure);
    near(layout::width(ui.root().unwrap().children[1].bounds()), 120.);
    assert!(
        ui.event(UiEvent::KeyDown {
            key: Key::Escape,
            shift: false,
            repeat: false
        })
        .release
    );
    prepare(&mut ui, &mut measure);
    near(ui.root().unwrap().children[0].bounds().right, original);
    ui.event(UiEvent::PointerDown(p));
    ui.event(UiEvent::PointerMove(PointF { x: 320., y: p.y }));
    prepare(&mut ui, &mut measure);
    ui.event(UiEvent::PointerUp(PointF { x: 320., y: p.y }));
    let changed = ui.root().unwrap().children[0].bounds().right;
    ui.invalidate();
    prepare(&mut ui, &mut measure);
    near(ui.root().unwrap().children[0].bounds().right, changed);
    key(&mut ui, Key::Home);
    prepare(&mut ui, &mut measure);
    near(layout::width(ui.root().unwrap().children[0].bounds()), 100.);
}

#[test]
fn locked_split_has_no_resize_capture_or_focus_stop() {
    let mut ui = UiRuntime::new(SplitApp { resizable: false }, &NoResources).unwrap();
    let mut measure = Measure::default();
    prepare(&mut ui, &mut measure);
    let bar = ui.root().unwrap().divider.unwrap();
    let p = PointF {
        x: (bar.left + bar.right) / 2.,
        y: bar.top + 20.,
    };
    assert!(!ui.event(UiEvent::PointerDown(p)).capture);
    key(&mut ui, Key::Tab);
    assert_eq!(ui.focused(), None);
}

#[test]
fn drag_regions_move_the_window_except_over_interactive_targets() {
    use crate::ui::view::{button, column, row, space};
    let mut ui = view_runtime(|| {
        column(
            1,
            [
                row(
                    2,
                    [
                        button(3, "Fermer", Msg::Click(3)).width(60.).height(30.),
                        space(4).width(100.).height(30.),
                    ],
                )
                .height(30.)
                .drag_region(true),
                space(5).height(100.),
            ],
        )
    });
    prepare(&mut ui, &mut Measure::default());
    let button = center(&ui, 3);
    let empty = center(&ui, 4);
    let below = center(&ui, 5);
    // Zone vide de la barre : déplacement ; bouton et contenu hors de la zone : clic.
    assert!(ui.drag_region_at(empty));
    assert!(!ui.drag_region_at(button));
    assert!(!ui.drag_region_at(below));
}

#[test]
fn resize_regions_resize_the_window_from_their_edge_except_over_interactive_targets() {
    use crate::{
        ui::view::{button, column, row, space},
        window::ResizeEdge,
    };
    let mut ui = view_runtime(|| {
        column(
            1,
            [
                space(2).height(100.).drag_region(true),
                row(
                    3,
                    [
                        button(4, "Volume", Msg::Click(4)).width(60.).height(16.),
                        space(5)
                            .width(16.)
                            .height(16.)
                            .resize_region(ResizeEdge::BottomRight),
                    ],
                )
                .height(16.),
            ],
        )
    });
    prepare(&mut ui, &mut Measure::default());
    // La poignée redimensionne par son coin ; le bouton voisin et la zone de déplacement
    // ne redimensionnent pas.
    assert_eq!(
        ui.resize_region_at(center(&ui, 5)),
        Some(ResizeEdge::BottomRight)
    );
    assert_eq!(ui.resize_region_at(center(&ui, 4)), None);
    assert_eq!(ui.resize_region_at(center(&ui, 2)), None);
    assert!(ui.drag_region_at(center(&ui, 2)));
}

#[test]
fn a_resize_region_inside_a_drag_region_keeps_both_gestures() {
    use crate::{
        ui::view::{column, row, space},
        window::ResizeEdge,
    };
    // Micro-lecteur : toute la pochette déplace la fenêtre, la poignée du coin
    // (`data-no-drag data-resize-region`) la redimensionne.
    let mut ui = view_runtime(|| {
        column(
            1,
            [
                space(2).height(100.),
                row(
                    3,
                    [
                        space(4).width(60.).height(16.),
                        space(5)
                            .width(16.)
                            .height(16.)
                            .no_drag(true)
                            .resize_region(ResizeEdge::BottomRight),
                    ],
                )
                .height(16.),
            ],
        )
        .drag_region(true)
    });
    prepare(&mut ui, &mut Measure::default());
    assert_eq!(
        ui.resize_region_at(center(&ui, 5)),
        Some(ResizeEdge::BottomRight)
    );
    assert!(!ui.drag_region_at(center(&ui, 5)));
    for id in [2, 4] {
        assert!(ui.drag_region_at(center(&ui, id)));
        assert_eq!(ui.resize_region_at(center(&ui, id)), None);
    }
}
