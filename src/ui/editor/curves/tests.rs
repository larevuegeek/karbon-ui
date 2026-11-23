//! Éditeur de courbes : valeurs entre les clés, clés glissées (un seul `Commit`, Échap),
//! pente tirée, clé ajoutée par un double clic, choix au rectangle et Maj, Suppr et
//! flèches au clavier, zoom autour du pointeur, cadrage, seules les clés visibles peintes.
use super::*;
use crate::{
    platform::headless::{HeadlessApp, RawModifiers},
    ui::{
        editor::{FieldChange, testing::window},
        element::{Element, ElementId},
        prelude::{Application, ViewContext},
        runtime::{KeyCode, KeyPress, PointerButton, PointerPhase},
        scene::PointF,
    },
};

struct Editor {
    curves: Vec<Curve>,
    state: CurveEditorState,
    log: Vec<CurveEvent>,
    /// Les clés au début d'un geste, pour l'appliquer « depuis le départ ».
    before: Option<Vec<Curve>>,
}

fn two_curves() -> Vec<Curve> {
    let key = |t: f32, v: f32| CurveKey {
        mode: KeyMode::Linear,
        ..CurveKey::new(t, v)
    };
    vec![
        Curve {
            name: "Rampe".into(),
            keys: vec![key(0., 0.), key(1., 1.), key(2., 0.)],
            ..Default::default()
        },
        Curve {
            name: "Plat".into(),
            keys: vec![key(0., -0.5), key(2., -0.5)],
            ..Default::default()
        },
    ]
}

impl Editor {
    fn new() -> Self {
        let mut state = CurveEditorState::default();
        state.frame_all(&two_curves(), (400., 300.));
        Self {
            curves: two_curves(),
            state,
            log: Vec::new(),
            before: None,
        }
    }

    fn apply(&mut self, event: &CurveEvent) {
        match event {
            CurveEvent::Move { keys, by, change } => {
                let before = self
                    .before
                    .get_or_insert_with(|| self.curves.clone())
                    .clone();
                for key in keys {
                    let start = before[key.curve].keys[key.key];
                    let moved = &mut self.curves[key.curve].keys[key.key];
                    moved.time = start.time + by.0;
                    moved.value = start.value + by.1;
                }
                if *change != FieldChange::Preview {
                    self.before = None;
                }
            }
            CurveEvent::Slope {
                key,
                out,
                slope,
                change,
            } if *change != FieldChange::Cancel => {
                let k = &mut self.curves[key.curve].keys[key.key];
                k.mode = KeyMode::Smooth;
                if *out {
                    k.out_slope = *slope
                } else {
                    k.in_slope = *slope
                }
            }
            CurveEvent::Add { curve, time, value } => {
                let keys = &mut self.curves[*curve].keys;
                let at = keys.partition_point(|k| k.time < *time);
                keys.insert(at, CurveKey::new(*time, *value));
            }
            CurveEvent::Delete(keys) => {
                let mut keys = keys.clone();
                keys.sort();
                for key in keys.into_iter().rev() {
                    self.curves[key.curve].keys.remove(key.key);
                }
            }
            _ => {}
        }
    }
}

impl Application for Editor {
    type Message = CurveMsg;
    fn view(&self, _: &ViewContext) -> Element {
        CurveEditor::new(1, &self.curves, &self.state, |m| m).element()
    }
    fn update(&mut self, message: CurveMsg) -> bool {
        let reply = self.state.update(message, &self.curves);
        if let Some(event) = reply.event {
            self.apply(&event);
            self.log.push(event);
        }
        reply.rebuild
    }
    fn key(&mut self, press: KeyPress) -> bool {
        let reply = self.state.key(press, &self.curves);
        if let Some(event) = reply.event {
            self.apply(&event);
            self.log.push(event);
        }
        reply.rebuild
    }
}

fn editor() -> HeadlessApp<Editor> {
    window(Editor::new(), 400., 300.)
}

/// La place à l'écran (dans la fenêtre) du point (`t`, `v`).
fn at(w: &HeadlessApp<Editor>, t: f32, v: f32) -> PointF {
    let area = w.bounds(ElementId::named("karbon-curves").item(1)).unwrap();
    let p = w.app().state.view.to_screen(PointF { x: t, y: v });
    PointF {
        x: area.left + p.x,
        y: area.top + p.y,
    }
}

fn press(w: &mut HeadlessApp<Editor>, phase: PointerPhase, place: PointF, shift: bool) {
    let left = (phase != PointerPhase::Move).then_some(PointerButton::Primary);
    let modifiers = RawModifiers {
        shift,
        ..Default::default()
    };
    w.raw_pointer(phase, left, place, modifiers).unwrap();
}

#[test]
fn values_between_keys_follow_their_modes() {
    let mut curve = Curve {
        keys: vec![CurveKey::new(0., 0.), CurveKey::new(1., 1.)],
        ..Curve::default()
    };
    // Lisse et à plat aux deux bouts : 0,5 au milieu, symétrique.
    assert!((curve.value_at(0.5) - 0.5).abs() < 1e-6);
    assert!((curve.value_at(0.25) + curve.value_at(0.75) - 1.).abs() < 1e-6);
    curve.keys[0].mode = KeyMode::Constant;
    assert_eq!(curve.value_at(0.9), 0.);
    curve.keys[0].mode = KeyMode::Linear;
    assert_eq!(curve.value_at(0.25), 0.25);
    assert_eq!(curve.value_at(5.), 1.);
    assert_eq!(Curve::default().value_at(1.), 0.);
}

#[test]
fn a_key_dragged_moves_with_one_commit_and_escape_puts_it_back() {
    let mut w = editor();
    let from = at(&w, 1., 1.);
    press(&mut w, PointerPhase::Down, from, false);
    let to = at(&w, 1.5, 0.5);
    press(&mut w, PointerPhase::Move, to, false);
    press(&mut w, PointerPhase::Up, to, false);
    let key = w.app().curves[0].keys[1];
    assert!(
        (key.time - 1.5).abs() < 1e-3 && (key.value - 0.5).abs() < 1e-3,
        "{key:?}"
    );
    let commits = w
        .app()
        .log
        .iter()
        .filter(|e| {
            matches!(
                e,
                CurveEvent::Move {
                    change: FieldChange::Commit,
                    ..
                }
            )
        })
        .count();
    assert_eq!(commits, 1);
    // Un autre glisser, abandonné par Échap : la clé revient.
    let from = at(&w, 1.5, 0.5);
    press(&mut w, PointerPhase::Down, from, false);
    let p = at(&w, 1.8, 0.2);
    press(&mut w, PointerPhase::Move, p, false);
    w.raw_key(KeyPress::new(KeyCode::Escape, true)).unwrap();
    let key = w.app().curves[0].keys[1];
    assert!(
        (key.time - 1.5).abs() < 1e-3 && (key.value - 0.5).abs() < 1e-3,
        "{key:?}"
    );
}

#[test]
fn a_double_click_on_a_curve_adds_a_key_where_the_curve_is() {
    let mut w = editor();
    let on = at(&w, 0.5, 0.5);
    press(&mut w, PointerPhase::Down, on, false);
    press(&mut w, PointerPhase::Up, on, false);
    press(&mut w, PointerPhase::Down, on, false);
    let added = w.app().log.iter().find_map(|e| match e {
        CurveEvent::Add { curve, time, value } => Some((*curve, *time, *value)),
        _ => None,
    });
    let (curve, time, value) = added.expect("une clé ajoutée");
    assert_eq!(curve, 0);
    assert!((time - 0.5).abs() < 0.02 && (value - time).abs() < 1e-4);
    assert_eq!(w.app().curves[0].keys.len(), 4);
    // Témoin : un double clic loin de toute courbe n'ajoute rien.
    let mut w = editor();
    let empty = at(&w, 1., -0.1);
    press(&mut w, PointerPhase::Down, empty, false);
    press(&mut w, PointerPhase::Up, empty, false);
    press(&mut w, PointerPhase::Down, empty, false);
    assert!(
        !w.app()
            .log
            .iter()
            .any(|e| matches!(e, CurveEvent::Add { .. }))
    );
}

#[test]
fn a_rectangle_and_shift_choose_keys_and_delete_removes_them() {
    let mut w = editor();
    // Un rectangle autour des clés à t = 0 (une par courbe).
    let p = at(&w, -0.1, 1.1);
    press(&mut w, PointerPhase::Down, p, false);
    let p = at(&w, 0.1, -0.7);
    press(&mut w, PointerPhase::Move, p, false);
    let p = at(&w, 0.1, -0.7);
    press(&mut w, PointerPhase::Up, p, false);
    assert_eq!(w.app().state.selection().len(), 2);
    // Maj et un clic sur une clé : ajoutée au choix.
    let top = at(&w, 1., 1.);
    press(&mut w, PointerPhase::Down, top, true);
    press(&mut w, PointerPhase::Up, top, true);
    assert_eq!(w.app().state.selection().len(), 3);
    // Les flèches poussent les clés choisies d'un seul pas d'historique chacune.
    w.raw_key(KeyPress::new(KeyCode::Right, true)).unwrap();
    assert!(w.app().curves[0].keys[1].time > 1.);
    w.raw_key(KeyPress::new(KeyCode::Delete, true)).unwrap();
    assert_eq!(w.app().curves[0].keys.len(), 1);
    assert_eq!(w.app().curves[1].keys.len(), 1);
    assert!(w.app().state.selection().is_empty());
}

#[test]
fn a_slope_handle_dragged_tilts_the_curve() {
    let mut w = editor();
    let key = at(&w, 1., 1.);
    press(&mut w, PointerPhase::Down, key, false);
    press(&mut w, PointerPhase::Up, key, false);
    // La poignée de sortie est à 40 pixels à droite (la clé est à plat) ; tirée vers le
    // haut, la pente devient positive.
    let handle = PointF {
        x: key.x + 40.,
        y: key.y,
    };
    press(&mut w, PointerPhase::Down, handle, false);
    let up = PointF {
        x: key.x + 40.,
        y: key.y - 40.,
    };
    press(&mut w, PointerPhase::Move, up, false);
    press(&mut w, PointerPhase::Up, up, false);
    let k = w.app().curves[0].keys[1];
    assert!(k.out_slope > 0., "{k:?}");
    assert!(w.app().log.iter().any(|e| matches!(
        e,
        CurveEvent::Slope {
            change: FieldChange::Commit,
            ..
        }
    )));
}

#[test]
fn the_wheel_zooms_around_the_pointer_and_f_frames_everything() {
    let mut w = editor();
    let point = at(&w, 1., 1.);
    let before = w.app().state.view.zoom;
    w.prepare().unwrap();
    let event = crate::ui::runtime::raw_input::PointerInput {
        phase: PointerPhase::Wheel,
        button: None,
        position: point,
        shift: false,
        control: true,
        alt: false,
        meta: false,
        motion: None,
    };
    w.runtime_mut().pointer_wheel(event, (0., -3.));
    w.prepare().unwrap();
    let after = w.app().state.view.zoom;
    // Ctrl : le temps seul.
    assert!(after.0 > before.0 && (after.1 - before.1).abs() < 1e-6);
    let back = at(&w, 1., 1.);
    assert!((back.x - point.x).abs() < 0.01 && (back.y - point.y).abs() < 0.01);
    w.raw_key(KeyPress::new(KeyCode::F, true)).unwrap();
    assert!((w.app().state.view.zoom.0 - before.0).abs() < 1e-3);
}
