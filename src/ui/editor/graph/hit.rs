//! Ce qui est sous le pointeur sur une toile de graphe : la mini-carte, un port, un
//! nœud (le plus haut d'abord), un fil, le titre d'un commentaire, ou rien.
use super::{
    model::{Graph, GraphWire, HEADER, PortRef},
    plane::{GraphView, Minimap, wire_points},
};
use crate::ui::scene::PointF;

/// Distance du pointeur à un port, à un fil, en pixels, pour le prendre.
const PORT_REACH: f32 = 8.;
const WIRE_REACH: f32 = 5.;

/// Ce qui est sous le pointeur.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Hit {
    Minimap,
    Port { port: PortRef, output: bool },
    Node(u64),
    Wire(GraphWire),
    Comment(u64),
    Nothing,
}

/// Ce qui est sous le point `at` (à l'écran) d'une toile de `size` pixels.
pub(super) fn hit(graph: &Graph, view: &GraphView, size: (f32, f32), at: PointF) -> Hit {
    if Minimap::of(graph, view, size).is_some_and(|m| m.contains(at)) {
        return Hit::Minimap;
    }
    if let Some((port, output)) = port_at(graph, view, at) {
        return Hit::Port { port, output };
    }
    let world = view.to_world(at);
    let inside = |r: crate::ui::scene::RectF| {
        world.x >= r.left && world.x <= r.right && world.y >= r.top && world.y <= r.bottom
    };
    if let Some(node) = graph.nodes.iter().rev().find(|n| inside(n.rect())) {
        return Hit::Node(node.id);
    }
    if let Some(wire) = wire_at(graph, view, at) {
        return Hit::Wire(wire);
    }
    let title = |c: &&super::model::GraphComment| inside(c.rect) && world.y <= c.rect.top + HEADER;
    if let Some(comment) = graph.comments.iter().rev().find(title) {
        return Hit::Comment(comment.id);
    }
    Hit::Nothing
}

/// Le port le plus proche de `at`, à portée : le nœud, le rang, et s'il sort.
pub(super) fn port_at(graph: &Graph, view: &GraphView, at: PointF) -> Option<(PortRef, bool)> {
    let mut best: Option<(f32, PortRef, bool)> = None;
    for node in graph.nodes.iter().rev() {
        let ports = (node.inputs.len(), node.outputs.len());
        let all = (0..ports.0)
            .map(|p| (p, false))
            .chain((0..ports.1).map(|p| (p, true)));
        for (port, output) in all {
            let p = view.to_screen(node.port_point(port, output));
            let distance = (p.x - at.x).hypot(p.y - at.y);
            if distance <= PORT_REACH && best.is_none_or(|(d, _, _)| distance < d) {
                let reference = PortRef {
                    node: node.id,
                    port,
                };
                best = Some((distance, reference, output));
            }
        }
    }
    best.map(|(_, port, output)| (port, output))
}

/// Le fil qui passe à portée de `at`, le dernier dessiné d'abord.
fn wire_at(graph: &Graph, view: &GraphView, at: PointF) -> Option<GraphWire> {
    graph.wires.iter().rev().copied().find(|wire| {
        let Some((a, b)) = graph.wire_ends(wire) else {
            return false;
        };
        let (a, b) = (view.to_screen(a), view.to_screen(b));
        let left = a.x.min(b.x) - 60.;
        let right = a.x.max(b.x) + 60.;
        let (top, bottom) = (a.y.min(b.y) - WIRE_REACH, a.y.max(b.y) + WIRE_REACH);
        if at.x < left || at.x > right || at.y < top || at.y > bottom {
            return false;
        }
        wire_points(a, b)
            .windows(2)
            .any(|s| segment_distance(at, s[0], s[1]) <= WIRE_REACH)
    })
}

/// La distance de `p` au segment `a`–`b`.
fn segment_distance(p: PointF, a: PointF, b: PointF) -> f32 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let length = dx * dx + dy * dy;
    let t = if length > 0. {
        (((p.x - a.x) * dx + (p.y - a.y) * dy) / length).clamp(0., 1.)
    } else {
        0.
    };
    (p.x - a.x - t * dx).hypot(p.y - a.y - t * dy)
}
