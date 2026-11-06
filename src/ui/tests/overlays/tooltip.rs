//! Infobulles : délai, focus clavier, Échap, portée des calques et accessibilité.
use crate::ui::tests::support::*;
use crate::ui::{
    modal::Modal,
    popover::{Anchor, Popover},
    tooltip::{Tooltip, TooltipStyle},
    widgets::{Select, SelectOption},
};
use std::{cell::RefCell, time::Duration};

struct Model {
    clicks: usize,
    modal: bool,
    popup: bool,
    removed: bool,
    disabled: bool,
    text: Arc<str>,
}
struct App(Rc<RefCell<Model>>);
impl Application for App {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        let m = self.0.borrow();
        let mut content = vec![];
        if !m.removed {
            let button = widgets::Button::new("Ouvrir", Msg::Click(1));
            content.push(
                Element::custom(
                    2,
                    if m.disabled {
                        button.disabled()
                    } else {
                        button
                    },
                )
                .tooltip_with(Tooltip::new(Arc::clone(&m.text)).max_width(180.)),
            );
        }
        content.push(widgets::button(3, "Dessous", Msg::Click(2)));
        content.push(widgets::text(4, "Information").tooltip("Aide du texte"));
        content.push(
            Element::custom(
                5,
                Select::new(Some(1), vec![SelectOption::new(1, "Un")], |key| {
                    Msg::Select(5, key)
                }),
            )
            .tooltip("Choisir une valeur"),
        );
        let page = Element::column(1, content).gap(4.);
        let mut overlays = vec![];
        if m.modal {
            overlays.push(Element::modal(
                10,
                Modal::new("Dialogue")
                    .on_dismiss(Msg::Click(3))
                    .initial_focus(12),
                Element::column(
                    11,
                    vec![
                        widgets::button(12, "Fermer", Msg::Click(3)).tooltip("Fermer ce dialogue"),
                    ],
                ),
            ));
        }
        if m.popup {
            overlays.push(Element::popover(
                20,
                Popover::new("Options", Anchor::Element(ElementId(2)), Msg::Click(4))
                    .initial_focus(22),
                Element::column(
                    21,
                    vec![
                        widgets::button(22, "Terminer", Msg::Click(4)).tooltip("Fermer ce panneau"),
                    ],
                ),
            ));
        }
        Element::layers(99, page, overlays)
    }
    fn update(&mut self, message: Msg) -> bool {
        let mut m = self.0.borrow_mut();
        match message {
            Msg::Click(1) => m.modal = true,
            Msg::Click(2) => m.clicks += 1,
            Msg::Click(3) => m.modal = false,
            Msg::Click(4) => m.popup = false,
            _ => {}
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
        let m = Rc::new(RefCell::new(Model {
            clicks: 0,
            modal: false,
            popup: false,
            removed: false,
            disabled: false,
            text: "Ouvrir le dialogue de démonstration avec un texte sur plusieurs lignes.".into(),
        }));
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
    fn event(&mut self, e: UiEvent) -> EventResult {
        let r = self.ui.event(e);
        self.prepare();
        r
    }
    fn key(&mut self, key: Key) -> EventResult {
        self.event(UiEvent::KeyDown {
            key,
            shift: false,
            repeat: false,
        })
    }
    fn hover(&mut self, id: u64) {
        let p = center(&self.ui, id);
        self.event(UiEvent::PointerMove(p));
    }
    fn show(&mut self) {
        let deadline = self.ui.next_wake().expect("Délai attendu");
        self.event(UiEvent::Timer(deadline));
        assert!(self.ui.tooltip().is_some());
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
}

#[test]
fn delayed_hover_schedules_one_wake_and_does_not_remeasure_the_page() {
    let mut f = Fixture::new();
    let stats = f.ui.stats();
    let calls = f.measure.calls;
    f.hover(2);
    let deadline = f.ui.next_wake().unwrap();
    assert!(f.ui.tooltip().is_none());
    f.event(UiEvent::Timer(deadline - Duration::from_millis(1)));
    assert!(f.ui.tooltip().is_none());
    f.hover(2);
    assert_eq!(f.ui.next_wake(), Some(deadline));
    assert_eq!(f.measure.calls, calls);
    f.show();
    assert!(f.ui.next_wake().is_none());
    assert_eq!(f.ui.stats().layouts, stats.layouts);
    assert_eq!(f.ui.stats().views, stats.views);
    let measured = f.measure.calls;
    let paints = f.ui.stats().paints;
    f.hover(2);
    f.event(UiEvent::Timer(deadline + Duration::from_secs(100)));
    assert_eq!(f.measure.calls, measured);
    assert_eq!(f.ui.stats().paints, paints);
}

#[test]
fn noninteractive_text_and_disabled_widgets_also_offer_help() {
    let mut f = Fixture::new();
    f.hover(4);
    f.show();
    assert_eq!(f.ui.tooltip().unwrap().owner, ElementId(4));
    f.event(UiEvent::PointerLeave);
    assert!(f.ui.tooltip().is_none());
    f.m.borrow_mut().disabled = true;
    f.rebuild();
    f.hover(2);
    f.show();
    assert_eq!(f.ui.tooltip().unwrap().owner, ElementId(2));
    assert!(
        !f.ui
            .accessibility()
            .iter()
            .find(|n| n.id == ElementId(2))
            .unwrap()
            .enabled
    );
}

#[test]
fn keyboard_focus_shows_help_without_changing_focus_or_tab_order() {
    let mut f = Fixture::new();
    f.hover(2);
    f.show();
    f.key(Key::Tab);
    f.key(Key::Tab);
    f.key(Key::Tab);
    let parked = center(&f.ui, 2);
    f.event(UiEvent::PointerMove(parked));
    f.show();
    assert_eq!(f.ui.tooltip().unwrap().owner, ElementId(5));
    f.event(UiEvent::PointerMove(PointF {
        x: parked.x + 1.,
        ..parked
    }));
    f.show();
    assert_eq!(f.ui.tooltip().unwrap().owner, ElementId(2));
    f.event(UiEvent::PointerLeave);
    f.focus(2);
    f.show();
    assert_eq!(f.ui.focused(), Some(ElementId(2)));
    f.key(Key::Tab);
    assert_eq!(f.ui.focused(), Some(ElementId(3)));
    assert!(f.ui.tooltip().is_none());
    assert!(f.ui.next_wake().is_none());
    f.key(Key::Tab);
    assert_eq!(f.ui.focused(), Some(ElementId(5)));
    f.show();
    assert_eq!(f.ui.tooltip().unwrap().owner, ElementId(5));
}

#[test]
fn escape_hides_only_tooltip_and_suppresses_until_leaving_its_owner() {
    let mut f = Fixture::new();
    f.hover(2);
    f.show();
    assert!(f.key(Key::Escape).handled);
    assert!(f.ui.tooltip().is_none());
    assert!(f.ui.next_wake().is_none());
    f.hover(2);
    assert!(f.ui.next_wake().is_none());
    f.event(UiEvent::KeyUp(Key::Escape));
    f.event(UiEvent::PointerLeave);
    f.hover(2);
    assert!(f.ui.next_wake().is_some());
}

#[test]
fn tooltip_and_gap_can_be_hovered_and_clicks_reach_the_control_underneath() {
    let mut f = Fixture::new();
    f.hover(2);
    f.show();
    let info = f.ui.tooltip().unwrap();
    let anchor = f.ui.root().unwrap().find(ElementId(2)).unwrap().bounds();
    let gap = PointF {
        x: info.bounds.left + 15.,
        y: (anchor.bottom + info.bounds.top) / 2.,
    };
    f.event(UiEvent::PointerMove(gap));
    assert!(f.ui.tooltip().is_some());
    let p = PointF {
        x: info.bounds.left + 15.,
        y: info.bounds.top + 8.,
    };
    f.event(UiEvent::PointerMove(p));
    assert!(f.ui.tooltip().is_some());
    assert!(f.event(UiEvent::PointerDown(p)).capture);
    assert!(f.ui.tooltip().is_none());
    f.event(UiEvent::PointerUp(p));
    assert_eq!(f.m.borrow().clicks, 1);
    assert!(f.ui.next_wake().is_none());
}

#[test]
fn cancel_scroll_and_focus_loss_cancel_pending_wake_and_release_text() {
    let mut f = Fixture::new();
    f.hover(2);
    let stale = f.ui.next_wake().unwrap();
    f.event(UiEvent::FocusChanged(false));
    assert!(f.ui.next_wake().is_none());
    f.event(UiEvent::Timer(stale));
    assert!(f.ui.tooltip().is_none());
    f.event(UiEvent::FocusChanged(true));
    f.hover(2);
    f.show();
    let p = center(&f.ui, 2);
    f.event(UiEvent::Scroll {
        position: p,
        delta_x: 0.,
        delta_y: 20.,
    });
    assert!(f.ui.tooltip().is_none());
    assert!(f.ui.next_wake().is_none());
    f.event(UiEvent::PointerLeave);
    f.hover(2);
    f.m.borrow_mut().removed = true;
    f.rebuild();
    assert!(f.ui.next_wake().is_none());
    assert!(f.ui.tooltip().is_none());
    assert_eq!(Arc::strong_count(&f.m.borrow().text), 1);
}

#[test]
fn modal_and_popover_scopes_hide_background_help_and_escape_closes_in_order() {
    let mut f = Fixture::new();
    // Interaction au clavier : les couches ouvertes ensuite montrent le focus et son aide.
    f.focus(2);
    f.show();
    f.m.borrow_mut().modal = true;
    f.rebuild();
    assert!(f.ui.tooltip().is_none());
    f.show();
    assert_eq!(f.ui.tooltip().unwrap().owner, ElementId(12));
    f.key(Key::Escape);
    assert!(f.m.borrow().modal);
    assert!(f.ui.tooltip().is_none());
    f.event(UiEvent::KeyUp(Key::Escape));
    f.key(Key::Escape);
    assert!(!f.m.borrow().modal);
    f.m.borrow_mut().popup = true;
    f.rebuild();
    f.show();
    assert_eq!(f.ui.tooltip().unwrap().owner, ElementId(22));
    f.key(Key::Escape);
    assert!(f.m.borrow().popup);
    f.event(UiEvent::KeyUp(Key::Escape));
    f.key(Key::Escape);
    assert!(!f.m.borrow().popup);
}

#[test]
fn select_and_pointer_capture_block_tooltips_until_interaction_finishes() {
    let mut f = Fixture::new();
    f.focus(5);
    f.show();
    f.key(Key::Enter);
    assert!(f.ui.tooltip().is_none());
    assert!(f.ui.next_wake().is_none());
    f.event(UiEvent::CaretTick);
    assert!(f.ui.next_wake().is_none());
    f.key(Key::Escape);
    assert!(f.ui.next_wake().is_some());
    f.event(UiEvent::PointerLeave);
    let p = center(&f.ui, 3);
    f.event(UiEvent::PointerDown(p));
    f.hover(2);
    assert!(f.ui.next_wake().is_none());
    f.event(UiEvent::Cancel);
}

#[test]
fn tooltip_uses_theme_styles_clipping_and_resets_when_geometry_changes() {
    let mut ui = view_runtime(|| {
        Element::column(
            1,
            vec![
                widgets::button(2, "Aide", Msg::Click(1)).tooltip_with(
                    Tooltip::new("Texte de l’aide")
                        .delay(Duration::ZERO)
                        .max_width(120.)
                        .style(TooltipStyle {
                            background: Some(crate::ui::theme::Color::rgb(20, 30, 40)),
                            ..TooltipStyle::default()
                        }),
                ),
            ],
        )
    });
    let mut measure = Measure::default();
    prepare(&mut ui, &mut measure);
    let p = center(&ui, 2);
    ui.event(UiEvent::PointerMove(p));
    prepare(&mut ui, &mut measure);
    let info = ui.tooltip().unwrap();
    assert!(layout::width(info.bounds) <= 120.);
    assert!(ui.scene().commands.iter().any(|c| matches!(c, crate::ui::scene::DrawCommand::Fill { color, .. } if *color == crate::ui::theme::Color::rgb(20,30,40))));
    ui.scene().validate().unwrap();
    let mut ctx = context();
    ctx.width = 100.;
    ctx.height = 90.;
    ctx.theme = Theme::LIGHT;
    ui.prepare(ctx, ChromeState::default(), &mut measure)
        .unwrap();
    if let Some(info) = ui.tooltip() {
        assert!(info.bounds.left >= 4. && info.bounds.right <= 96.);
    }
}

#[test]
fn accessible_help_is_available_before_display_and_invalid_configuration_is_rejected() {
    let mut f = Fixture::new();
    let nodes = f.ui.accessibility();
    assert!(
        nodes
            .iter()
            .find(|n| n.id == ElementId(2))
            .unwrap()
            .help
            .contains("Ouvrir le dialogue")
    );
    assert!(f.ui.tooltip().is_none());
    f.focus(5);
    f.show();
    assert_eq!(
        f.ui.accessibility_action(AccessibilityAction::ChooseOption {
            id: ElementId(5),
            option: 999
        }),
        Err(AccessibilityError::InvalidValue)
    );
    assert!(f.ui.tooltip().is_some());
    for tip in [
        Tooltip::new(" "),
        Tooltip::new("Texte").max_width(f32::NAN),
        Tooltip::new("Texte").delay(Duration::from_secs(61)),
        Tooltip::new("Texte").style(TooltipStyle {
            font_size: 0.,
            ..TooltipStyle::default()
        }),
    ] {
        assert!(crate::error::rejected(layout::validate(
            &widgets::button(1, "Test", Msg::Click(1)).tooltip_with(tip)
        )));
    }
}

#[test]
fn closing_a_popover_with_the_pointer_restores_focus_without_showing_its_help() {
    let mut f = Fixture::new();
    f.focus(2);
    f.m.borrow_mut().popup = true;
    f.rebuild();
    // Choix à la souris dans le panneau : il se ferme et rend le focus à son ancre.
    let p = center(&f.ui, 22);
    f.event(UiEvent::PointerMove(p));
    f.event(UiEvent::PointerDown(p));
    f.event(UiEvent::PointerUp(p));
    assert!(!f.m.borrow().popup);
    assert_eq!(f.ui.focused(), Some(ElementId(2)));
    if let Some(deadline) = f.ui.next_wake() {
        f.event(UiEvent::Timer(deadline + Duration::from_secs(2)));
    }
    assert!(
        f.ui.tooltip().is_none_or(|tip| tip.owner != ElementId(2)),
        "l'aide de l'ancre ne s'affiche pas après un choix à la souris"
    );
    // Au clavier, la fermeture par Échap montre toujours le focus et son aide.
    f.m.borrow_mut().popup = true;
    f.rebuild();
    f.key(Key::Escape);
    assert!(!f.m.borrow().popup);
    f.show();
    assert_eq!(f.ui.tooltip().unwrap().owner, ElementId(2));
}

#[test]
fn a_popover_opened_after_a_pointer_interaction_does_not_show_its_help() {
    let mut f = Fixture::new();
    let p = center(&f.ui, 3);
    f.event(UiEvent::PointerMove(p));
    f.event(UiEvent::PointerDown(p));
    f.event(UiEvent::PointerUp(p));
    f.m.borrow_mut().popup = true;
    f.rebuild();
    assert_eq!(
        f.ui.focused(),
        Some(ElementId(22)),
        "le focus passe dans le panneau"
    );
    if let Some(deadline) = f.ui.next_wake() {
        f.event(UiEvent::Timer(deadline + Duration::from_secs(2)));
    }
    assert!(
        f.ui.tooltip().is_none_or(|tip| tip.owner != ElementId(22)),
        "l'aide de l'élément focalisé ne s'affiche pas après un clic"
    );
}
