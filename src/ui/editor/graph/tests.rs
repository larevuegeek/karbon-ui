//! Toile de graphe, dans la fenêtre simulée, visée par la place des ports et des nœuds :
//! brancher, refuser un port d'une autre sorte, reprendre un fil et le débrancher ou le
//! poser ailleurs, déplacer (un seul `Commit`, Échap), annuler par l'historique de
//! l'application, choisir au rectangle, supprimer, dupliquer, copier, coller, encadrer
//! d'un commentaire qui emporte ses nœuds, menu du clic droit, double clic, mini-carte,
//! ordre de peinture, et le coût de 500 nœuds et 1 000 fils.
use super::*;
use crate::{
    platform::headless::HeadlessApp,
    ui::{
        editor::{
            FieldChange,
            testing::{CONTROL, NONE, SHIFT, click, key, pointer, window},
        },
        element::{Element, ElementId},
        prelude::{Application, ViewContext},
        runtime::{KeyCode, KeyPress, PointerButton, PointerPhase},
        scene::{DrawCommand, PointF},
    },
};

const FLOAT: u32 = 1;
const COLOR: u32 = 2;

struct Workshop {
    graph: Graph,
    state: GraphEditorState,
    log: Vec<GraphEvent>,
    /// L'historique : le graphe avant chaque étape.
    undo: Vec<Graph>,
    before: Option<Graph>,
}

fn node(id: u64, x: f32, y: f32, inputs: &[u32], outputs: &[u32]) -> GraphNode {
    let mut node = GraphNode::new(id, format!("Nœud {id}"), PointF { x, y });
    node.inputs = inputs
        .iter()
        .map(|&k| GraphPort::new("entrée", k))
        .collect();
    node.outputs = outputs
        .iter()
        .map(|&k| GraphPort::new("sortie", k))
        .collect();
    node
}

fn three_nodes() -> Graph {
    Graph {
        nodes: vec![
            node(1, 0., 0., &[], &[FLOAT, COLOR]),
            node(2, 300., 0., &[FLOAT, FLOAT], &[FLOAT]),
            node(3, 300., 160., &[COLOR], &[]),
        ],
        ..Graph::default()
    }
}

impl Workshop {
    fn apply(&mut self, event: &GraphEvent) {
        if let GraphEvent::Move {
            nodes,
            comments,
            by,
            change,
        } = event
        {
            let base = self
                .before
                .get_or_insert_with(|| self.graph.clone())
                .clone();
            if *change != FieldChange::Preview && *change != FieldChange::Cancel {
                self.undo.push(base.clone());
            }
            self.graph = base;
            for n in self
                .graph
                .nodes
                .iter_mut()
                .filter(|n| nodes.contains(&n.id))
            {
                n.position.x += by.0;
                n.position.y += by.1;
            }
            for c in self
                .graph
                .comments
                .iter_mut()
                .filter(|c| comments.contains(&c.id))
            {
                c.rect = crate::ui::scene::RectF::new(
                    c.rect.left + by.0,
                    c.rect.top + by.1,
                    c.rect.right - c.rect.left,
                    c.rect.bottom - c.rect.top,
                );
            }
            if *change != FieldChange::Preview {
                self.before = None;
            }
            return;
        }
        self.undo.push(self.graph.clone());
        match event {
            GraphEvent::Connect { wire, removes } => {
                self.graph.wires.retain(|w| !removes.contains(w));
                self.graph.wires.push(*wire);
            }
            GraphEvent::Disconnect(wire) => self.graph.wires.retain(|w| w != wire),
            GraphEvent::Delete { nodes, comments } => {
                self.graph.nodes.retain(|n| !nodes.contains(&n.id));
                let gone = |p: PortRef| nodes.contains(&p.node);
                self.graph.wires.retain(|w| !gone(w.from) && !gone(w.to));
                self.graph.comments.retain(|c| !comments.contains(&c.id));
            }
            GraphEvent::Comment { rect, .. } => self.graph.comments.push(GraphComment {
                id: 9,
                rect: *rect,
                text: "Groupe".into(),
                color: None,
            }),
            _ => drop(self.undo.pop()),
        }
    }
}

impl Application for Workshop {
    type Message = GraphMsg;
    fn view(&self, _: &ViewContext) -> Element {
        GraphEditor::new(1, &self.graph, &self.state, |m| m).element()
    }
    fn update(&mut self, message: GraphMsg) -> bool {
        let reply = self.state.update(message, &self.graph);
        if let Some(event) = reply.event {
            self.apply(&event);
            self.log.push(event);
        }
        reply.rebuild
    }
    fn key(&mut self, press: KeyPress) -> bool {
        if press.pressed && press.control && press.key == KeyCode::Z {
            if let Some(graph) = self.undo.pop() {
                self.graph = graph;
            }
            return true;
        }
        let reply = self.state.key(press, &self.graph);
        if let Some(event) = reply.event {
            self.apply(&event);
            self.log.push(event);
        }
        reply.rebuild
    }
}

fn workshop(graph: Graph) -> HeadlessApp<Workshop> {
    let app = Workshop {
        graph,
        state: GraphEditorState::default(),
        log: Vec::new(),
        undo: Vec::new(),
        before: None,
    };
    let mut w = window(app, 800., 500.);
    w.prepare().unwrap();
    w
}

/// La place dans la fenêtre d'un point du plan.
fn screen(w: &HeadlessApp<Workshop>, world: PointF) -> PointF {
    let area = w.bounds(ElementId::named("karbon-graph").item(1)).unwrap();
    let p = w.app().state.view.to_screen(world);
    PointF {
        x: area.left + p.x,
        y: area.top + p.y,
    }
}

fn port(w: &HeadlessApp<Workshop>, id: u64, rank: usize, output: bool) -> PointF {
    let node = w.app().graph.node(id).unwrap().port_point(rank, output);
    screen(w, node)
}

fn title(w: &HeadlessApp<Workshop>, id: u64) -> PointF {
    let r = w.app().graph.node(id).unwrap().rect();
    screen(
        w,
        PointF {
            x: r.left + 40.,
            y: r.top + 10.,
        },
    )
}

fn drag(w: &mut HeadlessApp<Workshop>, from: PointF, to: PointF) {
    let left = Some(PointerButton::Primary);
    pointer(w, PointerPhase::Down, left, from, NONE);
    let middle = PointF {
        x: (from.x + to.x) / 2.,
        y: (from.y + to.y) / 2.,
    };
    pointer(w, PointerPhase::Move, None, middle, NONE);
    pointer(w, PointerPhase::Move, None, to, NONE);
    pointer(w, PointerPhase::Up, left, to, NONE);
}

fn wire(from: (u64, usize), to: (u64, usize)) -> GraphWire {
    GraphWire {
        from: PortRef {
            node: from.0,
            port: from.1,
        },
        to: PortRef {
            node: to.0,
            port: to.1,
        },
    }
}

#[test]
fn a_wire_drawn_from_an_output_to_an_input_of_its_kind_connects_them() {
    let mut w = workshop(three_nodes());
    let (from, to) = (port(&w, 1, 0, true), port(&w, 2, 1, false));
    drag(&mut w, from, to);
    assert_eq!(w.app().graph.wires, [wire((1, 0), (2, 1))]);
    // Témoin : la sortie « float » ne va pas dans l'entrée « couleur ».
    let (from, to) = (port(&w, 1, 0, true), port(&w, 3, 0, false));
    drag(&mut w, from, to);
    assert_eq!(w.app().graph.wires.len(), 1, "sortes différentes : refusé");
    // Une sortie « couleur » y va ; et de l'entrée vers la sortie aussi.
    let (from, to) = (port(&w, 3, 0, false), port(&w, 1, 1, true));
    drag(&mut w, from, to);
    assert!(w.app().graph.wires.contains(&wire((1, 1), (3, 0))));
    // Les sortes déclarées compatibles se branchent.
    let mut graph = three_nodes();
    graph.compatible.push((FLOAT, COLOR));
    let mut w = workshop(graph);
    let (from, to) = (port(&w, 1, 0, true), port(&w, 3, 0, false));
    drag(&mut w, from, to);
    assert_eq!(w.app().graph.wires, [wire((1, 0), (3, 0))]);
}

#[test]
fn a_wire_picked_up_from_its_input_moves_or_disconnects_and_undo_brings_it_back() {
    let mut graph = three_nodes();
    graph.wires = vec![wire((1, 0), (2, 0))];
    let mut w = workshop(graph);
    // Reprise de l'entrée 0, posée sur l'entrée 1 : un seul `Connect` qui retire l'ancien.
    let (from, to) = (port(&w, 2, 0, false), port(&w, 2, 1, false));
    drag(&mut w, from, to);
    assert_eq!(
        w.app().log.last(),
        Some(&GraphEvent::Connect {
            wire: wire((1, 0), (2, 1)),
            removes: vec![wire((1, 0), (2, 0))]
        })
    );
    assert_eq!(w.app().graph.wires, [wire((1, 0), (2, 1))]);
    // Reprise puis lâchée dans le vide : débranchée.
    let (from, to) = (
        port(&w, 2, 1, false),
        screen(&w, PointF { x: 150., y: 300. }),
    );
    drag(&mut w, from, to);
    assert!(w.app().graph.wires.is_empty());
    // L'historique de l'application la rend, puis rend la première place.
    key(&mut w, KeyCode::Z, CONTROL);
    assert_eq!(w.app().graph.wires, [wire((1, 0), (2, 1))]);
    key(&mut w, KeyCode::Z, CONTROL);
    assert_eq!(w.app().graph.wires, [wire((1, 0), (2, 0))]);
    // Une entrée prise : un fil neuf remplace le sien.
    let (from, to) = (port(&w, 2, 0, true), port(&w, 2, 0, false));
    drag(&mut w, from, to);
    assert_eq!(
        w.app().graph.wires,
        [wire((1, 0), (2, 0))],
        "pas sur son propre nœud"
    );
}

#[test]
fn a_node_drags_in_one_step_and_escape_puts_it_back() {
    let mut w = workshop(three_nodes());
    let start = w.app().graph.node(2).unwrap().position;
    let from = title(&w, 2);
    let to = PointF {
        x: from.x + 50.,
        y: from.y + 20.,
    };
    drag(&mut w, from, to);
    let commits = w
        .app()
        .log
        .iter()
        .filter(|e| {
            matches!(
                e,
                GraphEvent::Move {
                    change: FieldChange::Commit,
                    ..
                }
            )
        })
        .count();
    assert_eq!(commits, 1, "une étape");
    let moved = w.app().graph.node(2).unwrap().position;
    assert!((moved.x - start.x - 50.).abs() < 0.01 && (moved.y - start.y - 20.).abs() < 0.01);
    key(&mut w, KeyCode::Z, CONTROL);
    assert_eq!(w.app().graph.node(2).unwrap().position, start, "annulé");
    // Échap pendant le glisser : il revient, sans étape.
    let from = title(&w, 2);
    pointer(
        &mut w,
        PointerPhase::Down,
        Some(PointerButton::Primary),
        from,
        NONE,
    );
    pointer(
        &mut w,
        PointerPhase::Move,
        None,
        PointF {
            x: from.x + 80.,
            y: from.y,
        },
        NONE,
    );
    key(&mut w, KeyCode::Escape, NONE);
    assert_eq!(w.app().graph.node(2).unwrap().position, start);
    assert!(w.app().undo.is_empty());
}

#[test]
fn a_rectangle_chooses_nodes_and_delete_removes_them_with_their_wires() {
    let mut graph = three_nodes();
    graph.wires = vec![wire((1, 0), (2, 0)), wire((1, 1), (3, 0))];
    let mut w = workshop(graph);
    let a = screen(&w, PointF { x: 280., y: -20. });
    let b = screen(&w, PointF { x: 500., y: 120. });
    drag(&mut w, a, b);
    assert_eq!(w.app().state.selection(), [2]);
    let third = title(&w, 3);
    click(&mut w, third, SHIFT);
    assert_eq!(w.app().state.selection(), [2, 3], "Maj ajoute");
    key(&mut w, KeyCode::Delete, NONE);
    assert_eq!(w.app().graph.nodes.len(), 1);
    assert!(
        w.app().graph.wires.is_empty(),
        "leurs fils partent avec eux"
    );
    key(&mut w, KeyCode::Z, CONTROL);
    assert_eq!(w.app().graph.nodes.len(), 3);
}

#[test]
fn copy_paste_duplicate_and_a_comment_that_carries_its_nodes() {
    let mut w = workshop(three_nodes());
    let second = title(&w, 2);
    click(&mut w, second, NONE);
    let third = title(&w, 3);
    click(&mut w, third, SHIFT);
    key(&mut w, KeyCode::C, CONTROL);
    key(&mut w, KeyCode::V, CONTROL);
    key(&mut w, KeyCode::D, CONTROL);
    let log = &w.app().log;
    assert!(log.contains(&GraphEvent::Copy(vec![2, 3])));
    assert!(log.iter().any(|e| matches!(e, GraphEvent::Paste { .. })));
    assert!(log.contains(&GraphEvent::Duplicate(vec![2, 3])));
    key(&mut w, KeyCode::C, NONE);
    let rect = w.app().graph.comments[0].rect;
    for id in [2, 3] {
        let r = w.app().graph.node(id).unwrap().rect();
        assert!(
            r.left > rect.left
                && r.right < rect.right
                && r.top > rect.top
                && r.bottom < rect.bottom
        );
    }
    // Le titre du commentaire tiré emporte ses deux nœuds, pas le premier.
    let first = w.app().graph.node(1).unwrap().position;
    let second = w.app().graph.node(2).unwrap().position;
    let grip = screen(
        &w,
        PointF {
            x: rect.left + 30.,
            y: rect.top + 8.,
        },
    );
    drag(
        &mut w,
        grip,
        PointF {
            x: grip.x + 30.,
            y: grip.y + 40.,
        },
    );
    assert_eq!(w.app().graph.node(1).unwrap().position, first);
    let after = w.app().graph.node(2).unwrap().position;
    assert!((after.x - second.x - 30.).abs() < 0.01 && (after.y - second.y - 40.).abs() < 0.01);
}

#[test]
fn a_right_click_asks_for_the_menu_and_a_double_click_opens_a_node() {
    let mut w = workshop(three_nodes());
    let empty = screen(&w, PointF { x: 150., y: 300. });
    pointer(
        &mut w,
        PointerPhase::Down,
        Some(PointerButton::Secondary),
        empty,
        NONE,
    );
    match w.app().log.last() {
        Some(GraphEvent::Menu { at, node: None, .. }) => {
            assert!(
                (at.x - 150.).abs() < 0.5 && (at.y - 300.).abs() < 0.5,
                "{at:?}"
            );
        }
        other => panic!("{other:?}"),
    }
    pointer(
        &mut w,
        PointerPhase::Up,
        Some(PointerButton::Secondary),
        empty,
        NONE,
    );
    let on = title(&w, 3);
    click(&mut w, on, NONE);
    click(&mut w, on, NONE);
    assert_eq!(w.app().log.last(), Some(&GraphEvent::Open(3)));
}

#[test]
fn the_minimap_moves_the_view_and_wires_are_painted_under_the_nodes() {
    let mut graph = three_nodes();
    graph.wires = vec![wire((1, 0), (2, 0))];
    graph.nodes.push(node(4, 2000., 1500., &[], &[]));
    let mut w = workshop(graph);
    let area = w.bounds(ElementId::named("karbon-graph").item(1)).unwrap();
    // Le coin de la mini-carte : le nœud lointain vient au milieu de la vue.
    let corner = PointF {
        x: area.right - 15.,
        y: area.bottom - 15.,
    };
    click(&mut w, corner, NONE);
    let view = w.app().state.view;
    let middle = view.to_world(PointF { x: 400., y: 250. });
    assert!(middle.x > 1000. && middle.y > 700., "{middle:?}");
    // Ordre de peinture : le fond, puis les fils, puis les nœuds.
    let mut w = workshop(three_nodes());
    let (from, to) = (port(&w, 1, 0, true), port(&w, 2, 0, false));
    drag(&mut w, from, to);
    let field = crate::ui::editor::EditorPalette::DARK.field;
    let commands = &w.scene().commands;
    let first_node = commands
        .iter()
        .position(|c| matches!(c, DrawCommand::Fill { color, .. } if *color == field))
        .unwrap();
    let accent = crate::ui::editor::EditorPalette::DARK.accent;
    let last_wire = commands.iter().rposition(|c| matches!(c, DrawCommand::Line { color, .. } if color.red == accent.red && color.green == accent.green)).unwrap();
    assert!(
        last_wire < first_node,
        "fil {last_wire}, premier nœud {first_node}"
    );
}

#[test]
fn five_hundred_nodes_and_a_thousand_wires_paint_quickly() {
    let mut graph = Graph::default();
    for i in 0..500u64 {
        let (x, y) = ((i % 25) as f32 * 220., (i / 25) as f32 * 120.);
        graph
            .nodes
            .push(node(i, x, y, &[FLOAT, FLOAT], &[FLOAT, FLOAT]));
    }
    for i in 0..1000u64 {
        let from = i % 500;
        let to = (from + 1 + i % 7) % 500;
        graph
            .wires
            .push(wire((from, (i % 2) as usize), (to, ((i / 2) % 2) as usize)));
    }
    let mut w = workshop(graph);
    w.update_app(|app| {
        let graph = app.graph.clone();
        app.state.frame_all(&graph, Some((800., 500.)));
    })
    .unwrap();
    let started = std::time::Instant::now();
    for step in 0..10 {
        w.update_app(|app| app.state.view.pan((step as f32, 0.)))
            .unwrap();
    }
    let each = started.elapsed().as_secs_f64() * 100.;
    eprintln!(
        "500 nœuds et 1 000 fils, tout montré : {each:.2} ms par image (construction et peinture)"
    );
    let budget = if cfg!(debug_assertions) { 200. } else { 16. };
    assert!(each < budget, "{each} ms");
}

#[test]
fn a_click_on_a_wire_chooses_it_and_delete_removes_it() {
    let mut graph = three_nodes();
    graph.wires = vec![wire((1, 0), (2, 0))];
    let mut w = workshop(graph);
    let (a, b) = (port(&w, 1, 0, true), port(&w, 2, 0, false));
    let middle = PointF {
        x: (a.x + b.x) / 2.,
        y: (a.y + b.y) / 2.,
    };
    click(&mut w, middle, NONE);
    assert_eq!(
        w.app().log.last(),
        Some(&GraphEvent::ChooseWire(wire((1, 0), (2, 0))))
    );
    assert_eq!(w.app().state.chosen_wire(), Some(wire((1, 0), (2, 0))));
    key(&mut w, KeyCode::Delete, NONE);
    assert!(w.app().graph.wires.is_empty());
    // Témoin : loin du fil, rien n'est choisi.
    let far = PointF {
        x: middle.x,
        y: middle.y + 60.,
    };
    click(&mut w, far, NONE);
    assert_eq!(w.app().state.chosen_wire(), None);
}
