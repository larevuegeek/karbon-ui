//! Toile de graphe de nœuds (KUI-040) : nœuds aux ports typés, fils d'une sortie à une
//! entrée selon les règles de l'application, commentaires qui emportent leurs nœuds,
//! choix au rectangle, copier, coller, dupliquer, menu du clic droit pour créer, mini-carte,
//! un pas d'historique par geste. Le sens des nœuds reste à l'application.
mod gestures;
mod hit;
mod model;
mod plane;
mod state;
#[cfg(test)]
mod tests;
mod view;

pub use model::{Graph, GraphComment, GraphNode, GraphPort, GraphWire, NODE_WIDTH, PortRef};
pub use plane::GraphView;
pub use state::{GraphEditorState, GraphEvent, GraphMsg, GraphReply};
pub use view::GraphEditor;
