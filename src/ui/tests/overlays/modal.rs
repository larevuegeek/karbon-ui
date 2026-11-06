//! Dialogues modaux : centrage, piège du focus, fond, Échap et accessibilité.
use crate::ui::tests::support::*;
use crate::ui::{
    modal::Modal,
    select::{Select, SelectOption},
    text_input::TextInput,
};
use std::cell::RefCell;

#[derive(Default)]
struct Model {
    depth: usize,
    background_clicks: usize,
    dismissals: usize,
    text: String,
    remove_opener: bool,
    empty: bool,
    locked: bool,
}
struct App(Rc<RefCell<Model>>);
impl Application for App {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        let m = self.0.borrow();
        let mut page = vec![widgets::button(3, "Autre", Msg::Click(9))];
        if !m.remove_opener {
            page.insert(0, widgets::button(2, "Ouvrir", Msg::Click(1)));
        }
        page.push(widgets::text(4, "Page"));
        let page = Element::scroll(
            5,
            Axis::Vertical,
            Element::column(1, page).height(Length::Px(1400.)),
        );
        let mut dialogs = vec![];
        if m.depth > 0 {
            let content = if m.empty {
                widgets::text(11, "Information")
            } else {
                Element::column(
                    11,
                    vec![
                        Element::custom(
                            12,
                            TextInput::new(m.text.as_str(), |text| Msg::Text(10, text))
                                .label("Nom"),
                        ),
                        Element::custom(
                            13,
                            Select::new(
                                Some(1),
                                vec![SelectOption::new(1, "Un"), SelectOption::new(2, "Deux")],
                                |key| Msg::Select(11, key),
                            ),
                        ),
                        widgets::button(14, "Confirmer", Msg::Click(2)),
                        widgets::button(15, "Fermer", Msg::Click(3)),
                    ],
                )
                .gap(8.)
            };
            dialogs.push(
                Element::modal(
                    10,
                    Modal::new("Dialogue")
                        .on_dismiss(Msg::Click(3))
                        .close_on_escape(!m.locked)
                        .close_on_backdrop(!m.locked)
                        .initial_focus(if m.empty { 11 } else { 12 }),
                    content,
                )
                .width(Length::Px(320.)),
            );
        }
        if m.depth > 1 {
            dialogs.push(
                Element::modal(
                    20,
                    Modal::new("Confirmation")
                        .on_dismiss(Msg::Click(4))
                        .initial_focus(22),
                    Element::column(
                        21,
                        vec![
                            widgets::button(22, "Revenir", Msg::Click(4)),
                            widgets::button(23, "Valider", Msg::Click(3)),
                        ],
                    ),
                )
                .width(Length::Px(260.)),
            );
        }
        Element::layers(100, page, dialogs)
    }
    fn update(&mut self, message: Msg) -> bool {
        let mut m = self.0.borrow_mut();
        match message {
            Msg::Click(1) => m.depth = 1,
            Msg::Click(2) => m.depth = 2,
            Msg::Click(3) => {
                m.depth = 0;
                m.dismissals += 1;
            }
            Msg::Click(4) => m.depth = 1,
            Msg::Click(9) => m.background_clicks += 1,
            Msg::Text(_, text) => m.text = text,
            _ => return false,
        }
        true
    }
}
struct Fixture {
    ui: UiRuntime,
    m: Rc<RefCell<Model>>,
    measure: Measure,
}
impl Fixture {
    fn new() -> Self {
        let m = Rc::new(RefCell::new(Model::default()));
        let mut f = Self {
            ui: UiRuntime::new(App(m.clone()), &NoResources).unwrap(),
            m,
            measure: Measure::default(),
        };
        f.prepare();
        f
    }
    fn prepare(&mut self) {
        prepare(&mut self.ui, &mut self.measure);
    }
    fn event(&mut self, event: UiEvent) -> EventResult {
        let r = self.ui.event(event);
        self.prepare();
        r
    }
    fn key(&mut self, key: Key, shift: bool) -> EventResult {
        self.event(UiEvent::KeyDown {
            key,
            shift,
            repeat: false,
        })
    }
    fn invoke(&mut self, id: u64) {
        self.ui
            .accessibility_action(AccessibilityAction::Invoke(ElementId(id)))
            .unwrap();
        self.prepare();
    }
    fn focus(&mut self, id: u64) {
        self.ui
            .accessibility_action(AccessibilityAction::Focus(ElementId(id)))
            .unwrap();
        self.prepare();
    }
    fn rebuild(&mut self) {
        self.ui.invalidate();
        self.prepare();
    }
    fn open(&mut self) {
        self.focus(2);
        self.invoke(2);
    }
}

#[test]
fn centered_dialog_preserves_page_layout_and_clamps_to_small_windows() {
    let mut f = Fixture::new();
    let before = f.ui.root().unwrap().find(ElementId(5)).unwrap().bounds();
    f.open();
    let page = f.ui.root().unwrap().find(ElementId(5)).unwrap().bounds();
    near(page.left, before.left);
    near(page.bottom, before.bottom);
    let dialog = f.ui.root().unwrap().find(ElementId(10)).unwrap().bounds();
    near((dialog.left + dialog.right) / 2., context().width / 2.);
    near(
        (dialog.top + dialog.bottom) / 2.,
        40. + context().height / 2.,
    );
    let mut ctx = context();
    ctx.width = 180.;
    ctx.height = 120.;
    f.ui.prepare(ctx, ChromeState::default(), &mut f.measure)
        .unwrap();
    let r = f.ui.root().unwrap().find(ElementId(10)).unwrap().bounds();
    assert!(r.left >= 16. && r.right <= 164. && r.top >= 56. && r.bottom <= 144.);
    f.ui.scene().validate().unwrap();
}

#[test]
fn shrinking_dialog_width_remeasures_wrapped_text_before_centering() {
    let mut ui = view_runtime(|| {
        Element::layers(
            1,
            Element::column(2, vec![]),
            vec![Element::modal(
                3,
                Modal::new("Information"),
                widgets::text(
                    4,
                    "Un texte assez long pour prendre plusieurs lignes dans une fenêtre étroite.",
                ),
            )],
        )
    });
    let mut measure = Measure::default();
    prepare(&mut ui, &mut measure);
    let wide = ui.root().unwrap().find(ElementId(3)).unwrap().bounds();
    let mut ctx = context();
    ctx.width = 260.;
    ui.prepare(ctx, ChromeState::default(), &mut measure)
        .unwrap();
    let dialog = ui.root().unwrap().find(ElementId(3)).unwrap();
    assert!(layout::height(dialog.bounds()) > layout::height(wide));
    let text = dialog.find(ElementId(4)).unwrap().bounds();
    assert!(text.bottom <= layout::content_rect(dialog).bottom + 0.01);
}

#[test]
fn focus_is_trapped_in_top_dialog_and_returns_to_opener() {
    let mut f = Fixture::new();
    f.open();
    assert_eq!(f.ui.focused(), Some(ElementId(12)));
    for id in [13, 14, 15, 12] {
        assert!(f.key(Key::Tab, false).handled);
        assert_eq!(f.ui.focused(), Some(ElementId(id)));
    }
    f.key(Key::Tab, true);
    assert_eq!(f.ui.focused(), Some(ElementId(15)));
    f.invoke(15);
    assert_eq!(f.ui.focused(), Some(ElementId(2)));
    assert_eq!(f.m.borrow().dismissals, 1);
}

#[test]
fn nested_dialog_restores_parent_focus_and_keeps_edits_through_rebuilds() {
    let mut f = Fixture::new();
    f.open();
    f.event(UiEvent::Text("Été 😃".into()));
    assert_eq!(f.ui.focused(), Some(ElementId(12)));
    f.focus(14);
    f.invoke(14);
    assert_eq!(f.ui.focused(), Some(ElementId(22)));
    assert!(!f.ui.accessibility().iter().any(|e| e.id == ElementId(12)));
    let mut ctx = context();
    ctx.theme = Theme::LIGHT;
    f.ui.prepare(ctx, ChromeState::default(), &mut f.measure)
        .unwrap();
    assert_eq!(f.ui.focused(), Some(ElementId(22)));
    f.invoke(22);
    assert_eq!(f.ui.focused(), Some(ElementId(14)));
    assert_eq!(f.ui.input_snapshot(ElementId(12)).unwrap().value, "Été 😃");
    f.key(Key::Escape, false);
    assert_eq!(f.ui.focused(), Some(ElementId(2)));
}

#[test]
fn backdrop_dismisses_on_release_and_never_clicks_background() {
    let mut f = Fixture::new();
    f.open();
    let p = center(&f.ui, 3);
    let down = f.event(UiEvent::PointerDown(p));
    assert!(down.handled && down.capture);
    assert_eq!(f.m.borrow().depth, 1);
    assert_eq!(f.ui.focused(), Some(ElementId(12)));
    let up = f.event(UiEvent::PointerUp(p));
    assert!(up.handled && up.release);
    assert_eq!(f.m.borrow().depth, 0);
    assert_eq!(f.m.borrow().background_clicks, 0);
    assert!(!f.ui.has_capture());
    f.event(UiEvent::PointerDown(p));
    f.event(UiEvent::PointerUp(p));
    assert_eq!(f.m.borrow().background_clicks, 1);
}

#[test]
fn locked_dialog_consumes_escape_and_backdrop_without_closing_host() {
    let mut f = Fixture::new();
    f.m.borrow_mut().locked = true;
    f.open();
    assert!(f.key(Key::Escape, false).handled);
    let p = center(&f.ui, 3);
    f.event(UiEvent::PointerDown(p));
    f.event(UiEvent::PointerUp(p));
    assert_eq!(f.m.borrow().depth, 1);
    assert_eq!(f.m.borrow().background_clicks, 0);
    f.m.borrow_mut().locked = false;
    f.rebuild();
    let p = center(&f.ui, 3);
    f.event(UiEvent::PointerDown(p));
    let inside = center(&f.ui, 12);
    f.event(UiEvent::PointerUp(inside));
    assert_eq!(f.m.borrow().depth, 1);
    f.event(UiEvent::FocusChanged(false));
    assert_eq!(f.m.borrow().depth, 1);
}

#[test]
fn escape_closes_select_then_composition_then_dialog_in_order() {
    let mut f = Fixture::new();
    f.open();
    f.focus(13);
    f.key(Key::Enter, false);
    assert!(
        f.ui.accessibility()
            .iter()
            .find(|e| e.id == ElementId(13))
            .unwrap()
            .select
            .as_ref()
            .unwrap()
            .expanded
    );
    f.key(Key::Escape, false);
    assert_eq!(f.m.borrow().depth, 1);
    f.focus(12);
    f.event(UiEvent::CompositionStart);
    f.event(UiEvent::CompositionUpdate {
        value: "仮".into(),
        cursor_utf16: 1,
    });
    assert!(f.ui.focused_input().unwrap().composing);
    f.key(Key::Escape, false);
    assert!(!f.ui.focused_input().unwrap().composing);
    assert_eq!(f.m.borrow().depth, 1);
    assert!(f.key(Key::Escape, false).handled);
    assert_eq!(f.m.borrow().depth, 0);
}

#[test]
fn background_accessible_nodes_and_commands_are_unavailable_while_modal_is_open() {
    let mut f = Fixture::new();
    f.open();
    for action in [
        AccessibilityAction::Focus(ElementId(2)),
        AccessibilityAction::Invoke(ElementId(3)),
    ] {
        assert_eq!(
            f.ui.accessibility_action(action),
            Err(AccessibilityError::Unavailable)
        );
    }
    let nodes = f.ui.accessibility();
    assert_eq!(nodes[0].id, ElementId(10));
    assert_eq!(nodes[0].role, Role::Dialog);
    assert_eq!(nodes[0].parent, None);
    assert_eq!(
        nodes.iter().find(|e| e.id == ElementId(12)).unwrap().parent,
        Some(ElementId(10))
    );
    assert!(nodes.iter().all(|n| n.id.0 >= 10 && n.id.0 <= 15));
}

#[test]
fn programmatic_changes_cancel_pressed_controls_and_absorb_pending_backdrop_release() {
    let mut f = Fixture::new();
    let p = center(&f.ui, 3);
    assert!(f.event(UiEvent::PointerDown(p)).capture);
    f.m.borrow_mut().depth = 1;
    f.rebuild();
    assert!(f.event(UiEvent::PointerUp(p)).release);
    assert_eq!(f.m.borrow().background_clicks, 0);
    let p = center(&f.ui, 3);
    assert!(f.event(UiEvent::PointerDown(p)).capture);
    f.m.borrow_mut().depth = 0;
    f.rebuild();
    assert!(f.ui.has_capture());
    assert!(f.event(UiEvent::PointerUp(p)).release);
    assert_eq!(f.m.borrow().background_clicks, 0);
    assert!(!f.ui.has_capture());
}

#[test]
fn empty_dialog_has_focus_and_removed_opener_uses_a_live_fallback() {
    let mut f = Fixture::new();
    f.m.borrow_mut().empty = true;
    f.open();
    assert_eq!(f.ui.focused(), Some(ElementId(10)));
    assert!(f.key(Key::Tab, false).handled);
    assert_eq!(f.ui.focused(), Some(ElementId(10)));
    f.m.borrow_mut().remove_opener = true;
    f.rebuild();
    f.key(Key::Escape, false);
    assert!(f.ui.focused().is_some());
    assert_ne!(f.ui.focused(), Some(ElementId(2)));
    assert!(f.ui.root().unwrap().find(f.ui.focused().unwrap()).is_some());
}

#[test]
fn modal_blocks_background_scroll_and_hover_reuses_layout() {
    let mut f = Fixture::new();
    f.open();
    let stats = f.ui.stats();
    let p = center(&f.ui, 3);
    assert!(
        f.event(UiEvent::Scroll {
            position: p,
            delta_x: 0.,
            delta_y: 100.
        })
        .handled
    );
    near(scroll_metrics(&f.ui, 5).offset, 0.);
    let p = center(&f.ui, 15);
    f.event(UiEvent::PointerMove(p));
    assert_eq!(f.ui.stats().layouts, stats.layouts);
    assert_eq!(f.ui.stats().views, stats.views);
}

#[test]
fn holding_escape_does_not_dismiss_multiple_layers_or_reach_the_host() {
    let mut f = Fixture::new();
    f.open();
    f.focus(14);
    f.invoke(14);
    f.key(Key::Escape, false);
    assert_eq!(f.m.borrow().depth, 1);
    for _ in 0..3 {
        assert!(
            f.event(UiEvent::KeyDown {
                key: Key::Escape,
                shift: false,
                repeat: true
            })
            .handled
        );
    }
    assert_eq!(f.m.borrow().depth, 1);
    assert!(f.event(UiEvent::KeyUp(Key::Escape)).handled);
    f.key(Key::Escape, false);
    assert_eq!(f.m.borrow().depth, 0);
    assert!(
        f.event(UiEvent::KeyDown {
            key: Key::Escape,
            shift: false,
            repeat: true
        })
        .handled
    );
    f.event(UiEvent::KeyUp(Key::Escape));
    assert!(!f.key(Key::Escape, false).handled);
}

#[test]
fn modal_structure_rejects_nested_layers_or_misplaced_and_unnamed_dialogs() {
    let content = || widgets::text(3, "Texte");
    assert!(crate::error::rejected(layout::validate(&Element::modal(
        2,
        Modal::new("Seul"),
        content()
    ))));
    assert!(crate::error::rejected(layout::validate(&Element::column(
        1,
        vec![Element::layers(2, content(), vec![])]
    ))));
    assert!(crate::error::rejected(layout::validate(&Element::layers(
        1,
        content(),
        vec![widgets::text(2, "Pas une modale")]
    ))));
    assert!(crate::error::rejected(layout::validate(&Element::layers(
        1,
        content(),
        vec![Element::modal(2, Modal::new(" "), widgets::text(4, "X"))]
    ))));
    assert!(crate::error::rejected(layout::validate(&Element::layers(
        1,
        content(),
        vec![Element::modal(
            2,
            Modal::new("Test").initial_focus(99),
            widgets::text(4, "X")
        )]
    ))));
}
