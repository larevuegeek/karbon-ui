//! Popovers : application d'essai partagée par les tests de panneaux ancrés et de
//! menus contextuels.
mod menu;
mod panel;
use crate::ui::tests::support::*;
use crate::ui::{
    modal::Modal,
    popover::{Anchor, ContextMenuRequest, Popover},
    widgets::{MenuItem, MenuSeparator, Slider, TextInput},
};
use std::cell::RefCell;

struct Model {
    popup: Option<(Anchor, bool)>,
    modal: bool,
    missing: bool,
    hidden: bool,
    disabled: bool,
    long: bool,
    actions: Vec<u64>,
    requests: Vec<ContextMenuRequest>,
    text: String,
    value: f64,
}
impl Default for Model {
    fn default() -> Self {
        Self {
            popup: None,
            modal: false,
            missing: false,
            hidden: false,
            disabled: false,
            long: false,
            actions: vec![],
            requests: vec![],
            text: String::new(),
            value: 50.,
        }
    }
}
struct App(Rc<RefCell<Model>>);
impl Application for App {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        let m = self.0.borrow();
        let mut page = vec![];
        if !m.missing {
            let button = widgets::Button::new("Réglages", Msg::Click(1));
            let button = if m.disabled {
                button.disabled()
            } else {
                button
            };
            page.push(
                Element::custom(2, button)
                    .height(Length::Px(if m.hidden { 0. } else { 40. }))
                    .context_menu(|request| Msg::Menu(3, request)),
            );
        }
        page.push(widgets::button(3, "Autre", Msg::Click(9)));
        page.push(
            widgets::button(5, "Actions", Msg::Click(2))
                .context_menu(|request| Msg::Menu(3, request)),
        );
        let page = Element::scroll(
            6,
            Axis::Vertical,
            Element::column(1, page)
                .context_menu(|request| Msg::Menu(30, request))
                .height(Length::Px(1200.)),
        );
        let mut layers = vec![];
        if m.modal {
            layers.push(Element::modal(
                20,
                Modal::new("Dialogue")
                    .on_dismiss(Msg::Click(6))
                    .initial_focus(22),
                Element::column(
                    21,
                    vec![
                        widgets::button(22, "Réglages du dialogue", Msg::Click(1))
                            .context_menu(|request| Msg::Menu(3, request)),
                        widgets::button(23, "Fermer le dialogue", Msg::Click(6)),
                    ],
                ),
            ));
        }
        if let Some((anchor, menu)) = m.popup {
            let config = Popover::new(
                if menu { "Actions" } else { "Réglages" },
                anchor,
                Msg::Click(4),
            );
            let (config, content) = if menu {
                let content = if m.long {
                    Element::scroll(
                        120,
                        Axis::Vertical,
                        Element::column(
                            121,
                            (0..30)
                                .map(|i| {
                                    Element::custom(
                                        200 + i,
                                        MenuItem::new(format!("Action {i}"), Msg::Click(11)),
                                    )
                                })
                                .collect(),
                        ),
                    )
                } else {
                    Element::column(
                        110,
                        vec![
                            Element::custom(
                                111,
                                MenuItem::new("Lire", Msg::Click(11)).shortcut("Entrée"),
                            ),
                            Element::custom(
                                112,
                                MenuItem::new("Indisponible", Msg::Click(12)).disabled(),
                            ),
                            Element::custom(113, MenuSeparator),
                            Element::custom(
                                114,
                                MenuItem::new("Ouvrir un dialogue", Msg::Click(12)),
                            ),
                        ],
                    )
                };
                (config.menu(), content)
            } else {
                (
                    config.initial_focus(102),
                    Element::column(
                        101,
                        vec![
                            Element::custom(
                                102,
                                TextInput::new(m.text.as_str(), |text| Msg::Text(15, text))
                                    .label("Nom"),
                            ),
                            Element::custom(
                                104,
                                Slider::new(m.value, |value| Msg::Value(16, value)),
                            ),
                            widgets::button(105, "Fermer", Msg::Click(4)),
                        ],
                    )
                    .gap(8.),
                )
            };
            layers.push(
                Element::popover(100, config, content)
                    .width(Length::Px(260.))
                    .height(if m.long {
                        Length::Px(170.)
                    } else {
                        Length::Auto
                    }),
            );
        }
        Element::layers(999, page, layers)
    }
    fn update(&mut self, message: Msg) -> bool {
        let mut m = self.0.borrow_mut();
        let action = match message {
            Msg::Click(action) => action,
            Msg::Menu(action, request) => {
                m.actions.push(action);
                m.requests.push(request);
                m.popup = Some((Anchor::Point(request.position), true));
                return true;
            }
            Msg::Text(_, text) => {
                m.text = text;
                return true;
            }
            Msg::Value(_, value) => {
                m.value = value;
                return true;
            }
            other => panic!("unexpected message {other:?}"),
        };
        m.actions.push(action);
        match action {
            1 => {
                m.popup = Some((
                    Anchor::Element(ElementId(if m.modal { 22 } else { 2 })),
                    false,
                ))
            }
            2 => m.popup = Some((Anchor::Element(ElementId(5)), true)),
            4 => m.popup = None,
            6 => m.modal = false,
            12 => m.modal = true,
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
    fn event(&mut self, e: UiEvent) -> EventResult {
        let r = self.ui.event(e);
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
    fn focus(&mut self, id: u64) {
        self.ui
            .accessibility_action(AccessibilityAction::Focus(ElementId(id)))
            .unwrap();
        self.prepare();
    }
    fn invoke(&mut self, id: u64) {
        self.ui
            .accessibility_action(AccessibilityAction::Invoke(ElementId(id)))
            .unwrap();
        self.prepare();
    }
    fn open(&mut self, menu: bool) {
        let id = if menu { 5 } else { 2 };
        self.focus(id);
        self.invoke(id);
    }
    fn rebuild(&mut self) {
        self.ui.invalidate();
        self.prepare();
    }
}
