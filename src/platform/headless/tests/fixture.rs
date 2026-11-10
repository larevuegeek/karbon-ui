pub(super) use crate::{
    application::{Application, Resources, ViewContext},
    task::{Task, TaskCommand, TaskId, TaskResult},
    ui::{
        element::Element,
        prelude::{button, column, input, percent, row, text},
    },
    window::{WindowAction, WindowCommand, WindowId},
};

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Msg {
    Add,
    Reset,
    Name(String),
    Submit,
    Close,
}

#[derive(Default)]
pub(super) struct Notes {
    pub(super) count: u32,
    pub(super) name: String,
    pub(super) submitted: Vec<String>,
    pub(super) commands: Vec<WindowCommand>,
}
impl Application for Notes {
    type Message = Msg;
    fn view(&self, ctx: &ViewContext) -> Element {
        column(
            "page",
            [
                text("count", format!("Count: {}", self.count)),
                row(
                    "actions",
                    [
                        button("add", "Add", Msg::Add),
                        button("reset", "Reset", Msg::Reset),
                        button("close", "Close", Msg::Close),
                    ],
                )
                .gap(8.),
                input("name", self.name.as_str(), Msg::Name)
                    .width(percent(50.))
                    .height(36),
                button("submit", "Submit", Msg::Submit),
                text("width", format!("Width: {}", ctx.width)),
            ],
        )
        .gap(10.)
        .padding(20.)
    }
    fn update(&mut self, message: Msg) -> bool {
        match message {
            Msg::Add => self.count += 1,
            Msg::Reset => self.count = 0,
            Msg::Name(name) => self.name = name,
            Msg::Submit => self.submitted.push(std::mem::take(&mut self.name)),
            Msg::Close => self
                .commands
                .push(WindowCommand::Action(WindowId::MAIN, WindowAction::Close)),
        }
        true
    }
    fn take_window_commands(&mut self) -> Vec<WindowCommand> {
        std::mem::take(&mut self.commands)
    }
}

#[derive(Default)]
pub(super) struct Loader {
    pub(super) tasks: Vec<TaskCommand>,
    pub(super) value: Option<u32>,
}
impl Application for Loader {
    type Message = ();
    fn load(&mut self, resources: &dyn Resources) -> crate::Result<()> {
        assert!(resources.decode_image(b"not an image").is_err());
        self.tasks
            .push(TaskCommand::Spawn(Task::new(TaskId(1), |_| 20_u32)));
        Ok(())
    }
    fn view(&self, _: &ViewContext) -> Element {
        text(
            "status",
            self.value
                .map_or_else(|| "Loading".to_owned(), |value| format!("Loaded {value}")),
        )
    }
    fn update(&mut self, _: ()) -> bool {
        false
    }
    fn take_task_commands(&mut self) -> Vec<TaskCommand> {
        std::mem::take(&mut self.tasks)
    }
    fn task_completed(&mut self, result: TaskResult) -> bool {
        let value = result.into_value::<u32>().unwrap();
        if value < 22 {
            self.tasks
                .push(TaskCommand::Spawn(Task::new(TaskId(2), move |_| value + 1)));
        }
        self.value = Some(value);
        true
    }
}
