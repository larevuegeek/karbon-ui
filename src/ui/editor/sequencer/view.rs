//! La frise à l'écran : la règle du temps, la colonne des noms, les pistes et leurs
//! clés en losanges, la tête de lecture, la fin de l'animation et le rectangle de choix.
//! Seules les pistes et les clés visibles sont peintes.
use super::state::{Drag, Frame, Inner, KeyRef, SequencerMsg, SequencerState};
use crate::ui::{
    editor::EditorPalette,
    element::{Element, ElementId, Length, PaintContext, SizeF, TextMeasurer, Widget},
    runtime::{Cursor, PointerButton, PointerEvent, PointerPhase},
    scene::{DrawCommand, PointF, RectF, Scene},
    theme::Color,
};
use std::{cell::Cell, rc::Rc, sync::Arc};

pub use super::track::SequencerTrack;
mod paint_key;

/// Largeur de la colonne des noms, hauteur de la règle, demi-diagonale d'un losange.
const NAMES: f32 = 150.;
const RULER: f32 = 24.;
const DIAMOND: f32 = 5.;
/// Écart voulu entre deux graduations nommées, en pixels.
const TICK_ROOM: f32 = 70.;
/// Les pas des graduations, en secondes.
const STEPS: [f32; 16] = [
    0.01, 0.02, 0.05, 0.1, 0.2, 0.25, 0.5, 1., 2., 5., 10., 15., 30., 60., 120., 300.,
];

/// A time line for dense editors: a ruler of time, tracks with their keys as diamonds, a
/// play head, the end of the animation, a choice of keys. Its gestures are told by
/// [`SequencerState::update`]; the application keeps the keys and the time of the play
/// head. Only the tracks and the keys in view are painted: thousands of tracks and tens
/// of thousands of keys stay quick.
///
/// ```
/// use karbon_ui::ui::{
///     editor::{Sequencer, SequencerEvent, SequencerMsg, SequencerState, SequencerTrack},
///     prelude::*,
/// };
///
/// #[derive(Clone)]
/// enum Msg {
///     Line(SequencerMsg),
/// }
/// #[derive(Default)]
/// struct Door {
///     time: f32,
///     line: SequencerState,
/// }
/// impl Application for Door {
///     type Message = Msg;
///     fn view(&self, _: &ViewContext) -> Element {
///         let track = SequencerTrack { name: "Rotation".into(), keys: vec![0.0, 1.0], ..Default::default() };
///         Sequencer::new(1, vec![track], self.time, &self.line, Msg::Line).length(1.0).element()
///     }
///     fn update(&mut self, message: Msg) -> bool {
///         let Msg::Line(message) = message;
///         let reply = self.line.update(message);
///         if let Some(SequencerEvent::Seek(time)) = reply.event {
///             self.time = time;
///         }
///         reply.rebuild
///     }
/// }
/// ```
pub struct Sequencer<M> {
    key: u64,
    tracks: Vec<SequencerTrack>,
    time: f32,
    length: Option<f32>,
    snap: Option<f32>,
    decimal_comma: bool,
    palette: EditorPalette,
    state: SequencerState,
    map: Rc<dyn Fn(SequencerMsg) -> M>,
}

impl<M: 'static> Sequencer<M> {
    /// The sequencer of `key` showing `tracks`, its play head at `time` seconds; `map`
    /// wraps its gestures in the application's message.
    pub fn new(
        key: u64,
        tracks: Vec<SequencerTrack>,
        time: f32,
        state: &SequencerState,
        map: impl Fn(SequencerMsg) -> M + 'static,
    ) -> Self {
        Self {
            key,
            tracks,
            time,
            length: None,
            snap: None,
            decimal_comma: false,
            palette: EditorPalette::DARK,
            state: state.clone(),
            map: Rc::new(map),
        }
    }

    /// The length of the animation: the time beyond it is shaded.
    pub fn length(mut self, seconds: f32) -> Self {
        self.length = Some(seconds);
        self
    }

    /// Snap the play head and the keys moved to multiples of `step` seconds (Alt held:
    /// free).
    pub fn snap(mut self, step: f32) -> Self {
        self.snap = (step > 0.).then_some(step);
        self
    }

    /// Write the times of the ruler with a decimal comma (`0,5`).
    pub fn decimal_comma(mut self, comma: bool) -> Self {
        self.decimal_comma = comma;
        self
    }

    /// Colours and sizes ([`EditorPalette::DARK`] by default).
    pub fn palette(mut self, palette: EditorPalette) -> Self {
        self.palette = palette;
        self
    }

    /// The element, filling its parent.
    pub fn element(self) -> Element {
        let id = ElementId::named("karbon-sequencer").item(self.key);
        let size = Rc::new(Cell::new((0., 0.)));
        let frame = Frame {
            key: self.key,
            size: (0., 0.),
            names: NAMES,
            ruler: RULER,
            row: self.palette.row_height,
            start: self.state.start,
            zoom: self.state.zoom,
            scroll: self.state.scroll,
            snap: self.snap,
        };
        let keys: Arc<Vec<Vec<f32>>> =
            Arc::new(self.tracks.iter().map(|t| t.keys.clone()).collect());
        let dragging = self.state.drag.is_some();
        let painter = Painter {
            frame,
            tracks: self.tracks,
            time: self.time,
            length: self.length,
            comma: self.decimal_comma,
            palette: self.palette,
            selection: self.state.selection.clone(),
            drag: self.state.drag.clone(),
            size: Rc::clone(&size),
        };
        let (on_pointer, on_wheel) = (Rc::clone(&self.map), Rc::clone(&self.map));
        let wheel_size = Rc::clone(&size);
        let mut element = Element::custom(id.0, painter)
            .width(Length::Fill(1.))
            .height(Length::Fill(1.))
            .on_pointer(move |event: PointerEvent| {
                let frame = Frame {
                    size: size.get(),
                    ..frame
                };
                let message = Inner::Pointer {
                    frame,
                    keys: Arc::clone(&keys),
                    event,
                };
                wanted(event, dragging).then(|| on_pointer(SequencerMsg(message)))
            })
            .on_pointer_wheel(move |event: PointerEvent| {
                let frame = Frame {
                    size: wheel_size.get(),
                    ..frame
                };
                Some(on_wheel(SequencerMsg(Inner::Wheel { frame, event })))
            });
        element
            .styles
            .get_or_insert_with(Default::default)
            .normal
            .cursor = Some(Cursor::Arrow);
        element
    }
}

/// Ce que la frise prend : un appui de l'un des trois boutons, puis pendant un geste.
fn wanted(event: PointerEvent, dragging: bool) -> bool {
    match event.phase {
        PointerPhase::Down => matches!(
            event.button,
            Some(PointerButton::Primary | PointerButton::Secondary | PointerButton::Middle)
        ),
        PointerPhase::Move | PointerPhase::Up => dragging,
        PointerPhase::Wheel => false,
    }
}

/// La peinture de la frise, qui retient sa taille pour son gestionnaire du pointeur.
struct Painter {
    frame: Frame,
    tracks: Vec<SequencerTrack>,
    time: f32,
    length: Option<f32>,
    comma: bool,
    palette: EditorPalette,
    selection: Vec<KeyRef>,
    drag: Option<Drag>,
    size: Rc<Cell<(f32, f32)>>,
}

impl Widget for Painter {
    fn measure(&self, _: SizeF, _: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
        Ok(SizeF::default())
    }

    fn paint_key(&self) -> Option<u64> {
        Some(self.key())
    }

    fn paint_reused(&self, context: &PaintContext) {
        let r = context.rect;
        self.size
            .set(((r.right - r.left).max(0.), (r.bottom - r.top).max(0.)));
    }

    fn paint(&self, context: &PaintContext, scene: &mut Scene) {
        let r = context.rect;
        let (width, height) = ((r.right - r.left).max(0.), (r.bottom - r.top).max(0.));
        self.size.set((width, height));
        let p = self.palette;
        scene.fill(r, 0., p.panel);
        scene.commands.push(DrawCommand::PushClip(r));
        self.paint_tracks(r, scene);
        self.paint_ruler(r, scene);
        self.paint_head(r, scene);
        if let Some(Drag::Boxed { from, to, .. }) = self.drag {
            let rect = RectF {
                left: r.left + from.0.min(to.0),
                top: r.top + from.1.min(to.1),
                right: r.left + from.0.max(to.0),
                bottom: r.top + from.1.max(to.1),
            };
            scene.fill(
                rect,
                0.,
                Color {
                    alpha: 40,
                    ..p.accent
                },
            );
            stroke(scene, rect, p.accent);
        }
        scene.commands.push(DrawCommand::PopClip);
    }
}

impl Painter {
    /// Les pistes visibles : leur fond, leur nom, leurs clés.
    fn paint_tracks(&self, r: RectF, scene: &mut Scene) {
        let (f, p) = (&self.frame, self.palette);
        let lanes = RectF {
            left: r.left + f.names,
            top: r.top + f.ruler,
            ..r
        };
        if let Some(length) = self.length {
            let end = (r.left + f.x_of(length)).max(lanes.left);
            if end < r.right {
                scene.fill(RectF { left: end, ..lanes }, 0., p.field);
            }
        }
        let first = (f.scroll / f.row).floor().max(0.) as usize;
        let shown = ((r.bottom - lanes.top) / f.row).ceil() as usize + 1;
        let (t0, t1) = (
            f.time_at(f.names - DIAMOND * 2.),
            f.time_at(r.right - r.left + DIAMOND * 2.),
        );
        for track in first..(first + shown).min(self.tracks.len()) {
            let top = lanes.top + track as f32 * f.row - f.scroll;
            let row = RectF::new(r.left, top, r.right - r.left, f.row);
            if track % 2 == 1 {
                scene.fill(
                    RectF {
                        left: lanes.left,
                        ..row
                    },
                    0.,
                    Color {
                        alpha: 60,
                        ..p.field
                    },
                );
            }
            scene.commands.push(DrawCommand::Line {
                start: PointF {
                    x: r.left,
                    y: row.bottom - 0.5,
                },
                end: PointF {
                    x: r.right,
                    y: row.bottom - 0.5,
                },
                width: 1.,
                color: Color {
                    alpha: 90,
                    ..p.border
                },
            });
            self.paint_name(&self.tracks[track], row, scene);
            let keys = &self.tracks[track].keys;
            let from = keys.partition_point(|&t| t < t0);
            let to = keys.partition_point(|&t| t <= t1);
            let color = self.tracks[track].color.unwrap_or(p.text);
            for (rank, &time) in keys.iter().enumerate().take(to).skip(from) {
                let chosen = self
                    .selection
                    .binary_search(&KeyRef { track, key: rank })
                    .is_ok();
                let centre = PointF {
                    x: r.left + f.x_of(time),
                    y: top + f.row / 2.,
                };
                diamond(scene, centre, if chosen { p.accent } else { color }, chosen);
            }
        }
        // La colonne des noms, séparée des pistes.
        scene.commands.push(DrawCommand::Line {
            start: PointF {
                x: lanes.left - 0.5,
                y: r.top,
            },
            end: PointF {
                x: lanes.left - 0.5,
                y: r.bottom,
            },
            width: 1.,
            color: p.border,
        });
    }

    fn paint_name(&self, track: &SequencerTrack, row: RectF, scene: &mut Scene) {
        let p = self.palette;
        let indent = 8. + f32::from(track.indent) * 12.;
        let rect = RectF {
            left: row.left + indent,
            right: row.left + self.frame.names - 4.,
            ..row
        };
        let color = if track.indent > 0 { p.muted } else { p.text };
        let top = rect.top + (row.bottom - row.top - p.font_size * 1.3) / 2.;
        scene.text_single_line(
            RectF { top, ..rect },
            track.name.as_str(),
            p.font_size,
            false,
            color,
        );
    }

    /// La règle : son fond, ses graduations et les instants écrits.
    fn paint_ruler(&self, r: RectF, scene: &mut Scene) {
        let (f, p) = (&self.frame, self.palette);
        let ruler = RectF::new(r.left, r.top, r.right - r.left, f.ruler);
        scene.fill(ruler, 0., p.field);
        let step = STEPS
            .into_iter()
            .find(|step| step * f.zoom >= TICK_ROOM)
            .unwrap_or(600.);
        let first = (f.time_at(f.names) / step).floor() as i64;
        let last = (f.time_at(r.right - r.left) / step).ceil() as i64;
        for tick in first..=last {
            let time = tick as f32 * step;
            let x = r.left + f.x_of(time);
            if x < r.left + f.names {
                continue;
            }
            // Cinq petites graduations entre deux instants écrits.
            for minor in 1..5 {
                let mx = x + minor as f32 * step * f.zoom / 5.;
                line(scene, (mx, ruler.bottom - 4.), (mx, ruler.bottom), p.border);
            }
            line(scene, (x, ruler.top + 4.), (x, ruler.bottom), p.muted);
            let label = RectF::new(x + 3., ruler.top + 3., 60., p.font_size * 1.3);
            scene.text_single_line(
                label,
                seconds(time, step, self.comma),
                p.font_size - 1.,
                false,
                p.muted,
            );
        }
        line(
            scene,
            (r.left, ruler.bottom - 0.5),
            (r.right, ruler.bottom - 0.5),
            p.border,
        );
    }

    /// La tête de lecture : un trait sur toutes les pistes et une marque sur la règle.
    fn paint_head(&self, r: RectF, scene: &mut Scene) {
        let f = &self.frame;
        let x = r.left + f.x_of(self.time);
        if x < r.left + f.names || x > r.right {
            return;
        }
        let red = self.palette.axes[0];
        line(scene, (x, r.top + f.ruler - 8.), (x, r.bottom), red);
        let points = vec![
            PointF {
                x: x - 6.,
                y: r.top + f.ruler - 12.,
            },
            PointF {
                x: x + 6.,
                y: r.top + f.ruler - 12.,
            },
            PointF {
                x,
                y: r.top + f.ruler - 4.,
            },
        ];
        scene.commands.push(DrawCommand::Path {
            points,
            closed: true,
            color: red,
        });
    }
}

/// Un trait d'un pixel.
fn line(scene: &mut Scene, from: (f32, f32), to: (f32, f32), color: Color) {
    scene.commands.push(DrawCommand::Line {
        start: PointF {
            x: from.0,
            y: from.1,
        },
        end: PointF { x: to.0, y: to.1 },
        width: 1.,
        color,
    });
}

/// Le tour d'un rectangle.
fn stroke(scene: &mut Scene, rect: RectF, color: Color) {
    scene.commands.push(DrawCommand::Stroke {
        rect,
        radius: 0.,
        width: 1.,
        color,
    });
}

/// Un losange plein autour de `centre` ; une clé choisie est plus grande.
fn diamond(scene: &mut Scene, centre: PointF, color: Color, chosen: bool) {
    let half = if chosen { DIAMOND + 1.5 } else { DIAMOND };
    let points = vec![
        PointF {
            x: centre.x,
            y: centre.y - half,
        },
        PointF {
            x: centre.x + half,
            y: centre.y,
        },
        PointF {
            x: centre.x,
            y: centre.y + half,
        },
        PointF {
            x: centre.x - half,
            y: centre.y,
        },
    ];
    scene.commands.push(DrawCommand::Path {
        points,
        closed: true,
        color,
    });
}

/// Un instant de la règle : autant de décimales que le pas en demande, puis « s ».
pub(super) fn seconds(time: f32, step: f32, comma: bool) -> String {
    let decimals = match step {
        s if s >= 1. => 0,
        s if s >= 0.1 => 1,
        _ => 2,
    };
    let mut text = format!("{time:.decimals$}");
    if text == "-0" || text.starts_with("-0") && text.trim_start_matches(['-', '0', '.']).is_empty()
    {
        text = text.trim_start_matches('-').to_owned();
    }
    if comma {
        text = text.replace('.', ",");
    }
    format!("{text} s")
}
