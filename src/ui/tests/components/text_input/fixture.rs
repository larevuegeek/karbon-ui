pub(super) use crate::ui::tests::support::*;
pub(super) use crate::ui::text_input::{ClipboardRequest, EditCommand, InputStyle, TextInput};
pub(super) use std::cell::RefCell;
pub(super) struct InputApp {
    pub(super) value: Rc<RefCell<String>>,
    pub(super) changes: Rc<Cell<usize>>,
    pub(super) submits: Rc<Cell<usize>>,
}
impl Application for InputApp {
    type Message = Msg;
    fn view(&self, ctx: &ViewContext) -> Element {
        Element::column(
            1,
            vec![
                Element::custom(
                    10,
                    TextInput::new(self.value.borrow().clone(), |text| Msg::Text(1, text))
                        .label("Recherche")
                        .placeholder("Un titre")
                        .on_submit(Msg::Click(2))
                        .max_chars(32),
                ),
                Element::custom(
                    11,
                    TextInput::new("Lecture seule", |text| Msg::Text(3, text)).read_only(),
                ),
                Element::custom(
                    12,
                    TextInput::new("Inactif", |text| Msg::Text(4, text)).disabled(),
                ),
                widgets::button(20, "Bouton", Msg::Click(5)),
            ],
        )
        .gap(10.)
        .surface(ctx.theme.surface, 0.)
    }
    fn update(&mut self, message: Msg) -> bool {
        match message {
            Msg::Text(id, value) => {
                assert_eq!(id, 1);
                *self.value.borrow_mut() = value;
                self.changes.set(self.changes.get() + 1);
                true
            }
            Msg::Click(2) => {
                self.submits.set(self.submits.get() + 1);
                false
            }
            _ => false,
        }
    }
}
pub(super) struct Fixture {
    pub(super) ui: UiRuntime,
    pub(super) measure: Measure,
    pub(super) value: Rc<RefCell<String>>,
    pub(super) changes: Rc<Cell<usize>>,
    pub(super) submits: Rc<Cell<usize>>,
}
impl Fixture {
    pub(super) fn new(initial: &str) -> Self {
        let value = Rc::new(RefCell::new(initial.into()));
        let changes = Rc::new(Cell::new(0));
        let submits = Rc::new(Cell::new(0));
        let ui = UiRuntime::new(
            InputApp {
                value: value.clone(),
                changes: changes.clone(),
                submits: submits.clone(),
            },
            &NoResources,
        )
        .unwrap();
        let mut f = Self {
            ui,
            measure: Measure::default(),
            value,
            changes,
            submits,
        };
        f.prepare();
        key(&mut f.ui, Key::Tab);
        f.prepare();
        f
    }
    pub(super) fn prepare(&mut self) {
        prepare(&mut self.ui, &mut self.measure);
    }
    pub(super) fn event(&mut self, event: UiEvent) -> EventResult {
        let result = self.ui.event(event);
        self.prepare();
        result
    }
    pub(super) fn text(&mut self, value: &str) {
        self.event(UiEvent::Text(value.into()));
    }
    pub(super) fn edit(&mut self, command: EditCommand) -> EventResult {
        self.event(UiEvent::Edit(command))
    }
    pub(super) fn key(&mut self, key: Key, shift: bool) {
        self.event(UiEvent::KeyDown {
            key,
            shift,
            repeat: false,
        });
    }
    pub(super) fn state(&self) -> crate::ui::text_input::InputSnapshot {
        self.ui.input_snapshot(ElementId(10)).unwrap()
    }
}
