//! Frise de temps des éditeurs : une règle, des pistes et leurs clés, une tête de
//! lecture, pour les animations, les cinématiques et le rejeu d'une partie. L'état des
//! gestes (vue, choix, glisser) est dans `state.rs`, l'élément peint dans `view.rs`.
//! Nommée `Sequencer` pour ne pas la confondre avec `ui::animation::timeline`, qui anime
//! l'interface elle-même.
mod state;
#[cfg(test)]
mod tests;
mod view;

pub use state::{KeyRef, SequencerEvent, SequencerMsg, SequencerReply, SequencerState};
pub use view::{Sequencer, SequencerTrack};
