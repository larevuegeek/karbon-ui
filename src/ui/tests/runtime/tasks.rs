//! Tâches asynchrones : commandes différées et reconstruction sur demande.
use crate::task::{Task, TaskCommand, TaskId, TaskResult};
use crate::ui::tests::support::*;

struct AsyncApp {
    value: String,
    commands: Vec<TaskCommand>,
}
impl Application for AsyncApp {
    type Message = ();
    fn update(&mut self, _: ()) -> bool {
        false
    }
    fn view(&self, _: &ViewContext) -> Element {
        widgets::text(1, self.value.as_str())
    }
    fn take_task_commands(&mut self) -> Vec<TaskCommand> {
        std::mem::take(&mut self.commands)
    }
    fn task_completed(&mut self, result: TaskResult) -> bool {
        if result.id != TaskId(1) {
            return false;
        }
        self.value = result.into_value().unwrap();
        true
    }
}
#[test]
fn task_callbacks_rebuild_only_when_requested_and_preserve_deferred_commands() {
    let mut ui = UiRuntime::new(
        AsyncApp {
            value: "before".into(),
            commands: vec![TaskCommand::Spawn(Task::new(TaskId(1), |_| {
                "after".to_owned()
            }))],
        },
        &NoResources,
    )
    .unwrap();
    let mut measure = Measure::default();
    prepare(&mut ui, &mut measure);
    let before = ui.stats();
    assert_eq!(ui.take_task_commands().len(), 1);
    assert!(ui.take_task_commands().is_empty());
    assert!(!ui.task_completed(TaskResult::success(TaskId(2), "ignored".to_owned())));
    prepare(&mut ui, &mut measure);
    assert_eq!(ui.stats().views, before.views);
    assert!(ui.task_completed(TaskResult::success(TaskId(1), "after".to_owned())));
    assert_eq!(ui.stats().views, before.views); // Reconstruction différée.
    prepare(&mut ui, &mut measure);
    assert_eq!(ui.stats().views, before.views + 1);
    assert!(ui.scene().commands.iter().any(|command| matches!(command,
        crate::ui::scene::DrawCommand::Text { text, .. } if &**text == "after")));
}
