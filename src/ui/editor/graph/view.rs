//! La toile de graphe à l'écran, peinte du fond vers le dessus : la grille, les
//! commentaires, les fils, les nœuds (titre, ports, noms des ports), le fil tiré, le
//! rectangle de choix, puis la mini-carte. Ce qui sort de la toile n'est pas peint.
use super::{
    model::{Graph, GraphNode, HEADER, ROW},
    plane::{GraphView, Minimap, wire_points},
    state::{Drag, GraphEditorState, GraphMsg},
};
use crate::ui::{
    editor::{EditorPalette, canvas::stroke},
    element::{Element, ElementId, Length, PaintContext, SizeF, TextMeasurer, Widget},
    runtime::{PointerButton, PointerEvent, PointerPhase},
    scene::{DrawCommand, PointF, RectF, Scene},
    theme::Color,
};
use std::{cell::Cell, rc::Rc};

/// Pas de la grille, en unités du plan ; rayon d'un port, en pixels.
const GRID: f32 = 20.;
const PORT: f32 = 4.5;

/// A node graph canvas (animation states, materials, behaviour trees): nodes with typed
/// ports, wires drawn from an output to an input (or picked up from an input and
/// dropped elsewhere), nodes dragged (one step of history per gesture), comments that
/// carry the nodes they frame, a rectangle or Shift to choose several, the middle
/// button to slide the view, the wheel to zoom around the pointer, a right click for
/// the application's menu (its search of the nodes to create), a mini-map to move
/// around, and the keys of [`GraphEditorState::key`]. The application keeps the
/// [`Graph`] and applies the [`GraphEvent`](super::GraphEvent)s.
///
/// ```
/// use karbon_ui::ui::{
///     editor::{Graph, GraphEditor, GraphEditorState, GraphMsg, GraphNode, GraphPort},
///     prelude::*,
///     scene::PointF,
/// };
///
/// #[derive(Clone)]
/// enum Msg {
///     Graph(GraphMsg),
/// }
/// let mut add = GraphNode::new(1, "Ajouter", PointF { x: 0., y: 0. });
/// add.inputs = vec![GraphPort::new("a", 1), GraphPort::new("b", 1)];
/// add.outputs = vec![GraphPort::new("somme", 1)];
/// let graph = Graph { nodes: vec![add], ..Graph::default() };
/// let state = GraphEditorState::default();
/// let canvas = GraphEditor::new(1, &graph, &state, Msg::Graph).element();
/// # let _ = canvas;
/// ```
pub struct GraphEditor<M> {
    key: u64,
    graph: Graph,
    state: GraphEditorState,
    palette: EditorPalette,
    map: Rc<dyn Fn(GraphMsg) -> M>,
}

impl<M: 'static> GraphEditor<M> {
    /// The canvas `key` showing `graph` in the state `state`; `map` wraps its gestures in
    /// the application's message.
    pub fn new(
        key: u64,
        graph: &Graph,
        state: &GraphEditorState,
        map: impl Fn(GraphMsg) -> M + 'static,
    ) -> Self {
        Self {
            key,
            graph: graph.clone(),
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
        let id = ElementId::named("karbon-graph").item(self.key);
        let size = Rc::new(Cell::new(self.state.size));
        let dragging = self.state.drag.is_some();
        let painter = Painter {
            graph: self.graph,
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
                let message = GraphMsg {
                    size: size.get(),
                    event,
                };
                wanted(event, dragging).then(|| on_pointer(message))
            })
            .on_pointer_wheel(move |event: PointerEvent| {
                let message = GraphMsg {
                    size: wheel_size.get(),
                    event,
                };
                Some(on_wheel(message))
            })
    }
}

/// Ce que la toile prend : un appui de n'importe quel bouton, puis pendant un geste.
fn wanted(event: PointerEvent, dragging: bool) -> bool {
    match event.phase {
        PointerPhase::Down => event.button.is_some(),
        PointerPhase::Move | PointerPhase::Up => {
            dragging && event.button != Some(PointerButton::Secondary)
        }
        PointerPhase::Wheel => false,
    }
}

/// La peinture de la toile, qui retient sa taille pour son gestionnaire du pointeur.
struct Painter {
    graph: Graph,
    state: GraphEditorState,
    palette: EditorPalette,
    size: Rc<Cell<(f32, f32)>>,
}

impl Widget for Painter {
    fn measure(&self, _: SizeF, _: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
        Ok(SizeF::default())
    }

    fn paint(&self, context: &PaintContext, scene: &mut Scene) {
        let r = context.rect;
        let size = ((r.right - r.left).max(0.), (r.bottom - r.top).max(0.));
        self.size.set(size);
        scene.fill(r, 0., self.palette.panel);
        scene.commands.push(DrawCommand::PushClip(r));
        let canvas = Canvas {
            origin: PointF {
                x: r.left,
                y: r.top,
            },
            size,
            view: self.state.view,
        };
        self.paint_grid(&canvas, scene);
        self.paint_comments(&canvas, scene);
        self.paint_wires(&canvas, scene);
        for node in &self.graph.nodes {
            self.paint_node(&canvas, node, scene);
        }
        self.paint_gesture(&canvas, scene);
        self.paint_minimap(&canvas, scene);
        scene.commands.push(DrawCommand::PopClip);
    }
}

/// La toile à l'écran : son coin, sa taille, sa vue.
struct Canvas {
    origin: PointF,
    size: (f32, f32),
    view: GraphView,
}

impl Canvas {
    /// Le point de la fenêtre d'un point du plan.
    fn screen(&self, world: PointF) -> PointF {
        let p = self.view.to_screen(world);
        PointF {
            x: self.origin.x + p.x,
            y: self.origin.y + p.y,
        }
    }

    /// Le rectangle de la fenêtre d'un rectangle du plan.
    fn rect(&self, world: RectF) -> RectF {
        let r = self.view.rect_to_screen(world);
        RectF {
            left: self.origin.x + r.left,
            top: self.origin.y + r.top,
            right: self.origin.x + r.right,
            bottom: self.origin.y + r.bottom,
        }
    }

    /// Le rectangle `r` (fenêtre) touche-t-il la toile ?
    fn shows(&self, r: RectF) -> bool {
        r.right >= self.origin.x
            && r.left <= self.origin.x + self.size.0
            && r.bottom >= self.origin.y
            && r.top <= self.origin.y + self.size.1
    }
}

/// `color` avec l'opacité `alpha`.
fn faded(color: Color, alpha: u8) -> Color {
    Color { alpha, ..color }
}

impl Painter {
    fn paint_grid(&self, c: &Canvas, scene: &mut Scene) {
        let step = GRID * c.view.zoom;
        if step < 8. {
            return;
        }
        let first = c.view.to_world(PointF { x: 0., y: 0. });
        let (mut x, top) = (
            (first.x / GRID).floor() * GRID,
            (first.y / GRID).floor() * GRID,
        );
        let color = faded(self.palette.border, 90);
        while c.view.to_screen(PointF { x, y: 0. }).x <= c.size.0 {
            let sx = c.screen(PointF { x, y: 0. }).x;
            let start = PointF {
                x: sx,
                y: c.origin.y,
            };
            let end = PointF {
                x: sx,
                y: c.origin.y + c.size.1,
            };
            scene.commands.push(DrawCommand::Line {
                start,
                end,
                width: 1.,
                color,
            });
            x += GRID * 5.;
        }
        let mut y = top;
        while c.view.to_screen(PointF { x: 0., y }).y <= c.size.1 {
            let sy = c.screen(PointF { x: 0., y }).y;
            let start = PointF {
                x: c.origin.x,
                y: sy,
            };
            let end = PointF {
                x: c.origin.x + c.size.0,
                y: sy,
            };
            scene.commands.push(DrawCommand::Line {
                start,
                end,
                width: 1.,
                color,
            });
            y += GRID * 5.;
        }
    }

    fn paint_comments(&self, c: &Canvas, scene: &mut Scene) {
        let p = self.palette;
        for comment in &self.graph.comments {
            let r = c.rect(comment.rect);
            if !c.shows(r) {
                continue;
            }
            let color = comment.color.unwrap_or(p.accent);
            scene.fill(r, 6. * c.view.zoom, faded(color, 34));
            let chosen = self.state.comments.contains(&comment.id);
            let edge = if chosen { p.text } else { faded(color, 140) };
            scene.commands.push(DrawCommand::Stroke {
                rect: r,
                radius: 6. * c.view.zoom,
                width: 1.,
                color: edge,
            });
            if c.view.zoom >= 0.4 {
                let title = RectF::new(
                    r.left + 8.,
                    r.top + 4.,
                    (r.right - r.left - 16.).max(0.),
                    20.,
                );
                scene.text_single_line(
                    title,
                    comment.text.as_str(),
                    p.font_size + 1.,
                    true,
                    p.text,
                );
            }
        }
    }

    fn paint_wires(&self, c: &Canvas, scene: &mut Scene) {
        let width = (2. * c.view.zoom).clamp(1., 3.);
        for wire in &self.graph.wires {
            let Some((a, b)) = self.graph.wire_ends(wire) else {
                continue;
            };
            let (a, b) = (c.screen(a), c.screen(b));
            let bounds = RectF {
                left: a.x.min(b.x) - 60.,
                top: a.y.min(b.y),
                right: a.x.max(b.x) + 60.,
                bottom: a.y.max(b.y),
            };
            if !c.shows(bounds) {
                continue;
            }
            let color = self
                .graph
                .node(wire.from.node)
                .and_then(|n| n.outputs.get(wire.from.port))
                .and_then(|port| port.color)
                .unwrap_or(self.palette.accent);
            let lit = self.state.selection.contains(&wire.from.node)
                || self.state.selection.contains(&wire.to.node);
            let (color, width) = match (self.state.wire == Some(*wire), lit) {
                (true, _) => (self.palette.text, width + 1.5),
                (false, true) => (color, width),
                (false, false) => (faded(color, 170), width),
            };
            stroke(scene, &wire_points(a, b), width, color, None);
        }
    }

    fn paint_node(&self, c: &Canvas, node: &GraphNode, scene: &mut Scene) {
        let p = self.palette;
        let r = c.rect(node.rect());
        if !c.shows(r) {
            return;
        }
        let z = c.view.zoom;
        let radius = 6. * z;
        scene.fill(r, radius, p.field);
        let head = RectF {
            bottom: r.top + HEADER * z,
            ..r
        };
        scene.fill(head, radius, node.color.unwrap_or(p.hover));
        let chosen = self.state.selection.contains(&node.id);
        let edge = match (chosen, node.mark) {
            (true, _) => Some((p.accent, 2.)),
            (false, Some(mark)) => Some((mark, 2.)),
            (false, None) => Some((p.border, 1.)),
        };
        if let Some((color, width)) = edge {
            scene.commands.push(DrawCommand::Stroke {
                rect: r,
                radius,
                width,
                color,
            });
        }
        let text = z >= 0.45;
        let font = p.font_size * z.min(1.4);
        if text {
            let title = RectF::new(
                r.left + 8. * z,
                r.top + 5. * z,
                (r.right - r.left - 16. * z).max(0.),
                HEADER * z,
            );
            scene.text_single_line(title, node.title.as_str(), font, true, p.text);
            if let Some(sub) = &node.subtitle {
                let line = RectF::new(
                    r.left + 8. * z,
                    head.bottom + 1.,
                    (r.right - r.left - 16. * z).max(0.),
                    ROW * z,
                );
                scene.text_single_line(line, sub.as_str(), font - 1., false, p.muted);
            }
        }
        for (output, ports) in [(false, &node.inputs), (true, &node.outputs)] {
            for (rank, port) in ports.iter().enumerate() {
                let at = c.screen(node.port_point(rank, output));
                let dot = RectF::new(at.x - PORT, at.y - PORT, 2. * PORT, 2. * PORT);
                scene.fill(dot, PORT, port.color.unwrap_or(p.accent));
                if text && !port.name.is_empty() {
                    let width = (r.right - r.left) / 2. - 12. * z;
                    let left = if output {
                        at.x - 10. * z - width
                    } else {
                        at.x + 10. * z
                    };
                    let label = RectF::new(left, at.y - ROW * z / 2., width.max(0.), ROW * z);
                    scene.text_single_line(label, port.name.as_str(), font - 1., false, p.muted);
                }
            }
        }
    }

    /// Le fil tiré et le rectangle de choix.
    fn paint_gesture(&self, c: &Canvas, scene: &mut Scene) {
        match &self.state.drag {
            Some(Drag::Wire {
                anchor, output, at, ..
            }) => {
                let Some(node) = self.graph.node(anchor.node) else {
                    return;
                };
                let from = c.screen(node.port_point(anchor.port, *output));
                let to = PointF {
                    x: c.origin.x + at.x,
                    y: c.origin.y + at.y,
                };
                let (a, b) = if *output { (from, to) } else { (to, from) };
                stroke(
                    scene,
                    &wire_points(a, b),
                    2.,
                    self.palette.text,
                    Some((6., 4.)),
                );
            }
            Some(Drag::Boxed { from, to, .. }) => {
                let rect = RectF {
                    left: c.origin.x + from.x.min(to.x),
                    top: c.origin.y + from.y.min(to.y),
                    right: c.origin.x + from.x.max(to.x),
                    bottom: c.origin.y + from.y.max(to.y),
                };
                scene.fill(rect, 0., faded(self.palette.accent, 40));
            }
            _ => {}
        }
    }

    fn paint_minimap(&self, c: &Canvas, scene: &mut Scene) {
        let Some(map) = Minimap::of(&self.graph, &c.view, c.size) else {
            return;
        };
        let p = self.palette;
        let shift = |r: RectF| RectF {
            left: c.origin.x + r.left,
            top: c.origin.y + r.top,
            right: c.origin.x + r.right,
            bottom: c.origin.y + r.bottom,
        };
        let frame = shift(map.rect);
        scene.fill(frame, 4., faded(p.field, 230));
        let to_map = |world: RectF| {
            let a = map.to_screen(PointF {
                x: world.left,
                y: world.top,
            });
            let b = map.to_screen(PointF {
                x: world.right,
                y: world.bottom,
            });
            shift(RectF {
                left: a.x,
                top: a.y,
                right: b.x.max(a.x + 1.),
                bottom: b.y.max(a.y + 1.),
            })
        };
        for node in &self.graph.nodes {
            let chosen = self.state.selection.contains(&node.id);
            let color = if chosen { p.accent } else { p.muted };
            scene.fill(to_map(node.rect()), 0., color);
        }
        let seen = RectF {
            left: c.view.origin.x,
            top: c.view.origin.y,
            right: c.view.origin.x + c.size.0 / c.view.zoom,
            bottom: c.view.origin.y + c.size.1 / c.view.zoom,
        };
        let seen = to_map(seen);
        scene.commands.push(DrawCommand::Stroke {
            rect: seen,
            radius: 0.,
            width: 1.,
            color: p.text,
        });
        scene.commands.push(DrawCommand::Stroke {
            rect: frame,
            radius: 4.,
            width: 1.,
            color: p.border,
        });
    }
}
