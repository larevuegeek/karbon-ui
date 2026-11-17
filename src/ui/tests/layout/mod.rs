//! Contrat de mise en page : lignes, grilles, texte ajusté, défilement (et ses
//! événements) et listes virtualisées.
mod anchoring;
mod basics;
mod containment;
mod incremental;
mod lazy;
mod scroll;
mod scroll_events;
pub(crate) mod text_fit;
mod virtual_list;
