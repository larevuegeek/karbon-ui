//! Lire un dossier hors du thread UI, avec annulation et résultats typés.
use karbon_ui::{
    application::{Application, ViewContext},
    platform,
    task::{Task, TaskCommand, TaskId, TaskResult},
    ui::{element::Element, style::SurfaceStyle, widgets},
    window::WindowOptions,
};
use std::{io, path::PathBuf};
#[derive(Debug, Clone)]
enum Msg {
    Read,
    Cancel,
}
struct DirectoryApp {
    directory: PathBuf,
    status: String,
    tasks: Vec<TaskCommand>,
}
impl Application for DirectoryApp {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        Element::column(
            1,
            vec![
                widgets::text(2, format!("Dossier : {}", self.directory.display())),
                widgets::text(3, self.status.as_str()),
                widgets::button(4, "Lire / relancer", Msg::Read),
                widgets::button(5, "Annuler", Msg::Cancel),
            ],
        )
        .styled(SurfaceStyle::default())
    }
    fn update(&mut self, message: Msg) -> bool {
        match message {
            Msg::Read => {
                let directory = self.directory.clone();
                self.status = "Lecture…".into();
                self.tasks.push(TaskCommand::Spawn(Task::new(
                    TaskId(1),
                    move |cancel| -> io::Result<Vec<String>> {
                        let mut names = Vec::new();
                        for entry in std::fs::read_dir(directory)? {
                            if cancel.is_cancelled() {
                                break;
                            }
                            names.push(entry?.file_name().to_string_lossy().into_owned());
                        }
                        Ok(names)
                    },
                )));
            }
            Msg::Cancel => {
                self.status = "Lecture annulée.".into();
                self.tasks.push(TaskCommand::Cancel(TaskId(1)));
            }
        }
        true
    }
    fn take_task_commands(&mut self) -> Vec<TaskCommand> {
        std::mem::take(&mut self.tasks)
    }
    fn task_completed(&mut self, result: TaskResult) -> bool {
        self.status = match result.into_value::<io::Result<Vec<String>>>() {
            Ok(Ok(names)) => format!(
                "{} entrées : {}",
                names.len(),
                names.into_iter().take(6).collect::<Vec<_>>().join(", ")
            ),
            Ok(Err(error)) => format!("Lecture impossible : {error}"),
            Err(error) => format!("Travail impossible : {error}"),
        };
        true
    }
}
fn main() -> karbon_ui::Result<()> {
    platform::run(
        WindowOptions {
            title: "Karbon UI - travail en arrière-plan",
            width: 680,
            height: 420,
            min_width: 420,
            min_height: 300,
            ..WindowOptions::DEFAULT
        },
        DirectoryApp {
            directory: std::env::current_dir()?,
            status: "Prêt.".into(),
            tasks: Vec::new(),
        },
    )
}
