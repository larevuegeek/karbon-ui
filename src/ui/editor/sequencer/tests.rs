//! Frise de temps : la tête de lecture, le choix et le glisser des clés (un seul
//! `Commit` par geste, Échap), le rectangle de choix, l'ajout par double clic, le zoom
//! autour du pointeur, et la peinture des seules clés visibles.
use super::*;
use crate::ui::{
    editor::{
        FieldChange,
        testing::{NONE, SHIFT, at, pointer, window},
    },
    element::{Element, ElementId},
    prelude::{Application, ViewContext, column},
    runtime::{KeyCode, PointerButton, PointerPhase, raw_input::PointerInput},
    scene::{DrawCommand, PointF},
};

const LEFT: Option<PointerButton> = Some(PointerButton::Primary);

/// L'élément de la frise de clé 7.
fn line_id() -> ElementId {
    ElementId::named("karbon-sequencer").item(7)
}

/// Une frise de deux pistes de 2 s (ou de `many` pistes), et ce qu'elle a demandé.
struct Line {
    tracks: Vec<SequencerTrack>,
    time: f32,
    state: SequencerState,
    events: Vec<SequencerEvent>,
}

impl Line {
    fn new(tracks: Vec<SequencerTrack>) -> Self {
        let mut state = SequencerState::default();
        // 100 pixels par seconde, zéro au bord gauche des pistes.
        state.set_view(0., 100.);
        Self {
            tracks,
            time: 0.,
            state,
            events: Vec::new(),
        }
    }
}

fn two_tracks() -> Vec<SequencerTrack> {
    let track = |name: &str, keys: Vec<f32>| SequencerTrack {
        name: name.into(),
        keys,
        ..SequencerTrack::default()
    };
    vec![
        track("Place", vec![0., 1., 2.]),
        track("Rotation", vec![0.5, 1.5]),
    ]
}

impl Application for Line {
    type Message = SequencerMsg;
    fn view(&self, _: &ViewContext) -> Element {
        let line = Sequencer::new(7, self.tracks.clone(), self.time, &self.state, |m| m)
            .length(2.)
            .snap(0.1)
            .element();
        column("page", [line])
    }
    fn update(&mut self, message: SequencerMsg) -> bool {
        let reply = self.state.update(message);
        if let Some(event) = reply.event {
            if let SequencerEvent::Seek(time) = event {
                self.time = time;
            }
            self.events.push(event);
        }
        reply.rebuild
    }
}

/// Le point de l'instant `time` sur la piste `track` (la colonne des noms fait 150, la
/// règle 24, une piste 22).
fn spot(window: &crate::platform::headless::HeadlessApp<Line>, time: f32, track: f32) -> PointF {
    at(
        window,
        line_id(),
        150. + time * 100.,
        24. + 22. * (track + 0.5),
    )
}

/// Un geste du pointeur à l'instant et sur la piste `(time, track)`.
fn gesture(
    w: &mut crate::platform::headless::HeadlessApp<Line>,
    phase: PointerPhase,
    button: Option<PointerButton>,
    (time, track): (f32, f32),
    mods: crate::ui::editor::testing::Mods,
) {
    let point = spot(w, time, track);
    pointer(w, phase, button, point, mods);
}

#[test]
fn the_ruler_moves_the_play_head_snapped_to_its_step() {
    let mut w = window(Line::new(two_tracks()), 900., 300.);
    gesture(&mut w, PointerPhase::Down, LEFT, (1.23, -0.7), NONE);
    gesture(&mut w, PointerPhase::Move, None, (1.57, -0.7), NONE);
    gesture(&mut w, PointerPhase::Up, LEFT, (1.57, -0.7), NONE);
    let seeks: Vec<f32> = (w.app().events.iter())
        .filter_map(|e| match e {
            SequencerEvent::Seek(t) => Some(*t),
            _ => None,
        })
        .collect();
    assert_eq!(seeks.len(), 2, "{seeks:?}");
    assert!((seeks[0] - 1.2).abs() < 1e-4 && (seeks[1] - 1.6).abs() < 1e-4);
    assert!((w.app().time - 1.6).abs() < 1e-4);
}

#[test]
fn keys_are_chosen_and_moved_with_one_commit_and_escape_puts_them_back() {
    let mut w = window(Line::new(two_tracks()), 900., 300.);
    // La clé à 1 s de la première piste, glissée de 0,3 s.
    gesture(&mut w, PointerPhase::Down, LEFT, (1.0, 0.), NONE);
    assert_eq!(w.app().state.selection(), [KeyRef { track: 0, key: 1 }]);
    gesture(&mut w, PointerPhase::Move, None, (1.12, 0.), NONE);
    gesture(&mut w, PointerPhase::Move, None, (1.31, 0.), NONE);
    gesture(&mut w, PointerPhase::Up, LEFT, (1.31, 0.), NONE);
    let moves: Vec<(f32, FieldChange)> = (w.app().events.iter())
        .filter_map(|e| match e {
            SequencerEvent::Move { by, change, .. } => Some((*by, *change)),
            _ => None,
        })
        .collect();
    let commits = moves
        .iter()
        .filter(|(_, c)| *c == FieldChange::Commit)
        .count();
    assert_eq!(commits, 1);
    let last = moves.last().unwrap();
    assert!(
        (last.0 - 0.3).abs() < 1e-4 && last.1 == FieldChange::Commit,
        "{moves:?}"
    );
    // Échap pendant un glisser : les clés reviennent, sans `Commit`.
    w.update_app(|app| app.events.clear()).unwrap();
    gesture(&mut w, PointerPhase::Down, LEFT, (2.0, 0.), NONE);
    gesture(&mut w, PointerPhase::Move, None, (2.5, 0.), NONE);
    w.update_app(|app| {
        let reply = app
            .state
            .key(crate::ui::runtime::KeyPress::new(KeyCode::Escape, true));
        app.events.extend(reply.event);
    })
    .unwrap();
    let last = w.app().events.last().cloned();
    assert!(matches!(
        last,
        Some(SequencerEvent::Move { by, change: FieldChange::Cancel, .. }) if by == 0.
    ));
}

#[test]
fn a_rectangle_chooses_the_keys_inside_it_and_shift_adds() {
    let mut w = window(Line::new(two_tracks()), 900., 300.);
    // De 0,8 s sur la première piste à 1,7 s sur la seconde : deux clés.
    gesture(&mut w, PointerPhase::Down, LEFT, (0.8, -0.3), NONE);
    gesture(&mut w, PointerPhase::Move, None, (1.7, 1.3), NONE);
    gesture(&mut w, PointerPhase::Up, LEFT, (1.7, 1.3), NONE);
    let both = [KeyRef { track: 0, key: 1 }, KeyRef { track: 1, key: 1 }];
    assert_eq!(w.app().state.selection(), both);
    // Maj : un second rectangle ajoute la clé à 2 s.
    gesture(&mut w, PointerPhase::Down, LEFT, (1.9, -0.3), SHIFT);
    gesture(&mut w, PointerPhase::Move, None, (2.1, 0.3), SHIFT);
    gesture(&mut w, PointerPhase::Up, LEFT, (2.1, 0.3), SHIFT);
    assert_eq!(w.app().state.selection().len(), 3);
    // Un clic dans le vide : la tête de lecture y va, le choix se vide.
    gesture(&mut w, PointerPhase::Down, LEFT, (0.7, 1.), NONE);
    gesture(&mut w, PointerPhase::Up, LEFT, (0.7, 1.), NONE);
    assert!(w.app().state.selection().is_empty());
    assert!((w.app().time - 0.7).abs() < 1e-4);
}

#[test]
fn a_double_click_adds_a_key_and_a_right_click_asks_for_a_menu() {
    let mut w = window(Line::new(two_tracks()), 900., 300.);
    for _ in 0..2 {
        gesture(&mut w, PointerPhase::Down, LEFT, (0.81, 1.), NONE);
        gesture(&mut w, PointerPhase::Up, LEFT, (0.81, 1.), NONE);
    }
    let added = w.app().events.iter().find_map(|e| match e {
        SequencerEvent::Add { track, time } => Some((*track, *time)),
        _ => None,
    });
    let (track, time) = added.expect("une clé demandée");
    assert_eq!(track, 1);
    assert!((time - 0.8).abs() < 1e-4);
    gesture(
        &mut w,
        PointerPhase::Down,
        Some(PointerButton::Secondary),
        (1.5, 0.),
        NONE,
    );
    assert!(matches!(
        w.app().events.last(),
        Some(SequencerEvent::Menu { track: Some(0), .. })
    ));
}

#[test]
fn ctrl_and_the_wheel_zoom_around_the_pointer() {
    let mut w = window(Line::new(two_tracks()), 900., 300.);
    let under = spot(&w, 1.5, 0.);
    w.prepare().unwrap();
    let input = PointerInput {
        phase: PointerPhase::Wheel,
        button: None,
        position: under,
        shift: false,
        control: true,
        alt: false,
        meta: false,
        motion: None,
    };
    w.runtime_mut().pointer_wheel(input, (0., -2.));
    w.prepare().unwrap();
    let state = &w.app().state;
    assert!(state.zoom() > 140.);
    // L'instant sous le pointeur n'a pas bougé.
    let time = state.start() + (under.x - at(&w, line_id(), 0., 0.).x - 150.) / state.zoom();
    assert!((time - 1.5).abs() < 1e-3, "{time}");
}

#[test]
fn only_the_keys_in_view_are_painted() {
    // 200 pistes de 250 clés : 50 000 clés, dont une petite part se voit.
    let tracks = (0..200)
        .map(|t| SequencerTrack {
            name: format!("piste {t}"),
            keys: (0..250).map(|k| k as f32 * 0.1).collect(),
            ..SequencerTrack::default()
        })
        .collect();
    let mut w = window(Line::new(tracks), 900., 300.);
    w.prepare().unwrap();
    let diamonds = (w.scene().commands.iter())
        .filter(
            |c| matches!(c, DrawCommand::Path { closed: true, points, .. } if points.len() == 4),
        )
        .count();
    // 750 pixels de pistes à 100 par seconde : 7,5 s, 77 clés par piste ; 14 pistes
    // (la dernière à moitié).
    assert!(diamonds > 0 && diamonds <= 80 * 14, "{diamonds}");
}

#[test]
fn ruler_times_are_written_with_the_decimals_the_step_needs() {
    assert_eq!(view::seconds(1.5, 0.5, true), "1,5 s");
    assert_eq!(view::seconds(2.0, 1.0, false), "2 s");
    assert_eq!(view::seconds(-0.0, 0.1, false), "0.0 s");
    assert_eq!(view::seconds(0.25, 0.05, false), "0.25 s");
}

/// Le temps d'une image de la frise de 200 pistes et 50 000 clés, mise en page et
/// peinture comprises, puis zoomée et défilée. À la demande :
/// `cargo test --release --lib sequencer_cost -- --ignored --nocapture`.
#[test]
#[ignore = "mesure"]
fn sequencer_cost() {
    let tracks = (0..200)
        .map(|t| SequencerTrack {
            name: format!("piste {t}"),
            keys: (0..250).map(|k| k as f32 * 0.1).collect(),
            ..SequencerTrack::default()
        })
        .collect();
    let mut w = window(Line::new(tracks), 1600., 900.);
    w.prepare().unwrap();
    let began = std::time::Instant::now();
    for frame in 0..100 {
        w.update_app(|app| {
            app.time = frame as f32 * 0.05;
            app.state.set_view(frame as f32 * 0.1, 40. + frame as f32);
        })
        .unwrap();
    }
    let each = began.elapsed().as_secs_f64() * 1000.0 / 100.0;
    println!(
        "frise de 200 pistes, 50 000 clés : {each:.3} ms par image (mise en page et peinture)"
    );
}
