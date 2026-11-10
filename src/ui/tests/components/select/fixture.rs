pub(super) use crate::ui::tests::support::*;
pub(super) use crate::ui::{
    select::{Select, SelectInfo, SelectOption},
    text_input::TextInput,
};
pub(super) use std::cell::RefCell;

pub(super) struct Model {
    pub(super) selected: Option<u64>,
    pub(super) options: Arc<[SelectOption]>,
    pub(super) enabled: bool,
    pub(super) changes: Vec<u64>,
    pub(super) clicks: usize,
}
pub(super) struct App(pub(super) Rc<RefCell<Model>>);
impl Application for App {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        let m = self.0.borrow();
        let select = Select::new(m.selected, Arc::clone(&m.options), |key| {
            Msg::Select(1, key)
        })
        .label("Source")
        .visible_items(4);
        Element::column(
            1,
            vec![
                widgets::button(2, "Avant", Msg::Click(99)),
                Element::custom(10, if m.enabled { select } else { select.disabled() })
                    .width(Length::Px(220.)),
                widgets::button(3, "Dessous", Msg::Click(99)),
                Element::custom(4, TextInput::new("Texte", |text| Msg::Text(2, text))),
                Element::custom(
                    11,
                    Select::new(Some(0), vec![SelectOption::new(0, "Autre")], |key| {
                        Msg::Select(3, key)
                    }),
                ),
            ],
        )
    }
    fn update(&mut self, message: Msg) -> bool {
        let mut m = self.0.borrow_mut();
        match message {
            Msg::Click(_) => m.clicks += 1,
            Msg::Select(id, key) => {
                assert_eq!(id, 1);
                m.selected = Some(key);
                m.changes.push(key);
            }
            Msg::Text(..) => return false,
            other => panic!("unexpected message {other:?}"),
        }
        true
    }
}
pub(super) struct Fixture {
    pub(super) ui: UiRuntime,
    pub(super) model: Rc<RefCell<Model>>,
    pub(super) measure: Measure,
}
impl Fixture {
    pub(super) fn new() -> Self {
        let mut options = vec![
            SelectOption::new(1, "Alpha"),
            SelectOption::new(2, "Inactif").disabled(),
            SelectOption::new(3, "Bravo"),
            SelectOption::new(4, "Beta"),
        ];
        options.extend((5..25).map(|i| SelectOption::new(i, format!("Source {i}"))));
        let model = Rc::new(RefCell::new(Model {
            selected: Some(1),
            options: options.into(),
            enabled: true,
            changes: Vec::new(),
            clicks: 0,
        }));
        let mut f = Self {
            ui: UiRuntime::new(App(model.clone()), &NoResources).unwrap(),
            model,
            measure: Measure::default(),
        };
        f.prepare();
        f
    }
    pub(super) fn prepare(&mut self) {
        prepare(&mut self.ui, &mut self.measure);
    }
    pub(super) fn event(&mut self, e: UiEvent) -> EventResult {
        let result = self.ui.event(e);
        self.prepare();
        result
    }
    pub(super) fn down(&mut self, key: Key) -> EventResult {
        self.event(UiEvent::KeyDown {
            key,
            shift: false,
            repeat: false,
        })
    }
    pub(super) fn focus(&mut self, id: u64) {
        self.ui
            .accessibility_action(AccessibilityAction::Focus(ElementId(id)))
            .unwrap();
        self.prepare();
    }
    pub(super) fn expand(&mut self) {
        self.ui
            .accessibility_action(AccessibilityAction::ExpandSelect {
                id: ElementId(10),
                expanded: true,
            })
            .unwrap();
        self.prepare();
    }
    pub(super) fn info(&self) -> SelectInfo {
        self.ui
            .accessibility()
            .into_iter()
            .find(|n| n.id == ElementId(10))
            .unwrap()
            .select
            .unwrap()
    }
    pub(super) fn active(&self) -> Option<u64> {
        self.info().options.iter().find(|o| o.focused).map(|o| o.id)
    }
    pub(super) fn option_point(&self, id: u64) -> PointF {
        let r = self
            .info()
            .options
            .into_iter()
            .find(|o| o.id == id)
            .unwrap()
            .clip;
        assert!(layout::height(r) > 0.);
        PointF {
            x: (r.left + r.right) / 2.,
            y: (r.top + r.bottom) / 2.,
        }
    }
}
