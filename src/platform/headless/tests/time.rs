//! Horloge virtuelle et tâches de fond : animations reproductibles, tâches exécutées à
//! la demande et enchaînées.
use super::Loader;
use crate::{
    application::{Application, ViewContext},
    platform::headless::{HeadlessApp, HeadlessOptions},
    ui::{
        animation::{Animation, Easing},
        element::Element,
        prelude::{button, column},
        scene::DrawCommand,
    },
};
use std::time::Duration;

struct Fade;
impl Application for Fade {
    type Message = ();
    fn view(&self, _: &ViewContext) -> Element {
        column(
            "page",
            [button("cta", "Go", ())
                .height(40)
                .animate(Animation::fade_in().duration(100).easing(Easing::Linear))],
        )
        .padding(40)
    }
    fn update(&mut self, _: ()) -> bool {
        false
    }
}
fn opacity(app: &HeadlessApp<Fade>) -> f32 {
    app.scene()
        .commands
        .iter()
        .find_map(|command| match command {
            DrawCommand::PushEffect { opacity, .. } => Some(*opacity),
            _ => None,
        })
        .unwrap_or(1.)
}

#[test]
fn virtual_time_drives_animations_deterministically() {
    let mut app = HeadlessApp::new(Fade).unwrap();
    assert_eq!(opacity(&app), 0.);
    app.advance(Duration::from_millis(50)).unwrap();
    assert!((opacity(&app) - 0.5).abs() < 0.01, "{}", opacity(&app));
    app.advance(Duration::from_millis(60)).unwrap();
    assert_eq!(opacity(&app), 1.);
    assert!(app.runtime().next_wake().is_none());
    let reduced = HeadlessApp::with_options(
        Fade,
        HeadlessOptions {
            reduced_motion: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(opacity(&reduced), 1.);
}

#[test]
fn background_tasks_run_only_when_requested_and_chain() {
    let mut app = HeadlessApp::new(Loader::default()).unwrap();
    assert!(app.contains_text("Loading"));
    app.run_tasks().unwrap();
    assert_eq!(app.app().value, Some(22));
    assert!(app.contains_text("Loaded 22"));
}
