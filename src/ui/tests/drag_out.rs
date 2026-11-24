//! Glisser qui sort de la fenêtre (KUI-047) : des données exportées en fichiers passent
//! au glisser du système quand le pointeur quitte la fenêtre ; le backend dit ensuite si
//! elles ont été prises, et la source l'apprend.
use crate::{
    platform::headless::{HeadlessApp, HeadlessOptions, RawModifiers},
    ui::{
        drag::{DragEvent, DragPhase, Payload},
        prelude::*,
        runtime::{PointerButton, PointerPhase, UiEvent},
        scene::PointF,
    },
};
use std::path::PathBuf;

struct Library {
    exported: bool,
    ends: Vec<(bool, bool)>,
}

impl Application for Library {
    type Message = (bool, bool);
    fn view(&self, _: &ViewContext) -> Element {
        let mut payload = Payload::new("asset", "Brique", 42_u64);
        if self.exported {
            payload = payload.exported_as(vec![PathBuf::from("C:/projet/brique.png")]);
        }
        let thumb = column("thumb", [])
            .width(Length::Px(80.))
            .height(Length::Px(60.))
            .drag_payload(payload)
            .on_drag(|e: &DragEvent| {
                (e.phase == DragPhase::End).then_some((e.dropped, e.cancelled))
            });
        column("page", [thumb])
    }
    fn update(&mut self, end: (bool, bool)) -> bool {
        self.ends.push(end);
        true
    }
}

fn mouse(app: &mut HeadlessApp<Library>, phase: PointerPhase, p: PointF) {
    let event = match phase {
        PointerPhase::Down => UiEvent::PointerDown(p),
        PointerPhase::Up => UiEvent::PointerUp(p),
        _ => UiEvent::PointerMove(p),
    };
    app.event(event).unwrap();
    let button = (phase != PointerPhase::Move).then_some(PointerButton::Primary);
    app.raw_pointer(phase, button, p, RawModifiers::default())
        .unwrap();
}

fn library(exported: bool) -> HeadlessApp<Library> {
    HeadlessApp::with_options(
        Library {
            exported,
            ends: Vec::new(),
        },
        HeadlessOptions {
            width: 400.,
            height: 300.,
            ..Default::default()
        },
    )
    .unwrap()
}

#[test]
fn exported_files_leave_the_window_and_the_source_learns_they_were_taken() {
    let mut app = library(true);
    mouse(&mut app, PointerPhase::Down, PointF { x: 20., y: 20. });
    mouse(&mut app, PointerPhase::Move, PointF { x: 60., y: 40. });
    assert!(app.runtime_mut().take_outgoing_files().is_none());
    // Hors de la fenêtre, à gauche : les fichiers passent au système, une fois.
    mouse(&mut app, PointerPhase::Move, PointF { x: -5., y: 40. });
    let files = app.runtime_mut().take_outgoing_files().unwrap();
    assert_eq!(&*files, [PathBuf::from("C:/projet/brique.png")]);
    assert!(app.runtime_mut().take_outgoing_files().is_none());
    // Le système dit qu'ils ont été pris.
    app.runtime_mut().outgoing_drag_done(true);
    app.prepare().unwrap();
    assert_eq!(app.app().ends, [(true, false)]);
    // Un glisser suivant repart de zéro.
    mouse(&mut app, PointerPhase::Down, PointF { x: 20., y: 20. });
    mouse(&mut app, PointerPhase::Move, PointF { x: 30., y: 500. });
    assert!(app.runtime_mut().take_outgoing_files().is_some());
    app.runtime_mut().outgoing_drag_done(false);
    app.prepare().unwrap();
    assert_eq!(app.app().ends[1], (false, true));
}

#[test]
fn without_a_system_drag_the_release_outside_ends_it() {
    // Hors de Windows, aucun backend ne prend les fichiers : le relâchement finit le
    // glisser, sans dépôt.
    let mut app = library(true);
    mouse(&mut app, PointerPhase::Down, PointF { x: 20., y: 20. });
    mouse(&mut app, PointerPhase::Move, PointF { x: -5., y: 40. });
    mouse(&mut app, PointerPhase::Up, PointF { x: -5., y: 40. });
    assert_eq!(app.app().ends, [(false, false)]);
}

#[test]
fn a_payload_without_files_stays_in_the_window() {
    // Témoin : sans `exported_as`, le pointeur hors de la fenêtre ne change rien.
    let mut app = library(false);
    mouse(&mut app, PointerPhase::Down, PointF { x: 20., y: 20. });
    mouse(&mut app, PointerPhase::Move, PointF { x: -5., y: 40. });
    assert!(app.runtime_mut().take_outgoing_files().is_none());
    mouse(&mut app, PointerPhase::Up, PointF { x: -5., y: 40. });
    assert_eq!(app.app().ends, [(false, false)]);
}
