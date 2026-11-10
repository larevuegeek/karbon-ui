//! Métriques de texte déterministes, indépendantes des polices installées.
mod lines;
mod measurer;
mod rich;
#[cfg(test)]
mod tests;

pub(super) use lines::formatted_lines;
pub(super) use rich::{align_lines, rich_lines};
mod metrics;
pub use metrics::*;
