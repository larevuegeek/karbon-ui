//! État d'une frise : ce qu'elle montre (le temps au bord gauche, les pixels par
//! seconde, le défilement des pistes), les clés choisies, et le geste en cours (tête de
//! lecture tirée, clés glissées, rectangle de choix, glisser de la vue).
use crate::ui::{
    editor::FieldChange,
    runtime::{KeyCode, KeyPress, PointerButton, PointerEvent, PointerPhase},
};
use std::sync::Arc;

/// Pixels par seconde au départ, et bornes du zoom.
const START_ZOOM: f32 = 100.;
const ZOOM_RANGE: (f32, f32) = (2., 20_000.);
/// Distance du pointeur à une clé, en pixels, pour la prendre.
const HIT: f32 = 6.;

/// A key of a [`Sequencer`](super::Sequencer): its track and its rank in that track, as
/// the application gave them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct KeyRef {
    /// The rank of its track.
    pub track: usize,
    /// Its rank in its track.
    pub key: usize,
}

/// La place des parties d'une frise peinte, et ce qu'elle montrait : ce que porte un
/// geste du pointeur.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Frame {
    pub(super) key: u64,
    pub(super) size: (f32, f32),
    /// Largeur de la colonne des noms, hauteur de la règle et d'une piste.
    pub(super) names: f32,
    pub(super) ruler: f32,
    pub(super) row: f32,
    pub(super) start: f32,
    pub(super) zoom: f32,
    pub(super) scroll: f32,
    pub(super) snap: Option<f32>,
}

impl Frame {
    /// L'instant sous l'abscisse `x` de l'élément.
    pub(super) fn time_at(&self, x: f32) -> f32 {
        self.start + (x - self.names) / self.zoom
    }

    /// L'abscisse de l'instant `time`.
    pub(super) fn x_of(&self, time: f32) -> f32 {
        self.names + (time - self.start) * self.zoom
    }

    /// La piste sous l'ordonnée `y`, si c'est une piste.
    fn track_at(&self, y: f32, tracks: usize) -> Option<usize> {
        if y < self.ruler {
            return None;
        }
        let row = ((y - self.ruler + self.scroll) / self.row).floor();
        (row >= 0. && (row as usize) < tracks).then_some(row as usize)
    }

    /// Un instant aimanté au pas de la frise (sans pas, ou Alt tenu : tel quel).
    fn snapped(&self, time: f32, free: bool) -> f32 {
        match self.snap.filter(|step| *step > 0. && !free) {
            Some(step) => (time / step).round() * step,
            None => time,
        }
    }
}

/// A gesture on a sequencer, to pass to [`SequencerState::update`]. The sequencers build
/// them; an application only forwards them.
#[derive(Clone, Debug, PartialEq)]
pub struct SequencerMsg(pub(super) Inner);

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Inner {
    Pointer {
        frame: Frame,
        /// Les instants des clés de chaque piste.
        keys: Arc<Vec<Vec<f32>>>,
        event: PointerEvent,
    },
    Wheel {
        frame: Frame,
        event: PointerEvent,
    },
}

/// What a gesture on a sequencer asks of the application.
#[derive(Clone, Debug, PartialEq)]
pub enum SequencerEvent {
    /// Put the play head at this time (a click or a drag on the ruler or in the empty
    /// part of a track).
    Seek(f32),
    /// The chosen keys changed ([`SequencerState::selection`]).
    Select(Vec<KeyRef>),
    /// Move these keys by `by` seconds: previews during the drag, one commit at its end,
    /// cancel on Escape (`by` is then 0: put them back).
    Move {
        /// The keys moved: the chosen ones.
        keys: Vec<KeyRef>,
        /// Seconds from where they were when the drag started.
        by: f32,
        /// Preview, commit or cancel.
        change: FieldChange,
    },
    /// A double click in a track: add a key there.
    Add {
        /// The rank of the track.
        track: usize,
        /// The time, snapped.
        time: f32,
    },
    /// A right click: open a menu at `at` (window coordinates), over the track `track`
    /// (none: on the ruler or below the tracks) at the time `time`.
    Menu {
        /// The rank of the track under the pointer.
        track: Option<usize>,
        /// The time under the pointer, snapped.
        time: f32,
        /// Where the pointer is, in window coordinates.
        at: (f32, f32),
    },
}

/// Answer of [`SequencerState::update`] and [`SequencerState::key`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SequencerReply {
    /// The sequencer changed on screen: return `true` from `Application::update`.
    pub rebuild: bool,
    /// What to do, if anything.
    pub event: Option<SequencerEvent>,
}

impl SequencerReply {
    fn redraw() -> Self {
        Self {
            rebuild: true,
            event: None,
        }
    }

    fn with(event: SequencerEvent) -> Self {
        Self {
            rebuild: true,
            event: Some(event),
        }
    }
}

/// Le geste en cours.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Drag {
    /// La tête de lecture tirée.
    Scrub,
    /// Les clés choisies glissées : l'instant pris, de combien elles vont.
    Keys { from: f32, by: f32 },
    /// Un rectangle de choix, d'un coin à l'autre (coordonnées de l'élément), en ajout
    /// au choix d'avant (Maj).
    Boxed {
        from: (f32, f32),
        to: (f32, f32),
        adding: bool,
    },
    /// La vue glissée au bouton du milieu : où le pointeur et la vue étaient.
    Pan {
        at: (f32, f32),
        start: f32,
        scroll: f32,
    },
}

/// State of the sequencers of a window, kept by the application: what each one shows
/// (the time at its left edge, its zoom, how far its tracks are scrolled), the chosen
/// keys and the gesture in progress.
///
/// Gestures: on the ruler, press and drag to move the play head; on a key, press to
/// choose it (Shift: add it to the choice, or take it out) and drag to move the chosen
/// keys, snapped to the step of the sequencer unless Alt is held; elsewhere in the
/// tracks, a click moves the play head and clears the choice, a drag chooses the keys
/// in a rectangle (Shift: adds them); a double click adds a key; the right button asks
/// for a menu; the middle button drags the view. The wheel scrolls the tracks, Shift
/// and the wheel scroll the time, Ctrl and the wheel zoom around the pointer. Escape
/// (forwarded to [`SequencerState::key`]) cancels a drag of keys.
///
/// ```
/// use karbon_ui::ui::editor::SequencerState;
///
/// let mut state = SequencerState::default();
/// state.set_view(0.0, 50.0);
/// assert_eq!(state.zoom(), 50.0);
/// assert!(state.selection().is_empty());
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct SequencerState {
    pub(super) start: f32,
    pub(super) zoom: f32,
    pub(super) scroll: f32,
    pub(super) selection: Vec<KeyRef>,
    pub(super) drag: Option<Drag>,
}

impl Default for SequencerState {
    fn default() -> Self {
        Self {
            start: 0.,
            zoom: START_ZOOM,
            scroll: 0.,
            selection: Vec::new(),
            drag: None,
        }
    }
}

impl SequencerState {
    /// The chosen keys, in the order of their tracks and ranks.
    pub fn selection(&self) -> &[KeyRef] {
        &self.selection
    }

    /// Choose these keys (after the application moved or removed keys, their ranks
    /// change).
    pub fn set_selection(&mut self, mut keys: Vec<KeyRef>) {
        keys.sort();
        keys.dedup();
        self.selection = keys;
    }

    /// Pixels per second.
    pub fn zoom(&self) -> f32 {
        self.zoom
    }

    /// The time at the left edge of the tracks.
    pub fn start(&self) -> f32 {
        self.start
    }

    /// Show the time `start` at the left edge of the tracks, `pixels_per_second` wide.
    pub fn set_view(&mut self, start: f32, pixels_per_second: f32) {
        self.start = start;
        self.zoom = pixels_per_second.clamp(ZOOM_RANGE.0, ZOOM_RANGE.1);
    }

    /// Show `seconds` of time from zero across `width` pixels of tracks, with a little
    /// room on both sides.
    pub fn frame(&mut self, seconds: f32, width: f32) {
        let seconds = seconds.max(0.01);
        self.set_view(-seconds * 0.04, width.max(1.) / (seconds * 1.08));
    }

    /// `true` while keys are dragged or the play head is.
    pub fn is_active(&self) -> bool {
        self.drag.is_some()
    }

    /// Apply a gesture of a sequencer ([`SequencerMsg`]).
    pub fn update(&mut self, message: SequencerMsg) -> SequencerReply {
        match message.0 {
            Inner::Pointer { frame, keys, event } => match event.phase {
                PointerPhase::Down => self.press(&frame, &keys, event),
                PointerPhase::Move => self.drag_to(&frame, event),
                PointerPhase::Up => self.release(&frame, &keys, event),
                PointerPhase::Wheel => SequencerReply::default(),
            },
            Inner::Wheel { frame, event } => self.wheel(&frame, event),
        }
    }

    /// Apply a key given to [`Application::key`](crate::application::Application::key):
    /// Escape during a drag of keys puts them back. Other keys change nothing.
    pub fn key(&mut self, press: KeyPress) -> SequencerReply {
        if !press.pressed || press.key != KeyCode::Escape {
            return SequencerReply::default();
        }
        match self.drag.take() {
            Some(Drag::Keys { .. }) => SequencerReply::with(SequencerEvent::Move {
                keys: self.selection.clone(),
                by: 0.,
                change: FieldChange::Cancel,
            }),
            Some(_) => SequencerReply::redraw(),
            None => SequencerReply::default(),
        }
    }

    fn press(&mut self, frame: &Frame, keys: &[Vec<f32>], event: PointerEvent) -> SequencerReply {
        let (x, y) = (event.local_x, event.local_y);
        let time = frame.time_at(x);
        match event.button {
            Some(PointerButton::Middle) => {
                self.drag = Some(Drag::Pan {
                    at: (x, y),
                    start: self.start,
                    scroll: self.scroll,
                });
                return SequencerReply::default();
            }
            Some(PointerButton::Secondary) => {
                return SequencerReply::with(SequencerEvent::Menu {
                    track: frame.track_at(y, keys.len()),
                    time: frame.snapped(time, event.alt),
                    at: (event.x, event.y),
                });
            }
            Some(PointerButton::Primary) => {}
            _ => return SequencerReply::default(),
        }
        if x < frame.names {
            return SequencerReply::default();
        }
        if y < frame.ruler {
            self.drag = Some(Drag::Scrub);
            return SequencerReply::with(SequencerEvent::Seek(frame.snapped(time, event.alt)));
        }
        let track = frame.track_at(y, keys.len());
        if event.clicks >= 2
            && let Some(track) = track
        {
            return SequencerReply::with(SequencerEvent::Add {
                track,
                time: frame.snapped(time, event.alt),
            });
        }
        match track.and_then(|track| hit(frame, keys, track, x)) {
            Some(key) => self.press_key(key, time, event.shift),
            None => {
                self.drag = Some(Drag::Boxed {
                    from: (x, y),
                    to: (x, y),
                    adding: event.shift,
                });
                SequencerReply::redraw()
            }
        }
    }

    /// Un appui sur une clé : elle est choisie (Maj : ajoutée au choix, ou retirée), et
    /// les clés choisies peuvent glisser.
    fn press_key(&mut self, key: KeyRef, time: f32, toggle: bool) -> SequencerReply {
        let chosen = self.selection.contains(&key);
        match (toggle, chosen) {
            (true, true) => {
                self.selection.retain(|k| *k != key);
                return SequencerReply::with(SequencerEvent::Select(self.selection.clone()));
            }
            (true, false) => self.selection.push(key),
            (false, false) => self.selection = vec![key],
            (false, true) => {}
        }
        self.selection.sort();
        self.drag = Some(Drag::Keys { from: time, by: 0. });
        SequencerReply::with(SequencerEvent::Select(self.selection.clone()))
    }

    fn drag_to(&mut self, frame: &Frame, event: PointerEvent) -> SequencerReply {
        let (x, y) = (event.local_x, event.local_y);
        match self.drag.as_mut() {
            Some(Drag::Scrub) => {
                let time = frame.snapped(frame.time_at(x), event.alt);
                SequencerReply::with(SequencerEvent::Seek(time))
            }
            Some(Drag::Keys { from, by }) => {
                let moved = frame.time_at(x) - *from;
                let snapped = frame.snapped(moved, event.alt);
                if snapped == *by {
                    return SequencerReply::default();
                }
                *by = snapped;
                SequencerReply::with(SequencerEvent::Move {
                    keys: self.selection.clone(),
                    by: snapped,
                    change: FieldChange::Preview,
                })
            }
            Some(Drag::Boxed { to, .. }) => {
                *to = (x, y);
                SequencerReply::redraw()
            }
            Some(Drag::Pan { at, start, scroll }) => {
                self.start = *start - (x - at.0) / frame.zoom;
                self.scroll = (*scroll - (y - at.1)).max(0.);
                SequencerReply::redraw()
            }
            None => SequencerReply::default(),
        }
    }

    fn release(&mut self, frame: &Frame, keys: &[Vec<f32>], event: PointerEvent) -> SequencerReply {
        match self.drag.take() {
            Some(Drag::Keys { by, .. }) if by != 0. => SequencerReply::with(SequencerEvent::Move {
                keys: self.selection.clone(),
                by,
                change: FieldChange::Commit,
            }),
            Some(Drag::Boxed { from, to, adding }) => {
                let small = (to.0 - from.0).abs() < 3. && (to.1 - from.1).abs() < 3.;
                if small {
                    // Un clic dans le vide : la tête de lecture y va, le choix se vide.
                    self.selection.clear();
                    let time = frame.snapped(frame.time_at(event.local_x), event.alt);
                    return SequencerReply::with(SequencerEvent::Seek(time));
                }
                let inside = boxed(frame, keys, from, to);
                if !adding {
                    self.selection.clear();
                }
                self.selection.extend(inside);
                self.selection.sort();
                self.selection.dedup();
                SequencerReply::with(SequencerEvent::Select(self.selection.clone()))
            }
            Some(_) => SequencerReply::redraw(),
            None => SequencerReply::default(),
        }
    }

    fn wheel(&mut self, frame: &Frame, event: PointerEvent) -> SequencerReply {
        let notches = event.wheel.1 + event.wheel.0;
        if notches == 0. {
            return SequencerReply::default();
        }
        if event.control {
            // Autour du pointeur : l'instant sous lui ne bouge pas.
            let under = frame.time_at(event.local_x);
            let zoom = (self.zoom * 1.2f32.powf(-notches)).clamp(ZOOM_RANGE.0, ZOOM_RANGE.1);
            self.start = under - (event.local_x - frame.names) / zoom;
            self.zoom = zoom;
        } else if event.shift || event.wheel.0 != 0. {
            self.start += notches * 60. / self.zoom;
        } else {
            self.scroll = (self.scroll + notches * frame.row * 3.).max(0.);
        }
        SequencerReply::redraw()
    }
}

/// La clé de la piste `track` sous l'abscisse `x`, la plus proche.
fn hit(frame: &Frame, keys: &[Vec<f32>], track: usize, x: f32) -> Option<KeyRef> {
    let times = keys.get(track)?;
    let (rank, distance) = times
        .iter()
        .enumerate()
        .map(|(rank, &time)| (rank, (frame.x_of(time) - x).abs()))
        .min_by(|a, b| a.1.total_cmp(&b.1))?;
    (distance <= HIT).then_some(KeyRef { track, key: rank })
}

/// Les clés dont le losange est dans le rectangle de `from` à `to`.
fn boxed(frame: &Frame, keys: &[Vec<f32>], from: (f32, f32), to: (f32, f32)) -> Vec<KeyRef> {
    let (left, right) = (from.0.min(to.0), from.0.max(to.0));
    let (top, bottom) = (from.1.min(to.1), from.1.max(to.1));
    let mut inside = Vec::new();
    for (track, times) in keys.iter().enumerate() {
        let middle = frame.ruler + (track as f32 + 0.5) * frame.row - frame.scroll;
        if middle < top || middle > bottom {
            continue;
        }
        for (key, &time) in times.iter().enumerate() {
            let x = frame.x_of(time);
            if x >= left && x <= right {
                inside.push(KeyRef { track, key });
            }
        }
    }
    inside
}
