//! Arbre virtualisé (KUI-029) : les nœuds de l'application aplatis en lignes visibles,
//! dont seules celles à l'écran sont construites ; pli, sélection par clé, clavier,
//! double-clic et glisser-déposer pour changer de parent.
mod keys;
#[cfg(test)]
mod payload_tests;
mod rows;
mod state;
#[cfg(test)]
mod tests;
mod view;

pub use rows::{TreeRow, TreeSource};
pub use state::{DropPlace, TreeDrop, TreeEvent, TreeMsg, TreeReply, TreeState};
pub use view::{TreeItem, TreeView};
