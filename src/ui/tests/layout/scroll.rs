//! Zones défilantes : géométrie, molette imbriquée, barre, clavier et conservation de
//! l'état.
use crate::ui::tests::support::*;

/// Texte qui compte ses instances vivantes et ses dessins.
struct TrackedText {
    live: Rc<Cell<usize>>,
    painted: Rc<Cell<usize>>,
}
impl Drop for TrackedText {
    fn drop(&mut self) {
        self.live.set(self.live.get() - 1);
    }
}
impl Widget for TrackedText {
    fn measure(&self, available: SizeF, text: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
        Text::new("Piste").measure(available, text)
    }
    fn paint(&self, ctx: &PaintContext, scene: &mut Scene) {
        self.painted.set(self.painted.get() + 1);
        Text::new("Piste").paint(ctx, scene);
    }
}

#[test]
fn scroll_geometry_clamps_reserves_gutter_and_reclips_hidden_children() {
    let mut ui = view_runtime(|| {
        Element::scroll(
            1,
            Axis::Vertical,
            Element::column(
                2,
                vec![
                    box_node(3, Length::Fill(1.), Length::Px(300.)),
                    widgets::button(4, "Fin", Msg::Click(1)).height(Length::Px(80.)),
                    box_node(5, Length::Fill(1.), Length::Px(300.)),
                ],
            ),
        )
        .padding(10.)
    });
    let mut measure = Measure::default();
    prepare(&mut ui, &mut measure);
    let s = scroll_metrics(&ui, 1);
    near(s.viewport.left, 10.);
    near(s.viewport.top, 50.);
    near(layout::width(s.viewport), 568.);
    near(s.viewport_size(), 380.);
    near(s.max_offset(), 300.);
    // La zone sous le viewport n'est pas cliquable.
    assert!(
        !ui.event(UiEvent::PointerDown(PointF { x: 30., y: 435. }))
            .capture
    );
    assert!(wheel(&mut ui, PointF { x: 30., y: 70. }, 0., 10000.).handled);
    near(scroll_metrics(&ui, 1).offset, 300.);
    let p = center(&ui, 4);
    assert!(ui.event(UiEvent::PointerDown(p)).capture);
    ui.event(UiEvent::Cancel);
    wheel(&mut ui, PointF { x: 30., y: 70. }, 0., -10000.);
    near(scroll_metrics(&ui, 1).offset, 0.);
    assert!(ui.scene().validate().is_ok());
    let mut malformed = Element::scroll(10, Axis::Vertical, Element::column(11, vec![]));
    malformed.children.clear();
    assert!(crate::error::rejected(layout::validate(&malformed)));
}

#[test]
fn nested_wheel_consumes_remaining_delta_in_parent_and_accepts_fractions() {
    let mut ui = view_runtime(|| {
        Element::scroll(
            1,
            Axis::Vertical,
            Element::column(
                2,
                vec![
                    Element::scroll(
                        3,
                        Axis::Vertical,
                        box_node(4, Length::Fill(1.), Length::Px(600.)),
                    )
                    .height(Length::Px(200.)),
                    box_node(5, Length::Fill(1.), Length::Px(600.)),
                ],
            ),
        )
    });
    let mut m = Measure::default();
    prepare(&mut ui, &mut m);
    let p = PointF { x: 20., y: 70. };
    wheel(&mut ui, p, 0., 0.25);
    near(scroll_metrics(&ui, 3).offset, 0.25);
    wheel(&mut ui, p, 0., 449.75);
    near(scroll_metrics(&ui, 3).offset, 400.);
    near(scroll_metrics(&ui, 1).offset, 50.);
    // Remonter consomme d'abord l'enfant, puis le reliquat dans le parent.
    wheel(&mut ui, p, 0., -450.);
    near(scroll_metrics(&ui, 3).offset, 0.);
    near(scroll_metrics(&ui, 1).offset, 0.);
    assert!(!wheel(&mut ui, p, 0., -1.).handled);
    assert!(!wheel(&mut ui, p, f32::NAN, 2.).handled);
}

#[test]
fn scroll_thumb_track_keyboard_and_cancel_share_the_same_limits() {
    let mut ui = view_runtime(|| {
        Element::scroll(
            1,
            Axis::Vertical,
            box_node(2, Length::Fill(1.), Length::Px(2000.)),
        )
    });
    let mut m = Measure::default();
    prepare(&mut ui, &mut m);
    let s = scroll_metrics(&ui, 1);
    let p = PointF {
        x: (s.thumb.left + s.thumb.right) / 2.,
        y: s.thumb.top + 10.,
    };
    assert!(ui.event(UiEvent::PointerDown(p)).capture);
    ui.event(UiEvent::PointerMove(PointF { x: p.x, y: 2000. }));
    near(scroll_metrics(&ui, 1).offset, 1600.);
    assert!(ui.has_capture());
    assert!(
        ui.event(UiEvent::KeyDown {
            key: Key::Escape,
            shift: false,
            repeat: false
        })
        .release
    );
    near(scroll_metrics(&ui, 1).offset, 0.);
    ui.event(UiEvent::PointerDown(p));
    ui.event(UiEvent::PointerMove(PointF {
        x: p.x,
        y: p.y + 80.,
    }));
    ui.event(UiEvent::PointerUp(p));
    near(scroll_metrics(&ui, 1).offset, 400.);
    key(&mut ui, Key::Home);
    near(scroll_metrics(&ui, 1).offset, 0.);
    key(&mut ui, Key::PageDown);
    near(scroll_metrics(&ui, 1).offset, 360.);
    key(&mut ui, Key::End);
    near(scroll_metrics(&ui, 1).offset, 1600.);
    key(&mut ui, Key::PageUp);
    near(scroll_metrics(&ui, 1).offset, 1240.);
    key(&mut ui, Key::Home);
    assert!(
        !ui.event(UiEvent::PointerDown(PointF { x: p.x, y: 350. }))
            .capture
    );
    near(scroll_metrics(&ui, 1).offset, 360.);
}

#[test]
fn horizontal_scroll_keeps_vertical_layout_and_supports_zero_viewports() {
    let mut ui = view_runtime(|| {
        Element::scroll(
            1,
            Axis::Horizontal,
            box_node(2, Length::Px(1600.), Length::Fill(1.)),
        )
    });
    let mut m = Measure::default();
    prepare(&mut ui, &mut m);
    let p = PointF { x: 20., y: 70. };
    wheel(&mut ui, p, 75.5, 0.);
    near(scroll_metrics(&ui, 1).offset, 75.5);
    near(ui.root().unwrap().children[0].bounds().left, -75.5);
    near(ui.root().unwrap().children[0].bounds().top, 40.);
    key(&mut ui, Key::Tab);
    key(&mut ui, Key::Right);
    near(scroll_metrics(&ui, 1).offset, 115.5);
    ui.prepare(
        ViewContext {
            width: 0.,
            height: 0.,
            ..context()
        },
        ChromeState::default(),
        &mut m,
    )
    .unwrap();
    assert!(scroll_metrics(&ui, 1).thumb.left.is_finite());
    assert!(ui.scene().validate().is_ok());
}

#[test]
fn removed_scroll_state_is_discarded_and_resize_saves_the_clamped_offset() {
    let present = Rc::new(Cell::new(true));
    let mut ui = {
        let present = present.clone();
        view_runtime(move || {
            let content = box_node(2, Length::Fill(1.), Length::Px(1600.));
            if present.get() {
                Element::scroll(1, Axis::Vertical, content)
            } else {
                Element::column(1, vec![content])
            }
        })
    };
    let mut m = Measure::default();
    prepare(&mut ui, &mut m);
    let p = PointF { x: 20., y: 70. };
    wheel(&mut ui, p, 0., 300.);
    present.set(false);
    ui.invalidate();
    prepare(&mut ui, &mut m);
    present.set(true);
    ui.invalidate();
    prepare(&mut ui, &mut m);
    near(scroll_metrics(&ui, 1).offset, 0.);
    wheel(&mut ui, p, 0., 300.);
    ui.prepare(
        ViewContext {
            height: 2000.,
            ..context()
        },
        ChromeState::default(),
        &mut m,
    )
    .unwrap();
    near(scroll_metrics(&ui, 1).offset, 0.);
    prepare(&mut ui, &mut m);
    near(scroll_metrics(&ui, 1).offset, 0.);
}

#[test]
fn thousand_rows_draw_only_visible_items_without_remeasure_and_release_widgets() {
    let live = Rc::new(Cell::new(0));
    let painted = Rc::new(Cell::new(0));
    let count = Rc::new(Cell::new(1000));
    let mut ui = {
        let live = live.clone();
        let painted = painted.clone();
        let count = count.clone();
        view_runtime(move || {
            Element::scroll(
                1,
                Axis::Vertical,
                Element::column(
                    2,
                    (0..count.get())
                        .map(|i| {
                            live.set(live.get() + 1);
                            Element::custom(
                                100 + i as u64,
                                TrackedText {
                                    live: live.clone(),
                                    painted: painted.clone(),
                                },
                            )
                            .height(Length::Px(32.))
                        })
                        .collect(),
                ),
            )
        })
    };
    let mut m = Measure::default();
    prepare(&mut ui, &mut m);
    assert_eq!(live.get(), 1000);
    assert!(painted.get() <= 14);
    let calls = m.calls;
    let before = ui.stats();
    let p = PointF { x: 20., y: 70. };
    for _ in 0..100 {
        painted.set(0);
        wheel(&mut ui, p, 0., 48.);
        prepare(&mut ui, &mut m);
        assert!(painted.get() <= 14);
        assert!(ui.scene().commands.len() < 100);
    }
    assert_eq!(m.calls, calls);
    assert_eq!(ui.stats().views, before.views);
    assert_eq!(ui.stats().layouts, before.layouts);
    assert_eq!(ui.stats().scrolls, 100);
    let offset = scroll_metrics(&ui, 1).offset;
    ui.invalidate();
    prepare(&mut ui, &mut m);
    near(scroll_metrics(&ui, 1).offset, offset);
    assert_eq!(live.get(), 1000);
    count.set(2);
    ui.invalidate();
    prepare(&mut ui, &mut m);
    near(scroll_metrics(&ui, 1).offset, 0.);
    assert_eq!(live.get(), 2);
    drop(ui);
    assert_eq!(live.get(), 0);
}
