//! Les données d'une toile de graphe, tenues par l'application : des nœuds placés sur un
//! plan, leurs ports d'entrée et de sortie typés, les fils d'une sortie à une entrée, les
//! commentaires (cadres posés derrière des nœuds, qui les emportent quand on les tire),
//! et les règles de branchement. Le sens des nœuds (ce qu'est une matière, un état
//! d'animation) reste à l'application.
use crate::ui::{
    scene::{PointF, RectF},
    theme::Color,
};

/// Hauteur du titre d'un nœud, d'une rangée de ports, marge du bas ; largeur par défaut.
pub(super) const HEADER: f32 = 26.;
pub(super) const ROW: f32 = 20.;
pub(super) const FOOT: f32 = 6.;
/// The width of a node when [`GraphNode::width`] is zero.
pub const NODE_WIDTH: f32 = 160.;

/// A port of a [`GraphNode`]: its name and its kind. A wire joins an output to an input
/// of the same kind, or of kinds the graph declares compatible
/// ([`Graph::compatible`]); kind `0` joins anything.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GraphPort {
    /// Its name, written beside it.
    pub name: String,
    /// Its kind (a number the application gives: « float », « couleur », « état »).
    pub kind: u32,
    /// Its colour; the accent of the palette by default.
    pub color: Option<Color>,
}

impl GraphPort {
    /// A port named `name` of kind `kind`.
    pub fn new(name: impl Into<String>, kind: u32) -> Self {
        Self {
            name: name.into(),
            kind,
            color: None,
        }
    }
}

/// A node of a [`Graph`]: its identity (chosen by the application, unique in the
/// graph), its title, its place on the plane (its top left corner, in graph units, `y`
/// downwards) and its ports.
#[derive(Clone, Debug, PartialEq)]
pub struct GraphNode {
    /// Its identity.
    pub id: u64,
    /// Its title.
    pub title: String,
    /// A second line under the title (« Marche », « 0,3 s »), if any.
    pub subtitle: Option<String>,
    /// Its top left corner.
    pub position: PointF,
    /// Its width; [`NODE_WIDTH`] when zero.
    pub width: f32,
    /// The colour of its title bar; the field colour of the palette by default.
    pub color: Option<Color>,
    /// A mark around it (the state being played, an error), if any.
    pub mark: Option<Color>,
    /// Its inputs, from the top, on its left side.
    pub inputs: Vec<GraphPort>,
    /// Its outputs, from the top, on its right side.
    pub outputs: Vec<GraphPort>,
}

impl GraphNode {
    /// A node `id` titled `title` at `position`, without ports.
    pub fn new(id: u64, title: impl Into<String>, position: PointF) -> Self {
        Self {
            id,
            title: title.into(),
            subtitle: None,
            position,
            width: 0.,
            color: None,
            mark: None,
            inputs: Vec::new(),
            outputs: Vec::new(),
        }
    }

    /// Its rectangle on the plane.
    pub fn rect(&self) -> RectF {
        let width = if self.width > 0. {
            self.width
        } else {
            NODE_WIDTH
        };
        let rows = self.inputs.len().max(self.outputs.len()) as f32;
        let sub = if self.subtitle.is_some() {
            ROW * 0.8
        } else {
            0.
        };
        let height = HEADER + sub + rows * ROW + FOOT;
        RectF::new(self.position.x, self.position.y, width, height)
    }

    /// The place on the plane of its port `port`, an output when `output`.
    pub fn port_point(&self, port: usize, output: bool) -> PointF {
        let r = self.rect();
        let sub = if self.subtitle.is_some() {
            ROW * 0.8
        } else {
            0.
        };
        PointF {
            x: if output { r.right } else { r.left },
            y: r.top + HEADER + sub + ROW * (port as f32 + 0.5),
        }
    }
}

/// A port of a node: the node's identity and the port's rank among its inputs or its
/// outputs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PortRef {
    /// The node.
    pub node: u64,
    /// The rank of the port.
    pub port: usize,
}

/// A wire, from an output to an input.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GraphWire {
    /// The output it leaves.
    pub from: PortRef,
    /// The input it reaches.
    pub to: PortRef,
}

/// A comment: a titled frame behind nodes. Dragging its title moves it and the nodes
/// inside it (a group).
#[derive(Clone, Debug, PartialEq)]
pub struct GraphComment {
    /// Its identity (unique among the comments).
    pub id: u64,
    /// Its rectangle on the plane.
    pub rect: RectF,
    /// Its text, written in its title.
    pub text: String,
    /// Its colour; the accent of the palette by default.
    pub color: Option<Color>,
}

/// What a [`GraphEditor`](super::GraphEditor) shows: nodes, wires, comments, and the
/// kinds of ports that join besides equal ones.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Graph {
    /// The nodes, drawn in this order (the last on top).
    pub nodes: Vec<GraphNode>,
    /// The wires.
    pub wires: Vec<GraphWire>,
    /// The comments, drawn behind the nodes.
    pub comments: Vec<GraphComment>,
    /// Pairs (output kind, input kind) that join although they differ (an integer into a
    /// float).
    pub compatible: Vec<(u32, u32)>,
    /// Whether an input takes several wires; by default an input takes one, and a new
    /// wire replaces the old one ([`GraphEvent::Connect`](super::GraphEvent::Connect)
    /// says which).
    pub many_per_input: bool,
}

impl Graph {
    /// The node `id`.
    pub fn node(&self, id: u64) -> Option<&GraphNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// Whether an output of kind `from` may join an input of kind `to`.
    pub fn accepts(&self, from: u32, to: u32) -> bool {
        from == 0 || to == 0 || from == to || self.compatible.contains(&(from, to))
    }

    /// The two ends of `wire` on the plane, if both nodes and ports exist.
    pub fn wire_ends(&self, wire: &GraphWire) -> Option<(PointF, PointF)> {
        let from = self.node(wire.from.node)?;
        let to = self.node(wire.to.node)?;
        (wire.from.port < from.outputs.len() && wire.to.port < to.inputs.len()).then(|| {
            (
                from.port_point(wire.from.port, true),
                to.port_point(wire.to.port, false),
            )
        })
    }

    /// The rectangle holding every node and comment, if there is any.
    pub fn bounds(&self) -> Option<RectF> {
        let rects = self.nodes.iter().map(GraphNode::rect);
        let rects = rects.chain(self.comments.iter().map(|c| c.rect));
        rects.reduce(|a, b| RectF {
            left: a.left.min(b.left),
            top: a.top.min(b.top),
            right: a.right.max(b.right),
            bottom: a.bottom.max(b.bottom),
        })
    }
}
