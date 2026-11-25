//! Peinture gardée des toiles de l'éditeur (KUI-056) : un graphe de 500 nœuds et 1 000
//! fils, une frise et des courbes à côté d'un compteur qui change. Les scènes sont celles
//! de la peinture refaite (`set_incremental(false)`), octet pour octet ; les toiles ne
//! sont pas repeintes quand seul le compteur change ; leurs gestes reçoivent toujours la
//! taille de la toile, même après une image recopiée.
use super::{
    Curve, CurveEditor, CurveEditorState, CurveKey, CurveMsg, Graph, GraphEditor, GraphEditorState,
    GraphMsg, GraphNode, GraphPort, GraphWire, PortRef, Sequencer, SequencerMsg, SequencerState,
    SequencerTrack,
};
use crate::{
    platform::headless::{HeadlessApp, HeadlessOptions, RawModifiers},
    ui::{
        element::{Element, Length},
        prelude::{Application, ViewContext, column, text},
        rendering::filter::Filters,
        runtime::{PointerButton, PointerPhase},
        scene::{PointF, RectF, Scene, signature::Signature},
    },
};

#[derive(Clone, Debug)]
enum Msg {
    Graph(GraphMsg),
    Sequencer(SequencerMsg),
    Curves(CurveMsg),
}

struct Desk {
    graph: Graph,
    graph_state: GraphEditorState,
    tracks: Vec<SequencerTrack>,
    sequencer: SequencerState,
    curves: Vec<Curve>,
    curve_state: CurveEditorState,
    counter: u32,
}

fn desk() -> Desk {
    let mut graph = Graph::default();
    for i in 0..500u64 {
        let at = PointF {
            x: (i % 25) as f32 * 220.,
            y: (i / 25) as f32 * 140.,
        };
        let mut node = GraphNode::new(i, format!("Nœud {i}"), at);
        node.inputs = vec![GraphPort::new("a", 1), GraphPort::new("b", 1)];
        node.outputs = vec![GraphPort::new("sortie", 1)];
        graph.nodes.push(node);
    }
    for i in 0..1000u64 {
        let (from, to) = (i % 500, (i % 500 + 1 + i % 7) % 500);
        graph.wires.push(GraphWire {
            from: PortRef {
                node: from,
                port: 0,
            },
            to: PortRef {
                node: to,
                port: (i % 2) as usize,
            },
        });
    }
    let tracks = (0..40)
        .map(|t| SequencerTrack {
            name: format!("Piste {t}"),
            keys: (0..200)
                .map(|k| k as f32 * 0.25 + t as f32 * 0.01)
                .collect(),
            ..Default::default()
        })
        .collect();
    let curves = vec![Curve {
        name: "Position X".into(),
        color: None,
        keys: (0..50)
            .map(|k| CurveKey::new(k as f32, (k as f32 * 0.7).sin()))
            .collect(),
    }];
    Desk {
        graph,
        graph_state: GraphEditorState::default(),
        tracks,
        sequencer: SequencerState::default(),
        curves,
        curve_state: CurveEditorState::default(),
        counter: 0,
    }
}

fn boxed(id: &'static str, height: f32, inner: Element) -> Element {
    column(id, [inner])
        .width(Length::Fill(1.))
        .height(Length::Px(height))
}

impl Application for Desk {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        let graph = GraphEditor::new(1, &self.graph, &self.graph_state, Msg::Graph).element();
        let sequencer =
            Sequencer::new(2, self.tracks.clone(), 1.5, &self.sequencer, Msg::Sequencer).element();
        let curves = CurveEditor::new(3, &self.curves, &self.curve_state, Msg::Curves).element();
        column(
            "page",
            [
                text("counter", format!("{}", self.counter)),
                boxed("graph", 300., graph),
                boxed("sequencer", 160., sequencer),
                boxed("curves", 160., curves),
            ],
        )
    }
    fn update(&mut self, message: Msg) -> bool {
        match message {
            Msg::Graph(m) => self.graph_state.update(m, &self.graph).rebuild,
            Msg::Sequencer(m) => self.sequencer.update(m).rebuild,
            Msg::Curves(m) => self.curve_state.update(m, &self.curves).rebuild,
        }
    }
}

fn digest(scene: &Scene) -> u64 {
    let nothing = RectF::new(0., 0., 0., 0.);
    Signature::new(&scene.commands, nothing, Filters::new(&[]), 0.).digest()
}

/// Empreintes des scènes, toiles recopiées, et l'état des toiles après un appui sur
/// chacune (qui lit la taille de la toile) fait après une image recopiée.
fn frames(incremental: bool) -> (Vec<u64>, u64, String) {
    let options = HeadlessOptions {
        width: 900.,
        height: 700.,
        ..Default::default()
    };
    let mut app = HeadlessApp::with_options(desk(), options).unwrap();
    app.runtime_mut().set_incremental(incremental);
    app.prepare().unwrap();
    let mut out = vec![digest(app.scene())];
    app.update_app(|desk| desk.counter += 1).unwrap();
    out.push(digest(app.scene()));
    for id in ["graph", "curves", "sequencer"] {
        let r = app.bounds(id).unwrap();
        let at = PointF {
            x: r.left + 40.,
            y: r.top + 60.,
        };
        let press = Some(PointerButton::Primary);
        let modifiers = RawModifiers::default();
        // Le pointeur est déjà sur la toile quand le compteur change : l'image suivante
        // la recopie, et l'appui qui suit lit la taille notée sans peinture.
        app.raw_pointer(PointerPhase::Move, None, at, modifiers)
            .unwrap();
        app.update_app(|desk| desk.counter += 1).unwrap();
        app.raw_pointer(PointerPhase::Down, press, at, modifiers)
            .unwrap();
        app.raw_pointer(PointerPhase::Up, press, at, modifiers)
            .unwrap();
        app.update_app(|desk| desk.counter += 1).unwrap();
        out.push(digest(app.scene()));
    }
    let desk = app.app();
    let states = format!(
        "{:?} {:?} {:?}",
        desk.graph_state, desk.curve_state, desk.sequencer
    );
    (out, app.runtime().kept_paints_reused(), states)
}

#[test]
fn kept_canvases_give_the_same_scenes_and_sizes_as_painting_again() {
    let (kept, reused, states) = frames(true);
    let (painted, none, full_states) = frames(false);
    assert_eq!(kept, painted);
    assert_eq!(
        states, full_states,
        "les gestes lisent la taille de la toile"
    );
    // Les trois toiles recopiées quand seul le compteur change ; jamais sans mémoires.
    assert!(reused >= 3, "toiles recopiées : {reused}");
    assert_eq!(none, 0);
    assert_ne!(kept[0], kept[1]);
}

/// Temps de peinture de la scène quand seul le compteur change, toiles recopiées ou
/// repeintes. À la demande :
/// `cargo test --release --lib kept_canvases_cost -- --ignored --nocapture`.
#[test]
#[ignore]
fn kept_canvases_cost() {
    for incremental in [false, true] {
        let options = HeadlessOptions {
            width: 1600.,
            height: 1000.,
            ..Default::default()
        };
        let mut app = HeadlessApp::with_options(desk(), options).unwrap();
        app.update_app(|desk| desk.graph_state.frame_all(&desk.graph, Some((1600., 300.))))
            .unwrap();
        app.runtime_mut().set_incremental(incremental);
        app.prepare().unwrap();
        let before = app.runtime().stats().paint_time;
        for _ in 0..200 {
            app.update_app(|desk| desk.counter += 1).unwrap();
        }
        let after = app.runtime().stats().paint_time;
        println!(
            "Peinture de la scène (graphe de 500 nœuds tout montré, frise, courbes), toiles {} : {:.3} ms par image",
            if incremental {
                "recopiées"
            } else {
                "repeintes"
            },
            (after - before).as_secs_f64() * 1000. / 200.
        );
    }
}
