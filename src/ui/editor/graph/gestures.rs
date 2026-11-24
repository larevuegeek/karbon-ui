//! Les gestes du pointeur sur une toile de graphe : appui, mouvement, relâchement,
//! molette. Un fil se tire d'une sortie vers une entrée (ou l'inverse) ; tiré d'une entrée
//! déjà branchée, il est repris et se pose ailleurs ou se débranche.
use super::{
    hit::{Hit, hit, port_at},
    model::{Graph, GraphWire, PortRef},
    plane::Minimap,
    state::{Drag, GraphEditorState, GraphEvent, GraphReply, reply},
};
use crate::ui::{
    editor::FieldChange,
    runtime::{PointerButton, PointerEvent, PointerPhase},
    scene::{PointF, RectF},
};

/// Un geste du pointeur, à `event.local_x`, `event.local_y` dans la toile.
pub(super) fn pointer(
    state: &mut GraphEditorState,
    event: PointerEvent,
    graph: &Graph,
) -> GraphReply {
    let at = PointF {
        x: event.local_x,
        y: event.local_y,
    };
    match event.phase {
        PointerPhase::Down => press(state, event, at, graph),
        PointerPhase::Move => moved(state, at),
        PointerPhase::Up => released(state, at, graph),
        PointerPhase::Wheel => {
            state.view.zoom_at(at, (-event.wheel.1 * 0.15).exp());
            reply(None)
        }
    }
}

fn press(
    state: &mut GraphEditorState,
    event: PointerEvent,
    at: PointF,
    graph: &Graph,
) -> GraphReply {
    let found = hit(graph, &state.view, state.size, at);
    match event.button {
        Some(PointerButton::Middle) => {
            state.drag = Some(Drag::Pan { last: at });
            return reply(None);
        }
        Some(PointerButton::Secondary) => {
            let node = match found {
                Hit::Node(id) => Some(id),
                Hit::Port { port, .. } => Some(port.node),
                _ => None,
            };
            let at_world = state.view.to_world(at);
            return reply(Some(GraphEvent::Menu {
                at: at_world,
                screen: at,
                node,
            }));
        }
        Some(PointerButton::Primary) => {}
        None => return GraphReply::default(),
    }
    match found {
        Hit::Minimap => {
            let map = Minimap::of(graph, &state.view, state.size);
            if let Some(map) = map {
                state.view.center_on(map.to_world(at), state.size);
                state.drag = Some(Drag::Minimap(map));
            }
            reply(None)
        }
        Hit::Port { port, output } => press_port(state, event.alt, (port, output), at, graph),
        Hit::Node(id) if event.clicks >= 2 => reply(Some(GraphEvent::Open(id))),
        Hit::Node(id) => {
            state.wire = None;
            choose(&mut state.selection, id, event.shift);
            if !event.shift {
                state.comments.clear();
            }
            start_items(state, at, graph);
            reply(None)
        }
        Hit::Comment(id) => {
            choose(&mut state.comments, id, event.shift);
            if !event.shift {
                state.selection.clear();
            }
            start_items(state, at, graph);
            reply(None)
        }
        Hit::Wire(wire) if event.alt => reply(Some(GraphEvent::Disconnect(wire))),
        Hit::Wire(wire) => {
            state.set_chosen_wire(Some(wire));
            reply(Some(GraphEvent::ChooseWire(wire)))
        }
        Hit::Nothing => {
            state.wire = None;
            if !event.shift {
                state.selection.clear();
                state.comments.clear();
            }
            state.drag = Some(Drag::Boxed {
                from: at,
                to: at,
                add: event.shift,
            });
            reply(None)
        }
    }
}

/// `id` choisi seul, ou ajouté et retiré du choix avec Maj ; déjà choisi sans Maj, le
/// choix reste (on tire tout ce qui est choisi).
fn choose(list: &mut Vec<u64>, id: u64, shift: bool) {
    match (shift, list.iter().position(|&x| x == id)) {
        (true, Some(index)) => drop(list.remove(index)),
        (true, None) => list.push(id),
        (false, Some(_)) => {}
        (false, None) => *list = vec![id],
    }
}

/// Le début du glisser de ce qui est choisi : les nœuds choisis, les commentaires
/// choisis et les nœuds qu'ils encadrent.
fn start_items(state: &mut GraphEditorState, at: PointF, graph: &Graph) {
    let mut nodes = state.selection.clone();
    for comment in graph
        .comments
        .iter()
        .filter(|c| state.comments.contains(&c.id))
    {
        let inside = graph
            .nodes
            .iter()
            .filter(|n| within(n.rect(), comment.rect));
        for node in inside {
            if !nodes.contains(&node.id) {
                nodes.push(node.id);
            }
        }
    }
    state.drag = Some(Drag::Items {
        start: state.view.to_world(at),
        by: (0., 0.),
        nodes,
        comments: state.comments.clone(),
    });
}

/// `inner` est-il tout entier dans `outer` ?
fn within(inner: RectF, outer: RectF) -> bool {
    inner.left >= outer.left
        && inner.right <= outer.right
        && inner.top >= outer.top
        && inner.bottom <= outer.bottom
}

/// Un appui sur un port : Alt débranche son fil ; une entrée branchée rend son fil, tenu
/// par sa sortie ; sinon un fil neuf part du port.
fn press_port(
    state: &mut GraphEditorState,
    alt: bool,
    (port, output): (PortRef, bool),
    at: PointF,
    graph: &Graph,
) -> GraphReply {
    let attached = |w: &&GraphWire| match output {
        true => w.from == port,
        false => w.to == port,
    };
    let existing = graph.wires.iter().rev().find(attached).copied();
    if alt {
        return match existing {
            Some(wire) => reply(Some(GraphEvent::Disconnect(wire))),
            None => GraphReply::default(),
        };
    }
    state.drag = Some(match (output, existing) {
        (false, Some(old)) => Drag::Wire {
            anchor: old.from,
            output: true,
            at,
            old: Some(old),
        },
        _ => Drag::Wire {
            anchor: port,
            output,
            at,
            old: None,
        },
    });
    reply(None)
}

fn moved(state: &mut GraphEditorState, at: PointF) -> GraphReply {
    let view = state.view;
    match &mut state.drag {
        Some(Drag::Pan { last }) => {
            let by = (at.x - last.x, at.y - last.y);
            *last = at;
            state.view.pan(by);
            reply(None)
        }
        Some(Drag::Boxed { to, .. }) => {
            *to = at;
            reply(None)
        }
        Some(Drag::Wire { at: end, .. }) => {
            *end = at;
            reply(None)
        }
        Some(Drag::Minimap(map)) => {
            let world = map.to_world(at);
            state.view.center_on(world, state.size);
            reply(None)
        }
        Some(Drag::Items {
            start,
            by,
            nodes,
            comments,
        }) => {
            let world = view.to_world(at);
            *by = (world.x - start.x, world.y - start.y);
            reply(Some(GraphEvent::Move {
                nodes: nodes.clone(),
                comments: comments.clone(),
                by: *by,
                change: FieldChange::Preview,
            }))
        }
        None => GraphReply::default(),
    }
}

fn released(state: &mut GraphEditorState, at: PointF, graph: &Graph) -> GraphReply {
    match state.drag.take() {
        Some(Drag::Items {
            by,
            nodes,
            comments,
            ..
        }) if by != (0., 0.) => reply(Some(GraphEvent::Move {
            nodes,
            comments,
            by,
            change: FieldChange::Commit,
        })),
        Some(Drag::Wire {
            anchor,
            output,
            old,
            ..
        }) => reply(drop_wire(state, (anchor, output), old, at, graph)),
        Some(Drag::Boxed { from, to, add }) => {
            choose_in(state, (from, to), add, graph);
            reply(None)
        }
        Some(_) => reply(None),
        None => GraphReply::default(),
    }
}

/// Le fil tiré depuis `anchor` lâché en `at` : branché sur le port d'en face s'il l'accepte,
/// débranché s'il avait été repris et qu'il tombe ailleurs.
fn drop_wire(
    state: &GraphEditorState,
    (anchor, output): (PortRef, bool),
    old: Option<GraphWire>,
    at: PointF,
    graph: &Graph,
) -> Option<GraphEvent> {
    let target = port_at(graph, &state.view, at)
        .filter(|&(port, out)| out != output && port.node != anchor.node);
    let Some((port, _)) = target else {
        return old.map(GraphEvent::Disconnect);
    };
    let wire = match output {
        true => GraphWire {
            from: anchor,
            to: port,
        },
        false => GraphWire {
            from: port,
            to: anchor,
        },
    };
    let kinds = (
        graph
            .node(wire.from.node)?
            .outputs
            .get(wire.from.port)?
            .kind,
        graph.node(wire.to.node)?.inputs.get(wire.to.port)?.kind,
    );
    if !graph.accepts(kinds.0, kinds.1) {
        return old.map(GraphEvent::Disconnect);
    }
    if Some(wire) == old || (old.is_none() && graph.wires.contains(&wire)) {
        return None;
    }
    let mut removes: Vec<GraphWire> = old.into_iter().collect();
    if !graph.many_per_input {
        let taken = graph
            .wires
            .iter()
            .filter(|w| w.to == wire.to && Some(**w) != old);
        removes.extend(taken.copied());
    }
    removes.retain(|w| *w != wire);
    Some(GraphEvent::Connect { wire, removes })
}

/// Choisit les nœuds que touche le rectangle `from`–`to` (à l'écran).
fn choose_in(state: &mut GraphEditorState, (from, to): (PointF, PointF), add: bool, graph: &Graph) {
    if !add {
        state.selection.clear();
    }
    let a = state.view.to_world(from);
    let b = state.view.to_world(to);
    let area = RectF {
        left: a.x.min(b.x),
        top: a.y.min(b.y),
        right: a.x.max(b.x),
        bottom: a.y.max(b.y),
    };
    for node in &graph.nodes {
        let r = node.rect();
        let touches = r.left <= area.right
            && r.right >= area.left
            && r.top <= area.bottom
            && r.bottom >= area.top;
        if touches && !state.selection.contains(&node.id) {
            state.selection.push(node.id);
        }
    }
}
