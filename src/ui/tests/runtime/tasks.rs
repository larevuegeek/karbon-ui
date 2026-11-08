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

/// Pochettes chargées en tâche de fond : l'application remplit des `ImageSlot` sans
/// demander de reconstruction.
struct CoverApp {
    fixed: crate::ui::template::ImageSlot,
    natural: crate::ui::template::ImageSlot,
    /// `width: 100%; height: 100%` dans une case de taille fixée (pochette d'une ligne).
    filling: crate::ui::template::ImageSlot,
}
impl Application for CoverApp {
    type Message = ();
    fn update(&mut self, _: ()) -> bool {
        false
    }
    fn view(&self, _: &ViewContext) -> Element {
        use crate::ui::{template::ImageWidget, view};
        view::column(
            1,
            [
                view::widget(2, ImageWidget::new(&self.fixed))
                    .width(40.)
                    .height(40.),
                view::widget(3, ImageWidget::new(&self.natural)),
                view::column(
                    4,
                    [view::widget(5, ImageWidget::new(&self.filling))
                        .width(Length::Percent(100.))
                        .height(Length::Percent(100.))],
                )
                .width(40.)
                .height(40.),
            ],
        )
    }
    fn task_completed(&mut self, result: TaskResult) -> bool {
        let image =
            std::sync::Arc::new(crate::ui::scene::Image::new(8, 6, vec![255; 8 * 6 * 4]).unwrap());
        match result.id {
            TaskId(1) => self.fixed.set(image),
            TaskId(2) => self.natural.set(image),
            TaskId(3) => self.filling.set(image),
            _ => {}
        }
        false
    }
}
#[test]
fn images_arriving_in_slots_repaint_without_rebuilding_and_lay_out_only_unsized_ones() {
    use crate::ui::scene::DrawCommand;
    let mut ui = UiRuntime::new(
        CoverApp {
            fixed: Default::default(),
            natural: Default::default(),
            filling: Default::default(),
        },
        &NoResources,
    )
    .unwrap();
    let mut measure = Measure::default();
    prepare(&mut ui, &mut measure);
    let images = |ui: &UiRuntime| {
        ui.scene()
            .commands
            .iter()
            .filter(|command| matches!(command, DrawCommand::Image { .. }))
            .count()
    };
    let size = |ui: &UiRuntime, id| {
        // Hauteur : la colonne étire la largeur.
        let r = ui.root().unwrap().find(ElementId(id)).unwrap().bounds();
        r.bottom - r.top
    };
    // Rien d'arrivé : rien de peint, l'image sans taille ne prend pas de place.
    assert_eq!(images(&ui), 0);
    assert_eq!(size(&ui, 3), 0.);
    let before = ui.stats();
    // Image de taille fixée : repeinte, sans reconstruction ni mise en page.
    assert!(ui.task_completed(TaskResult::success(TaskId(1), ())));
    prepare(&mut ui, &mut measure);
    let after = ui.stats();
    assert_eq!(after.views, before.views);
    assert_eq!(after.layouts, before.layouts);
    assert_eq!(after.paints, before.paints + 1);
    assert_eq!(images(&ui), 1);
    // En pourcentage d'une case fixée : sa taille ne dépend pas de l'image non plus.
    assert!(ui.task_completed(TaskResult::success(TaskId(3), ())));
    prepare(&mut ui, &mut measure);
    let after = ui.stats();
    assert_eq!(after.layouts, before.layouts);
    assert_eq!(images(&ui), 2);
    // Image sans taille : mise en page refaite à sa taille naturelle, toujours sans
    // reconstruire la vue.
    assert!(ui.task_completed(TaskResult::success(TaskId(2), ())));
    prepare(&mut ui, &mut measure);
    assert_eq!(ui.stats().views, before.views);
    assert_eq!(ui.stats().layouts, after.layouts + 1);
    assert_eq!(size(&ui, 3), 6.);
    assert_eq!(images(&ui), 3);
    // Plus rien n'arrive : la tâche suivante ne demande pas d'image.
    assert!(!ui.task_completed(TaskResult::success(TaskId(9), ())));
}
