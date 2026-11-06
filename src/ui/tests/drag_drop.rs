//! Glisser-déposer : distinction clic/glissement, annulations et défilement en bord
//! de zone.
use crate::ui::drag_drop::*;
use crate::ui::tests::support::*;
use std::cell::RefCell;
#[derive(Default)]
struct Data {
    clicks: usize,
    drops: Vec<DropEvent>,
    disabled: bool,
    remove_source: bool,
    group: u64,
}
struct App(Rc<RefCell<Data>>);
impl Application for App {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        let data = self.0.borrow();
        let mut children = vec![];
        if !data.remove_source {
            children.push(
                widgets::button(2, "Source", Msg::Click(1))
                    .height(Length::Px(60.))
                    .draggable(DragSource::new(7, 100)),
            );
        }
        let button = Button::new("Cible", Msg::Click(2));
        children.push(
            Element::custom(
                3,
                if data.disabled {
                    button.disabled()
                } else {
                    button
                },
            )
            .height(Length::Px(60.))
            .drop_target(DropTarget::new(data.group, 200, |event| {
                Msg::Drop(3, event)
            })),
        );
        Element::column(1, children).gap(8.)
    }
    fn update(&mut self, message: Msg) -> bool {
        match message {
            Msg::Click(_) => self.0.borrow_mut().clicks += 1,
            Msg::Drop(target, event) => {
                assert_eq!(target, 3);
                self.0.borrow_mut().drops.push(event);
            }
            other => panic!("unexpected message {other:?}"),
        }
        false
    }
}
fn setup() -> (UiRuntime, Measure, Rc<RefCell<Data>>) {
    let data = Rc::new(RefCell::new(Data {
        group: 7,
        ..Default::default()
    }));
    let mut ui = UiRuntime::new(App(Rc::clone(&data)), &NoResources).unwrap();
    let mut m = Measure::default();
    prepare(&mut ui, &mut m);
    (ui, m, data)
}
fn point(ui: &UiRuntime, id: u64, bottom: bool) -> PointF {
    let r = ui.root().unwrap().find(ElementId(id)).unwrap().rect;
    PointF {
        x: r.left + 15.,
        y: if bottom { r.bottom - 12. } else { r.top + 12. },
    }
}
#[test]
fn click_and_drag_are_distinct_and_drop_does_not_rebuild_or_layout_during_motion() {
    let (mut ui, mut m, data) = setup();
    let p = point(&ui, 2, false);
    assert!(ui.event(UiEvent::PointerDown(p)).capture);
    ui.event(UiEvent::PointerMove(PointF {
        x: p.x + 2.,
        y: p.y + 2.,
    }));
    assert!(ui.event(UiEvent::PointerUp(p)).release);
    assert_eq!(data.borrow().clicks, 1);
    let target = point(&ui, 3, true);
    let before = ui.stats();
    ui.event(UiEvent::PointerDown(p));
    ui.event(UiEvent::PointerMove(target));
    prepare(&mut ui, &mut m);
    assert!(ui.has_capture());
    assert_eq!(ui.stats().views, before.views);
    assert_eq!(ui.stats().layouts, before.layouts);
    assert!(ui.event(UiEvent::PointerUp(target)).release);
    assert!(!ui.has_capture());
    assert_eq!(data.borrow().clicks, 1);
    assert_eq!(
        data.borrow().drops,
        [DropEvent {
            group: 7,
            source: 100,
            target: 200,
            position: DropPosition::After
        }]
    );
}
#[test]
fn cancel_escape_focus_loss_incompatible_and_disabled_targets_never_commit() {
    let (mut ui, mut m, data) = setup();
    for mode in 0..5 {
        data.borrow_mut().group = if mode == 3 { 99 } else { 7 };
        data.borrow_mut().disabled = mode == 4;
        ui.invalidate();
        prepare(&mut ui, &mut m);
        let p = point(&ui, 2, false);
        let t = point(&ui, 3, false);
        ui.event(UiEvent::PointerDown(p));
        ui.event(UiEvent::PointerMove(t));
        let reply = match mode {
            0 => ui.event(UiEvent::Cancel),
            1 => ui.event(UiEvent::KeyDown {
                key: Key::Escape,
                shift: false,
                repeat: false,
            }),
            2 => ui.event(UiEvent::FocusChanged(false)),
            _ => ui.event(UiEvent::PointerUp(t)),
        };
        assert!(reply.release);
        ui.event(UiEvent::PointerUp(t));
        ui.event(UiEvent::KeyUp(Key::Escape));
        ui.event(UiEvent::FocusChanged(true));
        assert!(!ui.has_capture());
        assert!(data.borrow().drops.is_empty());
        assert_eq!(data.borrow().clicks, 0);
    }
}
#[test]
fn rebuilding_or_removing_a_source_cancels_the_pending_drop() {
    let (mut ui, mut m, data) = setup();
    let p = point(&ui, 2, false);
    let t = point(&ui, 3, false);
    ui.event(UiEvent::PointerDown(p));
    ui.event(UiEvent::PointerMove(t));
    data.borrow_mut().remove_source = true;
    ui.invalidate();
    prepare(&mut ui, &mut m);
    assert!(ui.event(UiEvent::PointerUp(t)).release);
    assert!(data.borrow().drops.is_empty());
    assert_eq!(data.borrow().clicks, 0);
}
#[test]
fn dragging_near_a_scroll_edge_moves_the_viewport_without_relayout() {
    let mut ui = view_runtime(|| {
        Element::scroll(
            1,
            Axis::Vertical,
            Element::column(
                2,
                (0..40)
                    .map(|i| {
                        widgets::button(10 + i, "Ligne", Msg::Click(i))
                            .height(Length::Px(50.))
                            .draggable(DragSource::new(1, i))
                            .drop_target(DropTarget::new(1, i, |event| Msg::Drop(90, event)))
                    })
                    .collect(),
            ),
        )
    });
    let mut m = Measure::default();
    prepare(&mut ui, &mut m);
    let p = point(&ui, 10, false);
    let viewport = scroll_metrics(&ui, 1).viewport;
    let before = ui.stats();
    ui.event(UiEvent::PointerDown(p));
    ui.event(UiEvent::PointerMove(PointF {
        x: viewport.left + 20.,
        y: viewport.bottom - 5.,
    }));
    prepare(&mut ui, &mut m);
    assert!(scroll_metrics(&ui, 1).offset > 0.);
    assert_eq!(ui.stats().layouts, before.layouts);
    assert!(ui.event(UiEvent::Cancel).release);
}
