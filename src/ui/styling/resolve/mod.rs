//! Résolution des styles : cascade des blocs dont les sélecteurs correspondent, héritage
//! et application du style calculé à chaque élément de l'arbre.
mod cascade;
mod environment;
mod matching;
mod node;
mod partial;
mod scope;

pub use cascade::resolve;
pub(crate) use cascade::resolve_inherited;
pub use environment::{AnimatedSizes, ElementColors, ElementSizes, StyleChanges, StyleEnvironment};
pub(crate) use matching::{PointerFacts, pointer_facts};
pub(crate) use node::local;
pub use partial::Partial;
