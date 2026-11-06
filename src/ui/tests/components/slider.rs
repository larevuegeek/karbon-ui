//! Curseurs : capture, pas, aperçu puis validation, annulation, clavier et
//! accessibilité numérique.
use crate::ui::slider::Slider;
use crate::ui::tests::support::*;
use std::cell::RefCell;

#[test]
fn custom_track_thickness_is_centered_in_both_axes_and_validated() {
    use crate::ui::{scene::DrawCommand, slider::SliderStyle};
    for axis in [Axis::Horizontal, Axis::Vertical] {
        let slider = Slider::new(50., |v| Msg::Value(1, v))
            .axis(axis)
            .style(SliderStyle {
                track_thickness: Some(9.),
                ..Default::default()
            });
        let mut scene = Scene::new(Theme::DARK.background);
        slider.paint(
            &PaintContext {
                rect: RectF::new(0., 0., 200., 80.),
                intrinsic: SizeF::default(),
                theme: Theme::DARK,
                state: VisualState::default(),
            },
            &mut scene,
        );
        let (rect, radius) = scene
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::Fill { rect, radius, .. } => Some((*rect, *radius)),
                _ => None,
            })
            .unwrap();
        near(radius, 4.5);
        if axis == Axis::Horizontal {
            near(layout::height(rect), 9.);
            near((rect.top + rect.bottom) / 2., 40.);
        } else {
            near(layout::width(rect), 9.);
            near((rect.left + rect.right) / 2., 100.);
        }
    }
    for thickness in [0., -1., f32::NAN, f32::INFINITY, 129.] {
        let slider = Slider::new(50., |v| Msg::Value(1, v)).style(SliderStyle {
            track_thickness: Some(thickness),
            ..Default::default()
        });
        assert!(crate::error::rejected(layout::validate(&Element::custom(
            1, slider
        ))));
    }
}

struct Model {
    value: f64,
    position: f64,
    vertical: f64,
    enabled: bool,
    step: f64,
    changes: Vec<(u64, f64)>,
    commits: Vec<(u64, f64)>,
}
struct App(Rc<RefCell<Model>>);
impl Application for App {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        let m = self.0.borrow();
        let first = Slider::new(m.value, |v| Msg::Value(1, v))
            .label("Volume")
            .step(m.step)
            .on_commit(|v| Msg::Commit(2, v));
        Element::column(
            1,
            vec![
                widgets::button(2, "Avant", Msg::Click(9)),
                Element::custom(10, if m.enabled { first } else { first.disabled() }),
                Element::custom(
                    11,
                    Slider::new(m.position, |v| Msg::Value(3, v))
                        .range(0., 200.)
                        .on_commit(|v| Msg::Commit(4, v)),
                ),
                Element::custom(12, Slider::new(50., |v| Msg::Value(5, v)).read_only()),
                Element::custom(13, Slider::new(50., |v| Msg::Value(6, v)).disabled()),
                Element::custom(
                    14,
                    Slider::new(m.vertical, |v| Msg::Value(7, v))
                        .range(0., 1.)
                        .step(0.1)
                        .axis(Axis::Vertical)
                        .on_commit(|v| Msg::Commit(8, v)),
                )
                .width(Length::Px(40.))
                .height(Length::Px(200.)),
            ],
        )
    }
    fn update(&mut self, message: Msg) -> bool {
        let mut m = self.0.borrow_mut();
        match message {
            Msg::Value(a, v) => {
                m.changes.push((a, v));
                match a {
                    1 => m.value = v,
                    7 => m.vertical = v,
                    _ => return false,
                }
                true
            }
            Msg::Commit(a, v) => {
                m.commits.push((a, v));
                if a == 4 {
                    m.position = v;
                }
                true
            }
            _ => false,
        }
    }
}
struct Fixture {
    ui: UiRuntime,
    model: Rc<RefCell<Model>>,
    measure: Measure,
}
impl Fixture {
    fn new() -> Self {
        let model = Rc::new(RefCell::new(Model {
            value: 50.,
            position: 20.,
            vertical: 0.5,
            enabled: true,
            step: 5.,
            changes: Vec::new(),
            commits: Vec::new(),
        }));
        let mut f = Self {
            ui: UiRuntime::new(App(model.clone()), &NoResources).unwrap(),
            model,
            measure: Measure::default(),
        };
        f.prepare();
        f
    }
    fn prepare(&mut self) {
        self.ui
            .prepare(
                ViewContext {
                    height: 800.,
                    ..context()
                },
                ChromeState::default(),
                &mut self.measure,
            )
            .unwrap();
    }
    fn event(&mut self, e: UiEvent) -> EventResult {
        let result = self.ui.event(e);
        self.prepare();
        result
    }
    fn focus(&mut self, id: u64) {
        self.ui
            .accessibility_action(AccessibilityAction::Focus(ElementId(id)))
            .unwrap();
        self.prepare();
    }
    fn down(&mut self, key: Key, repeat: bool) -> EventResult {
        self.event(UiEvent::KeyDown {
            key,
            shift: false,
            repeat,
        })
    }
    fn range(&self, id: u64) -> crate::ui::slider::RangeInfo {
        self.ui
            .accessibility()
            .into_iter()
            .find(|n| n.id == ElementId(id))
            .unwrap()
            .range
            .unwrap()
    }
    fn point(&self, id: u64, fraction: f32) -> PointF {
        let node = self.ui.root().unwrap().find(ElementId(id)).unwrap();
        let r = layout::content_rect(node);
        if node.widget.as_ref().unwrap().slider().unwrap().axis == Axis::Horizontal {
            PointF {
                x: r.left + 10. + (layout::width(r) - 20.) * fraction,
                y: (r.top + r.bottom) / 2.,
            }
        } else {
            PointF {
                x: (r.left + r.right) / 2.,
                y: r.bottom - 10. - (layout::height(r) - 20.) * fraction,
            }
        }
    }
}

#[test]
fn pointer_capture_snaps_clamps_and_commits_once_even_outside_the_control() {
    let mut f = Fixture::new();
    let p = f.point(10, 0.75);
    assert!(f.event(UiEvent::PointerDown(p)).capture);
    assert!(f.ui.has_capture());
    assert_eq!(f.model.borrow().value, 75.);
    assert!(f.model.borrow().commits.is_empty());
    let outside = f.point(10, 1.7);
    f.event(UiEvent::PointerMove(outside));
    assert_eq!(f.model.borrow().value, 100.);
    assert!(f.event(UiEvent::PointerUp(outside)).release);
    assert!(!f.ui.has_capture());
    assert_eq!(f.model.borrow().commits, &[(2, 100.)]);
    let count = f.model.borrow().changes.len();
    let p = f.point(10, 1.);
    f.event(UiEvent::PointerDown(p));
    f.event(UiEvent::PointerUp(p));
    assert_eq!(f.model.borrow().changes.len(), count);
    assert_eq!(f.model.borrow().commits.len(), 1);
}

#[test]
fn grabbing_the_thumb_does_not_jump_and_sub_step_motion_does_not_rebuild() {
    let mut f = Fixture::new();
    let mut p = f.point(10, 0.5);
    p.x += 6.;
    f.event(UiEvent::PointerDown(p));
    assert!(f.model.borrow().changes.is_empty());
    let views = f.ui.stats().views;
    for delta in [0.1, 0.2, 0.4, 0.8] {
        f.event(UiEvent::PointerMove(PointF {
            x: p.x + delta,
            ..p
        }));
    }
    assert!(f.model.borrow().changes.is_empty());
    assert_eq!(f.ui.stats().views, views);
    let end = PointF { x: p.x + 58., ..p };
    f.event(UiEvent::PointerMove(end));
    assert_eq!(f.model.borrow().value, 60.);
    f.event(UiEvent::PointerUp(end));
    assert_eq!(f.model.borrow().commits, &[(2, 60.)]);
}

#[test]
fn deferred_application_value_has_live_preview_then_single_commit() {
    let mut f = Fixture::new();
    let views = f.ui.stats().views;
    let version = f.ui.accessibility_version();
    let p = f.point(11, 0.6);
    f.event(UiEvent::PointerDown(p));
    assert_eq!(f.model.borrow().position, 20.);
    assert_eq!(f.range(11).value, 120.);
    assert_eq!(f.ui.stats().views, views);
    assert_ne!(f.ui.accessibility_version(), version);
    f.event(UiEvent::PointerUp(p));
    assert_eq!(f.model.borrow().position, 120.);
    assert_eq!(f.range(11).value, 120.);
    assert_eq!(f.model.borrow().commits, &[(4, 120.)]);
}

#[test]
fn cancellation_restores_original_without_commit_and_external_disable_releases_capture() {
    for cancel in [
        UiEvent::Cancel,
        UiEvent::FocusChanged(false),
        UiEvent::KeyDown {
            key: Key::Escape,
            shift: false,
            repeat: false,
        },
    ] {
        let mut f = Fixture::new();
        let p = f.point(10, 0.9);
        f.event(UiEvent::PointerDown(p));
        let result = f.event(cancel);
        assert!(result.release);
        assert!(!f.ui.has_capture());
        assert_eq!(f.model.borrow().value, 50.);
        assert!(f.model.borrow().commits.is_empty());
    }
    let mut f = Fixture::new();
    let p = f.point(10, 0.8);
    f.event(UiEvent::PointerDown(p));
    f.model.borrow_mut().enabled = false;
    f.ui.invalidate();
    f.prepare();
    assert!(!f.ui.has_capture());
    assert!(f.event(UiEvent::PointerUp(p)).release);
    assert!(f.model.borrow().commits.is_empty());
    let mut f = Fixture::new();
    let p = f.point(10, 0.8);
    f.event(UiEvent::PointerDown(p));
    f.model.borrow_mut().step = 10.;
    f.ui.invalidate();
    f.prepare();
    assert!(!f.ui.has_capture());
    f.event(UiEvent::PointerUp(p));
    assert!(f.model.borrow().commits.is_empty());
}

#[test]
fn keyboard_repeats_preview_and_commit_waits_for_all_adjustment_keys() {
    let mut f = Fixture::new();
    f.focus(10);
    f.down(Key::Right, false);
    f.down(Key::Right, true);
    assert_eq!(f.model.borrow().value, 60.);
    assert!(f.model.borrow().commits.is_empty());
    f.event(UiEvent::KeyUp(Key::Right));
    assert_eq!(f.model.borrow().commits, &[(2, 60.)]);
    f.down(Key::Right, false);
    f.down(Key::Up, false);
    f.event(UiEvent::KeyUp(Key::Right));
    assert_eq!(f.model.borrow().commits.len(), 1);
    f.event(UiEvent::KeyUp(Key::Up));
    assert_eq!(f.model.borrow().commits.len(), 2);
    assert_eq!(f.model.borrow().value, 70.);
    f.down(Key::End, false);
    f.event(UiEvent::KeyUp(Key::End));
    assert_eq!(f.model.borrow().value, 100.);
    f.down(Key::PageDown, false);
    f.event(UiEvent::KeyUp(Key::PageDown));
    assert_eq!(f.model.borrow().value, 50.);
    f.down(Key::Home, false);
    f.event(UiEvent::KeyUp(Key::Home));
    assert_eq!(f.model.borrow().value, 0.);
    f.down(Key::Right, false);
    let commits = f.model.borrow().commits.len();
    f.down(Key::Tab, false);
    assert_eq!(f.model.borrow().value, 0.);
    assert_eq!(f.model.borrow().commits.len(), commits);
    assert_eq!(f.ui.focused(), Some(ElementId(11)));
}

#[test]
fn vertical_slider_places_minimum_at_bottom_and_read_only_is_focusable() {
    let mut f = Fixture::new();
    let p = f.point(14, 0.9);
    f.event(UiEvent::PointerDown(p));
    f.event(UiEvent::PointerUp(p));
    assert!((f.model.borrow().vertical - 0.9).abs() < 1e-12);
    f.focus(14);
    f.down(Key::Down, false);
    f.event(UiEvent::KeyUp(Key::Down));
    assert!((f.model.borrow().vertical - 0.8).abs() < 1e-12);
    f.focus(12);
    let n = f.model.borrow().changes.len();
    assert!(f.down(Key::Right, false).handled);
    f.event(UiEvent::KeyUp(Key::Right));
    assert_eq!(f.model.borrow().changes.len(), n);
    let p = f.point(12, 0.8);
    f.event(UiEvent::PointerMove(p));
    assert_eq!(f.ui.cursor(), Cursor::Arrow);
    assert!(!f.event(UiEvent::PointerDown(p)).capture);
    f.event(UiEvent::PointerUp(p));
    assert_eq!(f.range(12).value, 50.);
    let p = f.point(13, 0.8);
    assert!(!f.event(UiEvent::PointerDown(p)).capture);
}

#[test]
fn numeric_accessibility_rounds_valid_values_without_focus_change_and_rejects_invalid_requests() {
    use crate::ui::runtime::{AccessibilityAction as A, AccessibilityError as E};
    let mut f = Fixture::new();
    f.focus(2);
    f.ui.accessibility_action(A::SetNumber {
        id: ElementId(10),
        value: 52.6,
    })
    .unwrap();
    f.prepare();
    assert_eq!(f.model.borrow().value, 55.);
    assert_eq!(f.ui.focused(), Some(ElementId(2)));
    assert_eq!(f.model.borrow().commits, &[(2, 55.)]);
    for value in [f64::NAN, f64::INFINITY, -1., 101.] {
        assert_eq!(
            f.ui.accessibility_action(A::SetNumber {
                id: ElementId(10),
                value
            }),
            Err(E::InvalidValue)
        );
    }
    assert_eq!(
        f.ui.accessibility_action(A::SetNumber {
            id: ElementId(12),
            value: 60.
        }),
        Err(E::ReadOnly)
    );
    assert_eq!(
        f.ui.accessibility_action(A::SetNumber {
            id: ElementId(13),
            value: 60.
        }),
        Err(E::Disabled)
    );
    assert_eq!(
        f.ui.accessibility_action(A::SetNumber {
            id: ElementId(2),
            value: 60.
        }),
        Err(E::Unsupported)
    );
    f.ui.accessibility_action(A::SetNumber {
        id: ElementId(11),
        value: 75.,
    })
    .unwrap();
    f.prepare();
    assert_eq!(f.model.borrow().position, 75.);
}

#[test]
fn slider_validation_rejects_invalid_ranges_and_accepts_fractional_and_non_divisible_steps() {
    for slider in [
        Slider::new(f64::NAN, |v| Msg::Value(1, v)),
        Slider::new(101., |v| Msg::Value(1, v)),
        Slider::new(0., |v| Msg::Value(1, v)).range(1., 1.),
        Slider::new(0., |v| Msg::Value(1, v)).step(0.),
        Slider::new(0., |v| Msg::Value(1, v)).step(f64::INFINITY),
        Slider::new(0., |v| Msg::Value(1, v)).range(-f64::MAX, f64::MAX),
        Slider::new(1e20, |v| Msg::Value(1, v))
            .range(1e20, 2e20)
            .step(1.),
    ] {
        assert!(crate::error::rejected(layout::validate(&Element::custom(
            1, slider
        ))));
    }
    let slider = Slider::new(0.5, |v| Msg::Value(1, v))
        .range(-1., 1.)
        .step(0.1);
    assert!(layout::validate(&Element::custom(1, slider)).is_ok());
    let slider = Slider::new(0., |v| Msg::Value(1, v))
        .range(0., 10.)
        .step(3.);
    assert_eq!(slider.snapped(10.), 10.);
    assert_eq!(slider.snapped(8.), 9.);
    assert_eq!(slider.snapped(9.8), 10.);
    let mut f = Fixture::new();
    f.model.borrow_mut().step = 3.;
    f.ui.invalidate();
    f.prepare();
    f.focus(10);
    f.down(Key::Right, false);
    f.event(UiEvent::KeyUp(Key::Right));
    assert_eq!(f.model.borrow().value, 51.);
    f.down(Key::End, false);
    f.event(UiEvent::KeyUp(Key::End));
    f.down(Key::Left, false);
    f.event(UiEvent::KeyUp(Key::Left));
    assert_eq!(f.model.borrow().value, 99.);
}

#[test]
fn pointer_after_keyboard_commits_previous_edit_before_starting_a_new_draft() {
    let mut f = Fixture::new();
    f.focus(11);
    f.down(Key::Right, false);
    assert_eq!(f.range(11).value, 21.);
    let p = f.point(11, 21. / 200.);
    f.event(UiEvent::PointerDown(p));
    assert_eq!(f.model.borrow().position, 21.);
    assert!(f.ui.has_capture());
    f.down(Key::Escape, false);
    assert_eq!(f.model.borrow().position, 21.);
    assert_eq!(f.model.borrow().commits, &[(4, 21.)]);
}
