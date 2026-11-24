//! État d'une toile de graphe : la vue, les nœuds et les commentaires choisis, le geste
//! en cours (nœuds tirés, fil tiré d'un port, vue glissée, rectangle de choix,
//! mini-carte), et ce que les gestes et les touches demandent à l'application.
use super::{
    model::{Graph, GraphWire, PortRef},
    plane::GraphView,
};
use crate::ui::{
    editor::FieldChange,
    runtime::{KeyCode, KeyPress, PointerEvent},
    scene::{PointF, RectF},
};

/// What the application does after a gesture of a [`GraphEditor`](super::GraphEditor).
/// The editor never changes the graph itself: the application applies the event, keeps
/// its history (one step per event, `Preview` excepted) and rebuilds the view.
#[derive(Clone, Debug, PartialEq)]
pub enum GraphEvent {
    /// Move these nodes and comments by `by` graph units: `Preview` during the drag, one
    /// `Commit` at its end, `Cancel` with Escape (back where they were).
    Move {
        /// The nodes moved.
        nodes: Vec<u64>,
        /// The comments moved (with the nodes inside them).
        comments: Vec<u64>,
        /// How far from where they were.
        by: (f32, f32),
        /// Preview, commit or cancel.
        change: FieldChange,
    },
    /// Add the wire `wire`, after removing the wires `removes` (the wire picked up from
    /// an input, the one that input had already).
    Connect {
        /// The new wire.
        wire: GraphWire,
        /// The wires to remove first.
        removes: Vec<GraphWire>,
    },
    /// Remove this wire (picked up and dropped on nothing, or Alt and a click on it).
    Disconnect(GraphWire),
    /// Remove these nodes (and their wires) and these comments (Delete).
    Delete {
        /// The nodes.
        nodes: Vec<u64>,
        /// The comments.
        comments: Vec<u64>,
    },
    /// Copy these nodes (Ctrl+C); the application keeps them.
    Copy(Vec<u64>),
    /// Paste what was copied, its top left corner at `at` on the plane (Ctrl+V).
    Paste {
        /// Where, on the plane.
        at: PointF,
    },
    /// Duplicate these nodes (Ctrl+D).
    Duplicate(Vec<u64>),
    /// Frame these nodes in a new comment (C) of rectangle `rect`.
    Comment {
        /// Its rectangle on the plane.
        rect: RectF,
        /// The nodes inside.
        nodes: Vec<u64>,
    },
    /// A right click: on the node `node`, or on the plane at `at` (where the
    /// application opens its search of the nodes to create).
    Menu {
        /// Where, on the plane.
        at: PointF,
        /// Where, in the canvas (to place the menu).
        screen: PointF,
        /// The node under the pointer, if any.
        node: Option<u64>,
    },
    /// A double click on the node `0` (open it).
    Open(u64),
    /// A click on a wire: it is chosen (to show its settings); Delete removes it.
    ChooseWire(GraphWire),
}

/// A gesture of a graph editor, to pass to [`GraphEditorState::update`].
#[derive(Clone, Debug, PartialEq)]
pub struct GraphMsg {
    pub(super) size: (f32, f32),
    pub(super) event: PointerEvent,
}

/// What a gesture did: the event for the application, and whether to rebuild the view.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GraphReply {
    /// What the application has to do.
    pub event: Option<GraphEvent>,
    /// Rebuild the view.
    pub rebuild: bool,
}

/// Le geste en cours.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Drag {
    /// Nœuds et commentaires tirés depuis `start` (plan), de `by`.
    Items {
        start: PointF,
        by: (f32, f32),
        nodes: Vec<u64>,
        comments: Vec<u64>,
    },
    /// Un fil tiré : son bout tenu (`anchor`, une sortie si `output`), le pointeur `at`
    /// (écran), et le fil repris d'une entrée.
    Wire {
        anchor: PortRef,
        output: bool,
        at: PointF,
        old: Option<GraphWire>,
    },
    Pan {
        last: PointF,
    },
    Boxed {
        from: PointF,
        to: PointF,
        add: bool,
    },
    /// La mini-carte tenue, telle qu'elle était à l'appui.
    Minimap(super::plane::Minimap),
}

/// State of a [`GraphEditor`](super::GraphEditor), kept by the application: the view,
/// what is chosen, the gesture in progress.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GraphEditorState {
    /// What the editor shows.
    pub view: GraphView,
    pub(super) selection: Vec<u64>,
    pub(super) comments: Vec<u64>,
    pub(super) wire: Option<GraphWire>,
    pub(super) drag: Option<Drag>,
    pub(super) size: (f32, f32),
}

impl GraphEditorState {
    /// The nodes chosen.
    pub fn selection(&self) -> &[u64] {
        &self.selection
    }

    /// The comments chosen.
    pub fn chosen_comments(&self) -> &[u64] {
        &self.comments
    }

    /// Choose these nodes (and no comment, no wire).
    pub fn set_selection(&mut self, nodes: Vec<u64>) {
        self.selection = nodes;
        self.comments.clear();
        self.wire = None;
    }

    /// The wire chosen, if any.
    pub fn chosen_wire(&self) -> Option<GraphWire> {
        self.wire
    }

    /// Choose this wire (and no node), or none.
    pub fn set_chosen_wire(&mut self, wire: Option<GraphWire>) {
        self.wire = wire;
        if wire.is_some() {
            self.selection.clear();
            self.comments.clear();
        }
    }

    /// The size of the canvas at its last paint, in pixels.
    pub fn size(&self) -> (f32, f32) {
        self.size
    }

    /// Whether a gesture is in progress.
    pub fn dragging(&self) -> bool {
        self.drag.is_some()
    }

    /// Show every node and comment of `graph` with a margin, in a canvas of `size` pixels
    /// (the last painted size when `None`).
    pub fn frame_all(&mut self, graph: &Graph, size: Option<(f32, f32)>) {
        let size = size.unwrap_or(self.size);
        if let Some(bounds) = graph.bounds() {
            self.view.frame(bounds, size, 32.);
        }
    }

    /// Apply a gesture of the editor showing `graph`.
    pub fn update(&mut self, message: GraphMsg, graph: &Graph) -> GraphReply {
        self.size = message.size;
        super::gestures::pointer(self, message.event, graph)
    }

    /// A key for the editor (forward the keys of `Application::key`, or of an
    /// `on_key` handler, while it has the focus): Delete removes what is chosen, Ctrl+A
    /// chooses every node, Ctrl+C, Ctrl+V and Ctrl+D copy, paste and duplicate, C frames
    /// the nodes chosen in a comment, F frames them (or everything), Escape cancels a
    /// gesture. Returns `GraphReply::default()` for a key it does not use.
    pub fn key(&mut self, press: KeyPress, graph: &Graph) -> GraphReply {
        if !press.pressed {
            return GraphReply::default();
        }
        if press.key == KeyCode::Escape {
            return self.cancel();
        }
        if self.drag.is_some() {
            return GraphReply::default();
        }
        let chosen = self.selection.clone();
        let delete = matches!(press.key, KeyCode::Delete | KeyCode::Backspace);
        if let (true, false, Some(wire)) = (delete, press.control, self.wire) {
            self.wire = None;
            return reply(Some(GraphEvent::Disconnect(wire)));
        }
        let event = match (press.key, press.control) {
            (KeyCode::Delete | KeyCode::Backspace, false)
                if !chosen.is_empty() || !self.comments.is_empty() =>
            {
                self.selection.clear();
                GraphEvent::Delete {
                    nodes: chosen,
                    comments: std::mem::take(&mut self.comments),
                }
            }
            (KeyCode::A, true) => {
                self.selection = graph.nodes.iter().map(|n| n.id).collect();
                return reply(None);
            }
            (KeyCode::C, true) if !chosen.is_empty() => GraphEvent::Copy(chosen),
            (KeyCode::V, true) => GraphEvent::Paste {
                at: self.view.to_world(PointF {
                    x: self.size.0 / 3.,
                    y: self.size.1 / 3.,
                }),
            },
            (KeyCode::D, true) if !chosen.is_empty() => GraphEvent::Duplicate(chosen),
            (KeyCode::C, false) if !chosen.is_empty() => comment_around(graph, chosen),
            (KeyCode::F | KeyCode::Home, false) => {
                self.frame_chosen(graph);
                return reply(None);
            }
            _ => return GraphReply::default(),
        };
        reply(Some(event))
    }

    /// Cadre les nœuds choisis, ou tout le graphe sans choix.
    fn frame_chosen(&mut self, graph: &Graph) {
        let chosen = graph
            .nodes
            .iter()
            .filter(|n| self.selection.contains(&n.id))
            .map(|n| n.rect())
            .reduce(union);
        match chosen {
            Some(rect) => self.view.frame(rect, self.size, 32.),
            None => self.frame_all(graph, None),
        }
    }

    /// Échap : le geste en cours s'arrête, les nœuds tirés reviennent.
    fn cancel(&mut self) -> GraphReply {
        match self.drag.take() {
            Some(Drag::Items {
                nodes, comments, ..
            }) => reply(Some(GraphEvent::Move {
                nodes,
                comments,
                by: (0., 0.),
                change: FieldChange::Cancel,
            })),
            Some(_) => reply(None),
            None => GraphReply::default(),
        }
    }
}

/// Un commentaire qui encadre les nœuds `nodes`, avec une marge et la place de son titre.
fn comment_around(graph: &Graph, nodes: Vec<u64>) -> GraphEvent {
    let rect = graph
        .nodes
        .iter()
        .filter(|n| nodes.contains(&n.id))
        .map(|n| n.rect())
        .reduce(union)
        .unwrap_or(RectF::new(0., 0., 0., 0.));
    let rect = RectF {
        left: rect.left - 20.,
        top: rect.top - 20. - super::model::HEADER,
        right: rect.right + 20.,
        bottom: rect.bottom + 20.,
    };
    GraphEvent::Comment { rect, nodes }
}

/// Le plus petit rectangle qui tient `a` et `b`.
pub(super) fn union(a: RectF, b: RectF) -> RectF {
    RectF {
        left: a.left.min(b.left),
        top: a.top.min(b.top),
        right: a.right.max(b.right),
        bottom: a.bottom.max(b.bottom),
    }
}

/// Une réponse qui refait la vue.
pub(super) fn reply(event: Option<GraphEvent>) -> GraphReply {
    GraphReply {
        event,
        rebuild: true,
    }
}
