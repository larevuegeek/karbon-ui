//! État d'un éditeur de courbes : la vue (temps et valeurs à l'écran), les clés
//! choisies, le geste en cours (clés glissées, pente tirée, rectangle de choix, vue
//! glissée), et ce que les gestes et les touches demandent à l'application.
use super::model::Curve;
use crate::ui::{
    editor::{FieldChange, canvas::CanvasView},
    runtime::{KeyCode, KeyPress, PointerButton, PointerEvent, PointerPhase},
    scene::{PointF, RectF},
};

/// Distance du pointeur à une clé ou à une poignée, en pixels, pour la prendre ; longueur
/// d'une poignée de pente à l'écran ; pas des flèches, en pixels.
const HIT: f32 = 6.;
pub(super) const HANDLE: f32 = 40.;
const NUDGE: f32 = 10.;

/// A key of a curve editor: its curve and its rank in that curve.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CurveRef {
    /// The rank of its curve.
    pub curve: usize,
    /// Its rank in its curve.
    pub key: usize,
}

/// What the application does after a gesture of a [`CurveEditor`](super::CurveEditor).
#[derive(Clone, Debug, PartialEq)]
pub enum CurveEvent {
    /// Move the keys `keys` by `by` (time, value): `Preview` during the gesture, one
    /// `Commit` at its end (one step of history), `Cancel` with Escape (back to `by`
    /// zero).
    Move {
        /// The keys moved.
        keys: Vec<CurveRef>,
        /// How far, in time and in value, from where they were.
        by: (f32, f32),
        /// Preview, commit or cancel.
        change: FieldChange,
    },
    /// Give the key `key` this slope on one side (`out`: leaving it), or both when its
    /// slopes were equal (`Preview`, one `Commit`, `Cancel`).
    Slope {
        /// The key.
        key: CurveRef,
        /// The side tied: the slope leaving the key, else the one arriving.
        out: bool,
        /// The new slope.
        slope: f32,
        /// Preview, commit or cancel.
        change: FieldChange,
    },
    /// Add a key to the curve `curve` at `time` (a double click on or near it).
    Add {
        /// The curve.
        curve: usize,
        /// The time under the pointer.
        time: f32,
        /// The value of the curve there.
        value: f32,
    },
    /// Remove these keys (Delete).
    Delete(Vec<CurveRef>),
}

/// A gesture of a curve editor, to pass to [`CurveEditorState::update`].
#[derive(Clone, Debug, PartialEq)]
pub struct CurveMsg(pub(super) Inner);

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Inner {
    Pointer {
        size: (f32, f32),
        event: PointerEvent,
    },
}

/// What a gesture did: the event for the application, and whether to rebuild the view.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CurveReply {
    /// What the application has to do.
    pub event: Option<CurveEvent>,
    /// Rebuild the view.
    pub rebuild: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Drag {
    Keys { start: PointF, by: (f32, f32) },
    Tangent { key: CurveRef, out: bool },
    Pan { last: PointF },
    Boxed { from: PointF, to: PointF, add: bool },
}

/// State of a [`CurveEditor`](super::CurveEditor), kept by the application: the view, the
/// keys chosen, the gesture in progress.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CurveEditorState {
    /// What the editor shows.
    pub view: CanvasView,
    pub(super) selection: Vec<CurveRef>,
    pub(super) drag: Option<Drag>,
    pub(super) size: (f32, f32),
}

/// Ce qui est sous le pointeur.
enum Hit {
    Key(CurveRef),
    Tangent(CurveRef, bool),
    Curve(usize),
    Nothing,
}

impl CurveEditorState {
    /// The keys chosen.
    pub fn selection(&self) -> &[CurveRef] {
        &self.selection
    }

    /// Choose these keys (after the application changed the ranks, keys added or removed).
    pub fn set_selection(&mut self, keys: Vec<CurveRef>) {
        self.selection = keys;
    }

    /// Show every key of `curves` with a margin (framing), in a view of `size` pixels.
    pub fn frame_all(&mut self, curves: &[Curve], size: (f32, f32)) {
        let mut keys = curves.iter().flat_map(|c| c.keys.iter());
        let Some(first) = keys.next() else {
            return;
        };
        let mut world = RectF {
            left: first.time,
            right: first.time,
            bottom: first.value,
            top: first.value,
        };
        for key in keys {
            world.left = world.left.min(key.time);
            world.right = world.right.max(key.time);
            world.bottom = world.bottom.min(key.value);
            world.top = world.top.max(key.value);
        }
        self.view.frame(world, size, 24.);
    }

    /// Apply a gesture of the editor showing `curves`.
    pub fn update(&mut self, message: CurveMsg, curves: &[Curve]) -> CurveReply {
        let CurveMsg(Inner::Pointer { size, event }) = message;
        self.size = size;
        let at = PointF {
            x: event.local_x,
            y: event.local_y,
        };
        match event.phase {
            PointerPhase::Down => self.press(event, at, curves),
            PointerPhase::Move => self.moved(at, curves),
            PointerPhase::Up => self.released(curves),
            PointerPhase::Wheel => self.wheel(event, at),
        }
    }

    /// A key for the editor (forward the keys from `Application::key` while it has the
    /// focus): Delete removes the keys chosen, F frames every key, the arrows nudge the
    /// keys chosen by ten pixels, Escape cancels a drag.
    pub fn key(&mut self, press: KeyPress, curves: &[Curve]) -> CurveReply {
        if !press.pressed {
            return CurveReply::default();
        }
        let nudge = |x: f32, y: f32| (x * NUDGE, y * NUDGE);
        let moved = match press.key {
            KeyCode::Escape => return self.cancel(),
            KeyCode::Delete | KeyCode::Backspace if !self.selection.is_empty() => {
                let keys = std::mem::take(&mut self.selection);
                return reply(Some(CurveEvent::Delete(keys)));
            }
            KeyCode::F | KeyCode::Home => {
                self.frame_all(curves, self.size);
                return reply(None);
            }
            KeyCode::Left => nudge(-1., 0.),
            KeyCode::Right => nudge(1., 0.),
            KeyCode::Up => nudge(0., 1.),
            KeyCode::Down => nudge(0., -1.),
            _ => return CurveReply::default(),
        };
        if self.selection.is_empty() || self.drag.is_some() {
            return CurveReply::default();
        }
        let by = (moved.0 / self.view.zoom.0, moved.1 / self.view.zoom.1);
        let keys = self.selection.clone();
        reply(Some(CurveEvent::Move {
            keys,
            by,
            change: FieldChange::Commit,
        }))
    }

    fn hit(&self, at: PointF, curves: &[Curve]) -> Hit {
        let near = |p: PointF| (p.x - at.x).abs() <= HIT && (p.y - at.y).abs() <= HIT;
        for &key in &self.selection {
            let Some(k) = curves.get(key.curve).and_then(|c| c.keys.get(key.key)) else {
                continue;
            };
            for out in [true, false] {
                if near(handle(&self.view, k.time, k.value, slope(k, out), out)) {
                    return Hit::Tangent(key, out);
                }
            }
        }
        for (c, curve) in curves.iter().enumerate().rev() {
            for (k, key) in curve.keys.iter().enumerate() {
                let p = self.view.to_screen(PointF {
                    x: key.time,
                    y: key.value,
                });
                if near(p) {
                    return Hit::Key(CurveRef { curve: c, key: k });
                }
            }
        }
        let world = self.view.to_world(at);
        for (c, curve) in curves.iter().enumerate().rev() {
            let y = self.view.to_screen(PointF {
                x: world.x,
                y: curve.value_at(world.x),
            });
            if !curve.keys.is_empty() && (y.y - at.y).abs() <= HIT {
                return Hit::Curve(c);
            }
        }
        Hit::Nothing
    }

    fn press(&mut self, event: PointerEvent, at: PointF, curves: &[Curve]) -> CurveReply {
        match event.button {
            Some(PointerButton::Middle) => {
                self.drag = Some(Drag::Pan { last: at });
                return reply(None);
            }
            Some(PointerButton::Primary) => {}
            _ => return CurveReply::default(),
        }
        let hit = self.hit(at, curves);
        if event.clicks >= 2
            && let Hit::Curve(curve) | Hit::Key(CurveRef { curve, .. }) = hit
        {
            let time = self.view.to_world(at).x;
            let value = curves[curve].value_at(time);
            return reply(Some(CurveEvent::Add { curve, time, value }));
        }
        match hit {
            Hit::Tangent(key, out) => self.drag = Some(Drag::Tangent { key, out }),
            Hit::Key(key) => {
                if event.shift {
                    match self.selection.iter().position(|k| *k == key) {
                        Some(index) => drop(self.selection.remove(index)),
                        None => self.selection.push(key),
                    }
                } else if !self.selection.contains(&key) {
                    self.selection = vec![key];
                }
                let start = self.view.to_world(at);
                self.drag = Some(Drag::Keys {
                    start,
                    by: (0., 0.),
                });
            }
            Hit::Curve(_) | Hit::Nothing => {
                if !event.shift {
                    self.selection.clear();
                }
                self.drag = Some(Drag::Boxed {
                    from: at,
                    to: at,
                    add: event.shift,
                });
            }
        }
        reply(None)
    }

    fn moved(&mut self, at: PointF, curves: &[Curve]) -> CurveReply {
        let view = self.view;
        match &mut self.drag {
            Some(Drag::Pan { last }) => {
                let by = (at.x - last.x, at.y - last.y);
                *last = at;
                self.view.pan(by);
                reply(None)
            }
            Some(Drag::Boxed { to, .. }) => {
                *to = at;
                reply(None)
            }
            Some(Drag::Keys { start, by }) => {
                let world = view.to_world(at);
                *by = (world.x - start.x, world.y - start.y);
                let event = CurveEvent::Move {
                    keys: self.selection.clone(),
                    by: *by,
                    change: FieldChange::Preview,
                };
                reply(Some(event))
            }
            Some(Drag::Tangent { key, out }) => {
                let (key, out) = (*key, *out);
                let Some(k) = curves.get(key.curve).and_then(|c| c.keys.get(key.key)) else {
                    return CurveReply::default();
                };
                let origin = view.to_screen(PointF {
                    x: k.time,
                    y: k.value,
                });
                let slope = screen_slope(&view, origin, at, out);
                reply(Some(CurveEvent::Slope {
                    key,
                    out,
                    slope,
                    change: FieldChange::Preview,
                }))
            }
            None => CurveReply::default(),
        }
    }

    fn released(&mut self, curves: &[Curve]) -> CurveReply {
        match self.drag.take() {
            Some(Drag::Keys { by, .. }) if by != (0., 0.) => reply(Some(CurveEvent::Move {
                keys: self.selection.clone(),
                by,
                change: FieldChange::Commit,
            })),
            Some(Drag::Tangent { key, out }) => {
                let Some(k) = curves.get(key.curve).and_then(|c| c.keys.get(key.key)) else {
                    return reply(None);
                };
                let event = CurveEvent::Slope {
                    key,
                    out,
                    slope: slope(k, out),
                    change: FieldChange::Commit,
                };
                reply(Some(event))
            }
            Some(Drag::Boxed { from, to, add }) => {
                self.choose_in(from, to, add, curves);
                reply(None)
            }
            _ => reply(None),
        }
    }

    /// Choisit les clés dans le rectangle `from`–`to` (en plus du choix avec Maj).
    fn choose_in(&mut self, from: PointF, to: PointF, add: bool, curves: &[Curve]) {
        if !add {
            self.selection.clear();
        }
        let (left, right) = (from.x.min(to.x), from.x.max(to.x));
        let (top, bottom) = (from.y.min(to.y), from.y.max(to.y));
        for (c, curve) in curves.iter().enumerate() {
            for (k, key) in curve.keys.iter().enumerate() {
                let p = self.view.to_screen(PointF {
                    x: key.time,
                    y: key.value,
                });
                let inside = p.x >= left && p.x <= right && p.y >= top && p.y <= bottom;
                let key = CurveRef { curve: c, key: k };
                if inside && !self.selection.contains(&key) {
                    self.selection.push(key);
                }
            }
        }
    }

    fn wheel(&mut self, event: PointerEvent, at: PointF) -> CurveReply {
        let factor = (-event.wheel.1 * 0.15).exp();
        let factor = match (event.control, event.shift) {
            (true, false) => (factor, 1.),
            (false, true) => (1., factor),
            _ => (factor, factor),
        };
        self.view.zoom_at(at, factor);
        reply(None)
    }

    fn cancel(&mut self) -> CurveReply {
        match self.drag.take() {
            Some(Drag::Keys { .. }) => reply(Some(CurveEvent::Move {
                keys: self.selection.clone(),
                by: (0., 0.),
                change: FieldChange::Cancel,
            })),
            Some(Drag::Tangent { key, out }) => reply(Some(CurveEvent::Slope {
                key,
                out,
                slope: 0.,
                change: FieldChange::Cancel,
            })),
            Some(_) => reply(None),
            None => CurveReply::default(),
        }
    }
}

fn reply(event: Option<CurveEvent>) -> CurveReply {
    CurveReply {
        event,
        rebuild: true,
    }
}

/// La pente d'un côté de la clé.
pub(super) fn slope(key: &super::CurveKey, out: bool) -> f32 {
    if out { key.out_slope } else { key.in_slope }
}

/// Le bout de la poignée de pente `slope` d'une clé en (`time`, `value`), à l'écran : à
/// `HANDLE` pixels, à droite pour la pente qui part, à gauche pour celle qui arrive.
pub(super) fn handle(view: &CanvasView, time: f32, value: f32, slope: f32, out: bool) -> PointF {
    let origin = view.to_screen(PointF { x: time, y: value });
    // La direction de la pente à l'écran : une unité de temps, `slope` unités de valeur.
    let (dx, dy) = (view.zoom.0, -slope * view.zoom.1);
    let length = dx.hypot(dy).max(1e-6);
    let sign = if out { 1. } else { -1. };
    PointF {
        x: origin.x + sign * dx / length * HANDLE,
        y: origin.y + sign * dy / length * HANDLE,
    }
}

/// La pente que donne une poignée tirée de `origin` jusqu'à `at` (à l'écran).
fn screen_slope(view: &CanvasView, origin: PointF, at: PointF, out: bool) -> f32 {
    let sign = if out { 1. } else { -1. };
    let dx = (sign * (at.x - origin.x)).max(1.);
    let dy = -sign * (at.y - origin.y);
    (dy / view.zoom.1) / (dx / view.zoom.0)
}
