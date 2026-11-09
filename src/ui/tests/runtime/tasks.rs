//! Tâches asynchrones : commandes différées et reconstruction sur demande.
use crate::task::{Task, TaskCommand, TaskId, TaskResult};
use crate::ui::tests::support::*;
use std::time::Duration;

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

struct SlicedApp {
    waiting: Rc<Cell<usize>>,
    shown: Rc<Cell<usize>>,
    views: Rc<Cell<usize>>,
}
impl Application for SlicedApp {
    type Message = ();
    fn update(&mut self, _: ()) -> bool {
        false
    }
    fn view(&self, _: &ViewContext) -> Element {
        self.views.set(self.views.get() + 1);
        widgets::text(1, self.shown.get().to_string())
    }
    fn has_frame_work(&self) -> bool {
        self.waiting.get() > 0
    }
    fn frame_work(&mut self) -> bool {
        self.waiting.set(self.waiting.get() - 1);
        self.shown.set(self.shown.get() + 1);
        true
    }
}

#[test]
fn work_spread_over_frames_advances_one_slice_per_frame_and_then_stops_waking() {
    let (waiting, shown, views) = (
        Rc::new(Cell::new(3)),
        Rc::new(Cell::new(0)),
        Rc::new(Cell::new(0)),
    );
    let mut ui = UiRuntime::new(
        SlicedApp {
            waiting: waiting.clone(),
            shown: shown.clone(),
            views: views.clone(),
        },
        &NoResources,
    )
    .unwrap();
    let mut measure = Measure::default();
    prepare(&mut ui, &mut measure);
    let built = views.get();
    let start = std::time::Instant::now();
    assert!(ui.next_wake().is_some(), "une tranche attend");
    ui.event(UiEvent::Timer(start));
    assert_eq!(shown.get(), 1);
    // Même image : la tranche suivante attend son tour.
    ui.event(UiEvent::Timer(start + Duration::from_millis(2)));
    assert_eq!(shown.get(), 1);
    prepare(&mut ui, &mut measure);
    assert_eq!(views.get(), built + 1, "la tranche reconstruit la vue");
    ui.event(UiEvent::Timer(start + Duration::from_millis(17)));
    ui.event(UiEvent::Timer(start + Duration::from_millis(34)));
    assert_eq!(shown.get(), 3);
    assert_eq!(waiting.get(), 0);
    prepare(&mut ui, &mut measure);
    ui.event(UiEvent::Timer(start + Duration::from_millis(51)));
    assert_eq!(shown.get(), 3, "plus rien à faire");
    assert!(ui.next_wake().is_none(), "plus de réveil");
}

#[test]
fn a_slice_done_before_waiting_for_the_screen_is_the_only_one_of_its_frame() {
    let (waiting, shown) = (Rc::new(Cell::new(3)), Rc::new(Cell::new(0)));
    let mut ui = UiRuntime::new(
        SlicedApp {
            waiting,
            shown: shown.clone(),
            views: Rc::new(Cell::new(0)),
        },
        &NoResources,
    )
    .unwrap();
    prepare(&mut ui, &mut Measure::default());
    let start = std::time::Instant::now();
    ui.begin_frame_work(start);
    assert_eq!(shown.get(), 1);
    // Le réveil qui suit l'attente de l'écran, bien après : même image.
    ui.event(UiEvent::Timer(start + Duration::from_millis(15)));
    assert_eq!(shown.get(), 1);
    ui.begin_frame_work(start + Duration::from_millis(17));
    assert_eq!(shown.get(), 2);
}

#[test]
fn a_rebuild_asked_after_the_wait_for_the_screen_goes_to_the_next_frame() {
    let views = Rc::new(Cell::new(0));
    let counter = views.clone();
    let mut ui = view_runtime(move || {
        counter.set(counter.get() + 1);
        let rows = (0..40u64)
            .map(|n| box_node(10 + n, Length::Fill(1.), Length::Px(50.)))
            .collect();
        Element::scroll(1, Axis::Vertical, Element::column(2, rows))
    });
    let mut measure = Measure::default();
    prepare(&mut ui, &mut measure);
    wheel(&mut ui, PointF { x: 30., y: 70. }, 0., 40.);
    prepare(&mut ui, &mut measure);
    assert!(ui.is_scrolling());
    let built = views.get();
    let start = std::time::Instant::now();
    ui.hold_rebuild(start);
    ui.invalidate();
    ui.prepare_at(context(), ChromeState::default(), &mut measure, start)
        .unwrap();
    assert_eq!(views.get(), built, "retenue pendant cette image");
    assert!(ui.rebuild_pending(), "toujours demandée");
    assert!(ui.next_wake().is_some());
    // Image suivante : la reconstruction passe avant l'attente de l'écran.
    ui.begin_frame_work(start + Duration::from_millis(16));
    ui.prepare_at(
        context(),
        ChromeState::default(),
        &mut measure,
        start + Duration::from_millis(16),
    )
    .unwrap();
    assert_eq!(views.get(), built + 1);
    assert!(!ui.rebuild_pending());
}

#[test]
fn a_held_rebuild_is_not_kept_past_one_frame_when_no_frame_comes() {
    let views = Rc::new(Cell::new(0));
    let counter = views.clone();
    let mut ui = view_runtime(move || {
        counter.set(counter.get() + 1);
        let rows = (0..40u64)
            .map(|n| box_node(10 + n, Length::Fill(1.), Length::Px(50.)))
            .collect();
        Element::scroll(1, Axis::Vertical, Element::column(2, rows))
    });
    let mut measure = Measure::default();
    prepare(&mut ui, &mut measure);
    wheel(&mut ui, PointF { x: 30., y: 70. }, 0., 40.);
    prepare(&mut ui, &mut measure);
    let built = views.get();
    let start = std::time::Instant::now();
    ui.hold_rebuild(start);
    ui.invalidate();
    ui.prepare_at(
        context(),
        ChromeState::default(),
        &mut measure,
        start + Duration::from_millis(25),
    )
    .unwrap();
    assert_eq!(views.get(), built + 1, "la retenue a expiré");
}
