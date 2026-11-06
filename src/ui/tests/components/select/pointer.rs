//! Liste de choix au pointeur : placement de la liste ancrée, ouverture au relâchement,
//! clic extérieur, survol stable, défilement borné et sortie du clip parent.
use super::*;
use crate::ui::layers::{AnchoredLayer, Placement};

#[test]
fn anchored_layers_choose_available_side_and_stay_inside_the_window() {
    let viewport = RectF::new(4., 44., 292., 250.);
    let below = AnchoredLayer::place(
        RectF::new(20., 60., 120., 40.),
        SizeF {
            width: 180.,
            height: 120.,
        },
        viewport,
    );
    assert_eq!(below.placement, Placement::Below);
    assert!(below.bounds.top >= 104.);
    let above = AnchoredLayer::place(
        RectF::new(230., 240., 100., 40.),
        SizeF {
            width: 180.,
            height: 160.,
        },
        viewport,
    );
    assert_eq!(above.placement, Placement::Above);
    assert!(above.bounds.right <= viewport.right);
    assert!(above.bounds.top >= viewport.top);
    assert!(above.bounds.bottom < 240.);
    let tiny = AnchoredLayer::place(
        RectF::new(0., 40., 100., 40.),
        SizeF {
            width: 300.,
            height: 1000.,
        },
        RectF::new(4., 44., 20., 20.),
    );
    assert!(layout::width(tiny.bounds) <= 20.);
    assert!(layout::height(tiny.bounds) <= 20.);
}

#[test]
fn mouse_opens_after_release_and_popup_wins_over_controls_underneath() {
    let mut f = Fixture::new();
    let p = center(&f.ui, 10);
    assert!(f.event(UiEvent::PointerDown(p)).capture);
    assert!(!f.info().expanded);
    assert!(f.event(UiEvent::PointerUp(p)).release);
    assert!(f.info().expanded);
    assert!(!f.ui.has_capture());
    let p = f.option_point(3);
    assert!(f.event(UiEvent::PointerDown(p)).capture);
    assert_eq!(f.model.borrow().selected, Some(1));
    assert!(f.event(UiEvent::PointerUp(p)).release);
    assert!(!f.info().expanded);
    assert_eq!(f.model.borrow().selected, Some(3));
    assert_eq!(f.model.borrow().changes, &[3]);
    assert_eq!(f.model.borrow().clicks, 0);
    assert_eq!(f.ui.focused(), Some(ElementId(10)));
}

#[test]
fn outside_click_closes_without_activating_the_underlying_button() {
    let mut f = Fixture::new();
    f.expand();
    let p = center(&f.ui, 2);
    let down = f.event(UiEvent::PointerDown(p));
    assert!(down.handled && down.capture);
    assert!(!f.info().expanded);
    assert!(f.event(UiEvent::PointerUp(p)).release);
    assert_eq!(f.model.borrow().clicks, 0);
    assert_eq!(f.model.borrow().selected, Some(1));
    assert!(!f.ui.has_capture());
    f.event(UiEvent::PointerDown(p));
    f.event(UiEvent::PointerUp(p));
    assert_eq!(f.model.borrow().clicks, 1);
}

#[test]
fn stationary_pointer_does_not_replace_keyboard_selection_after_scrolling() {
    let mut f = Fixture::new();
    f.expand();
    let p = f.option_point(3);
    f.event(UiEvent::PointerMove(p));
    assert_eq!(f.active(), Some(3));
    f.down(Key::End);
    assert_eq!(f.active(), Some(24));
    f.event(UiEvent::PointerMove(p));
    assert_eq!(f.active(), Some(24));
    f.down(Key::Enter);
    assert_eq!(f.model.borrow().selected, Some(24));

    f.expand();
    let mut p = f.option_point(23);
    p.x += 1.;
    f.event(UiEvent::PointerMove(p));
    assert_eq!(f.active(), Some(23));
}

#[test]
fn popup_scrolling_is_bounded_and_disabled_click_does_not_choose() {
    let mut f = Fixture::new();
    f.expand();
    let p = f.option_point(2);
    f.event(UiEvent::PointerDown(p));
    f.event(UiEvent::PointerUp(p));
    assert!(f.info().expanded);
    assert!(f.model.borrow().changes.is_empty());
    let r = f.info().popup.unwrap();
    let p = PointF {
        x: r.left + 20.,
        y: r.top + 20.,
    };
    f.event(UiEvent::Scroll {
        position: p,
        delta_x: 0.,
        delta_y: 10000.,
    });
    let info = f.info();
    assert!(layout::height(info.options.last().unwrap().clip) > 0.);
    assert_eq!(f.model.borrow().selected, Some(1));
    f.down(Key::Home);
    assert_eq!(f.active(), Some(1));
    assert!(layout::height(f.info().options[0].clip) > 0.);
    let stats = f.ui.stats();
    let p = f.option_point(1);
    f.event(UiEvent::PointerMove(p));
    let paints = f.ui.stats().paints;
    f.event(UiEvent::PointerMove(p));
    assert_eq!(f.ui.stats().paints, paints);
    assert_eq!(f.ui.stats().layouts, stats.layouts);
    let r = f.info().popup.unwrap();
    let grab = PointF {
        x: r.right - 7.,
        y: r.top + 7.,
    };
    assert!(f.event(UiEvent::PointerDown(grab)).capture);
    let end = PointF {
        x: grab.x,
        y: r.bottom - 8.,
    };
    f.event(UiEvent::PointerMove(end));
    assert!(f.event(UiEvent::PointerUp(end)).release);
    assert!(f.info().expanded);
    assert!(layout::height(f.info().options.last().unwrap().clip) > 0.);
    assert!(f.model.borrow().changes.is_empty());
}

#[test]
fn popup_escapes_parent_clipping_and_closes_on_scroll_or_resize() {
    let mut ui = view_runtime(|| {
        Element::column(
            1,
            vec![
                Element::scroll(
                    2,
                    Axis::Vertical,
                    Element::column(
                        3,
                        vec![
                            widgets::text(4, "Avant").height(Length::Px(60.)),
                            Element::custom(
                                10,
                                Select::new(
                                    Some(0),
                                    (0..20)
                                        .map(|i| SelectOption::new(i, format!("Choix {i}")))
                                        .collect::<Vec<_>>(),
                                    |key| Msg::Select(1, key),
                                )
                                .visible_items(4),
                            ),
                            widgets::text(5, "Après").height(Length::Px(600.)),
                        ],
                    ),
                )
                .height(Length::Px(140.)),
                widgets::button(6, "Sous le panneau", Msg::Click(2)),
            ],
        )
    });
    let mut m = Measure::default();
    prepare(&mut ui, &mut m);
    ui.accessibility_action(AccessibilityAction::ExpandSelect {
        id: ElementId(10),
        expanded: true,
    })
    .unwrap();
    prepare(&mut ui, &mut m);
    let info = ui
        .accessibility()
        .into_iter()
        .find(|n| n.id == ElementId(10))
        .unwrap()
        .select
        .unwrap();
    assert!(info.popup.unwrap().bottom > scroll_metrics(&ui, 2).viewport.bottom);
    ui.scene().validate().unwrap();
    let view = scroll_metrics(&ui, 2).viewport;
    assert!(
        wheel(
            &mut ui,
            PointF {
                x: view.left + 20.,
                y: view.top + 10.
            },
            0.,
            30.
        )
        .handled
    );
    prepare(&mut ui, &mut m);
    assert!(
        !ui.accessibility()
            .iter()
            .find(|n| n.id == ElementId(10))
            .unwrap()
            .select
            .as_ref()
            .unwrap()
            .expanded
    );
    ui.accessibility_action(AccessibilityAction::ExpandSelect {
        id: ElementId(10),
        expanded: true,
    })
    .unwrap();
    prepare(&mut ui, &mut m);
    ui.prepare(
        ViewContext {
            width: 500.,
            ..context()
        },
        ChromeState::default(),
        &mut m,
    )
    .unwrap();
    assert!(
        !ui.accessibility()
            .iter()
            .find(|n| n.id == ElementId(10))
            .unwrap()
            .select
            .as_ref()
            .unwrap()
            .expanded
    );
}
