//! Templates `.ui` : balisage HTML analysé (`markup`), réécrit pour les comportements
//! Bootstrap, puis traduit en constructeurs d'`Element` (`generate`).
mod entry;
pub(crate) mod generate;
pub(crate) mod html;
pub(crate) mod markup;
pub use entry::*;
