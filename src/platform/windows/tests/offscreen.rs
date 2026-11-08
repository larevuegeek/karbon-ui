//! Services hors écran du backend (`NativeOffscreen`) : protocole de mesure, tâches
//! enchaînées et compteurs du processus.
use crate::{
    application::{Application, Resources, ViewContext},
    diagnostics::{BenchmarkOptions, render_benchmark},
    task::TaskCommand,
    task::{Task, TaskId, TaskResult},
    ui::{
        element::{Axis, Element, Length},
        widgets,
    },
};
use std::{cell::Cell, rc::Rc};
struct App {
    value: Rc<Cell<u32>>,
    tasks: Vec<TaskCommand>,
}
impl Application for App {
    type Message = ();
    fn update(&mut self, _: ()) -> bool {
        false
    }
    fn load(&mut self, _: &dyn Resources) -> crate::Result<()> {
        self.tasks
            .push(TaskCommand::Spawn(Task::new(TaskId(1), |_| 1_u32)));
        Ok(())
    }
    fn task_completed(&mut self, result: TaskResult) -> bool {
        let value = result.into_value::<u32>().unwrap();
        self.value.set(value);
        if value == 1 {
            self.tasks
                .push(TaskCommand::Spawn(Task::new(TaskId(2), |_| 2_u32)));
        }
        true
    }
    fn take_task_commands(&mut self) -> Vec<TaskCommand> {
        std::mem::take(&mut self.tasks)
    }
    fn view(&self, _: &ViewContext) -> Element {
        Element::scroll(
            1,
            Axis::Vertical,
            Element::column(
                2,
                vec![
                    widgets::button(3, format!("{}", self.value.get()), ())
                        .height(Length::Px(1000.)),
                ],
            ),
        )
    }
}
#[test]
fn offscreen_protocol_waits_for_chained_tasks_and_measures_scroll_without_relayout() {
    let value = Rc::new(Cell::new(0));
    let report = render_benchmark(
        BenchmarkOptions {
            width: 320,
            height: 240,
            frames: 4,
            renderer: crate::window::RenderMode::Software,
            ..Default::default()
        },
        App {
            value: Rc::clone(&value),
            tasks: Vec::new(),
        },
    )
    .unwrap();
    assert_eq!(value.get(), 2);
    assert_eq!(report.phases.len(), 6);
    assert!(report.phases[0].after.working_set_bytes > 0);
    assert_eq!(report.phases[1].view_rebuilds, 0);
    assert_eq!(report.phases[1].layouts, 0);
    assert_eq!(report.phases[1].scene_paints, 4);
    // Reconstruction : la vue redemandée à chaque image.
    assert_eq!(report.phases[2].view_rebuilds, 4);
    assert_eq!(report.phases[2].layouts, 4);
    // Animation sur l'horloge simulée, sans clic ni animation : rien à refaire.
    assert_eq!(report.phases[4].name, "animation");
    assert_eq!(report.phases[4].view_rebuilds, 0);
    assert_eq!(report.phases[4].frame_requests, 0);
    assert!(
        report
            .phases
            .iter()
            .all(|phase| phase.p50_ms <= phase.p95_ms && phase.p95_ms <= phase.max_ms)
    );
}
