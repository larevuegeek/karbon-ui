//! L'éditeur de courbes à l'écran : la grille et ses valeurs, les courbes tracées sur ce
//! qui se voit, les clés en carrés, les poignées de pente des clés choisies, le rectangle
//! de choix.
use super::{
    model::{Curve, KeyMode},
    state::{CurveEditorState, CurveMsg, Drag, Inner, handle, slope},
};
use crate::ui::{
    editor::{
        EditorPalette,
        canvas::{CanvasView, stroke},
    },
    element::{Element, ElementId, Length, PaintContext, SizeF, TextMeasurer, Widget},
    runtime::{PointerButton, PointerEvent, PointerPhase},
    scene::{DrawCommand, PointF, RectF, Scene},
    theme::Color,
};
use std::{cell::Cell, rc::Rc, sync::Arc};

mod paint_key;

/// Écart voulu entre deux lignes de la grille, en pixels ; demi-côté d'une clé.
const GRID_ROOM: f32 = 60.;
const KEY: f32 = 3.5;

/// An editor of curves (animation curves, a torque curve, the response of a steering):
/// keys moved by dragging (one step of history per gesture), their slopes by their
/// handles, a double click to add a key, a rectangle or Shift to choose several, the
/// middle button to slide the view, the wheel to zoom around the pointer (Ctrl: time
/// only, Shift: values only), Delete, the arrows and F (frame everything) from
/// [`CurveEditorState::key`]. The application keeps the curves and applies the
/// [`CurveEvent`](super::CurveEvent)s.
///
/// ```
/// use karbon_ui::ui::{editor::{Curve, CurveEditor, CurveEditorState, CurveKey, CurveMsg}, prelude::*};
///
/// #[derive(Clone)]
/// enum Msg {
///     Curves(CurveMsg),
/// }
/// let curves = vec![Curve { name: "Couple".into(), keys: vec![CurveKey::new(0., 0.), CurveKey::new(1., 1.)], ..Default::default() }];
/// let state = CurveEditorState::default();
/// let editor = CurveEditor::new(1, &curves, &state, Msg::Curves).element();
/// # let _ = editor;
/// ```
pub struct CurveEditor<M> {
    key: u64,
    curves: Vec<Curve>,
    state: CurveEditorState,
    palette: EditorPalette,
    map: Rc<dyn Fn(CurveMsg) -> M>,
}

impl<M: 'static> CurveEditor<M> {
    /// The editor of `key` showing `curves` in the state `state`; `map` wraps its gestures
    /// in the application's message.
    pub fn new(
        key: u64,
        curves: &[Curve],
        state: &CurveEditorState,
        map: impl Fn(CurveMsg) -> M + 'static,
    ) -> Self {
        Self {
            key,
            curves: curves.to_vec(),
            state: state.clone(),
            palette: EditorPalette::DARK,
            map: Rc::new(map),
        }
    }

    /// Colours and sizes ([`EditorPalette::DARK`] by default).
    pub fn palette(mut self, palette: EditorPalette) -> Self {
        self.palette = palette;
        self
    }

    /// The element, filling its parent.
    pub fn element(self) -> Element {
        let id = ElementId::named("karbon-curves").item(self.key);
        let size = Rc::new(Cell::new(self.state.size));
        let dragging = self.state.drag.is_some();
        let painter = Painter {
            curves: self.curves,
            state: self.state,
            palette: self.palette,
            size: Rc::clone(&size),
        };
        let (on_pointer, on_wheel) = (Rc::clone(&self.map), Rc::clone(&self.map));
        let wheel_size = Rc::clone(&size);
        Element::custom(id.0, painter)
            .width(Length::Fill(1.))
            .height(Length::Fill(1.))
            .on_pointer(move |event: PointerEvent| {
                let message = Inner::Pointer {
                    size: size.get(),
                    event,
                };
                wanted(event, dragging).then(|| on_pointer(CurveMsg(message)))
            })
            .on_pointer_wheel(move |event: PointerEvent| {
                let message = Inner::Pointer {
                    size: wheel_size.get(),
                    event,
                };
                Some(on_wheel(CurveMsg(message)))
            })
    }
}

/// Ce que l'éditeur prend : un appui du bouton gauche ou du milieu, puis pendant un geste.
fn wanted(event: PointerEvent, dragging: bool) -> bool {
    match event.phase {
        PointerPhase::Down => matches!(
            event.button,
            Some(PointerButton::Primary | PointerButton::Middle)
        ),
        PointerPhase::Move | PointerPhase::Up => dragging,
        _ => false,
    }
}

/// La peinture de l'éditeur, qui retient sa taille pour son gestionnaire du pointeur.
struct Painter {
    curves: Vec<Curve>,
    state: CurveEditorState,
    palette: EditorPalette,
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
        self.size
            .set(((r.right - r.left).max(0.), (r.bottom - r.top).max(0.)));
        scene.fill(r, 0., self.palette.panel);
        scene.commands.push(DrawCommand::PushClip(r));
        self.paint_grid(r, scene);
        for (index, curve) in self.curves.iter().enumerate() {
            self.paint_curve(r, index, curve, scene);
        }
        if let Some(Drag::Boxed { from, to, .. }) = self.state.drag {
            let rect = RectF {
                left: r.left + from.x.min(to.x),
                top: r.top + from.y.min(to.y),
                right: r.left + from.x.max(to.x),
                bottom: r.top + from.y.max(to.y),
            };
            let tint = Color {
                alpha: 40,
                ..self.palette.accent
            };
            scene.fill(rect, 0., tint);
        }
        scene.commands.push(DrawCommand::PopClip);
    }
}

/// Un pas « rond » (1, 2 ou 5 fois une puissance de dix) d'au moins `minimum`.
fn nice_step(minimum: f32) -> f32 {
    let power = 10f32.powf(minimum.max(1e-6).log10().floor());
    [1., 2., 5., 10.]
        .into_iter()
        .map(|m| m * power)
        .find(|s| *s >= minimum)
        .unwrap_or(10. * power)
}

impl Painter {
    fn view(&self) -> CanvasView {
        self.state.view
    }

    /// Les lignes de la grille, en temps et en valeur, et leurs nombres.
    fn paint_grid(&self, r: RectF, scene: &mut Scene) {
        let p = self.palette;
        let view = self.view();
        let (width, height) = (r.right - r.left, r.bottom - r.top);
        let line = |scene: &mut Scene, a: PointF, b: PointF| {
            scene.commands.push(DrawCommand::Line {
                start: a,
                end: b,
                width: 1.,
                color: p.border,
            });
        };
        let step = nice_step(GRID_ROOM / view.zoom.0);
        let first = view.to_world(PointF { x: 0., y: 0. }).x;
        let mut t = (first / step).floor() * step;
        while view.to_screen(PointF { x: t, y: 0. }).x <= width {
            let x = r.left + view.to_screen(PointF { x: t, y: 0. }).x;
            line(scene, PointF { x, y: r.top }, PointF { x, y: r.bottom });
            let label = RectF::new(x + 3., r.top + 2., 60., p.font_size * 1.4);
            scene.text_single_line(
                label,
                Arc::from(format_number(t)),
                p.font_size - 1.,
                false,
                p.muted,
            );
            t += step;
        }
        let step = nice_step(GRID_ROOM / view.zoom.1);
        let lowest = view.to_world(PointF { x: 0., y: height }).y;
        let mut v = (lowest / step).floor() * step;
        while view.to_screen(PointF { x: 0., y: v }).y >= 0. {
            let y = r.top + view.to_screen(PointF { x: 0., y: v }).y;
            line(scene, PointF { x: r.left, y }, PointF { x: r.right, y });
            let label = RectF::new(r.left + 3., y - p.font_size * 1.4, 60., p.font_size * 1.4);
            scene.text_single_line(
                label,
                Arc::from(format_number(v)),
                p.font_size - 1.,
                false,
                p.muted,
            );
            v += step;
        }
    }

    /// Une courbe sur la largeur visible (un point tous les deux pixels), ses clés et les
    /// poignées des clés choisies.
    fn paint_curve(&self, r: RectF, index: usize, curve: &Curve, scene: &mut Scene) {
        if curve.keys.is_empty() {
            return;
        }
        let p = self.palette;
        let view = self.view();
        let color = curve.color.unwrap_or(p.accent);
        let width = r.right - r.left;
        let points: Vec<PointF> = (0..=(width / 2.).ceil() as usize)
            .map(|i| {
                let x = i as f32 * 2.;
                let t = view.to_world(PointF { x, y: 0. }).x;
                let y = view
                    .to_screen(PointF {
                        x: t,
                        y: curve.value_at(t),
                    })
                    .y;
                PointF {
                    x: r.left + x,
                    y: r.top + y,
                }
            })
            .collect();
        stroke(scene, &points, 1.5, color, None);
        for (k, key) in curve.keys.iter().enumerate() {
            let at = view.to_screen(PointF {
                x: key.time,
                y: key.value,
            });
            let at = PointF {
                x: r.left + at.x,
                y: r.top + at.y,
            };
            let chosen = self
                .state
                .selection
                .iter()
                .any(|s| s.curve == index && s.key == k);
            if chosen && key.mode == KeyMode::Smooth {
                for out in [false, true] {
                    let end = handle(&view, key.time, key.value, slope(key, out), out);
                    let end = PointF {
                        x: r.left + end.x,
                        y: r.top + end.y,
                    };
                    stroke(scene, &[at, end], 1., p.muted, None);
                    scene.fill(RectF::new(end.x - 3., end.y - 3., 6., 6.), 3., p.text);
                }
            }
            let fill = if chosen { p.text } else { color };
            let square = RectF::new(at.x - KEY, at.y - KEY, 2. * KEY, 2. * KEY);
            scene.fill(square, 1., fill);
        }
    }
}

/// Un nombre court pour la grille (« 0,5 » s'écrit « 0.5 »).
fn format_number(value: f32) -> String {
    let rounded = (value * 1000.).round() / 1000.;
    if rounded == 0. {
        "0".to_owned()
    } else {
        format!("{rounded}")
    }
}
