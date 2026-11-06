//! Static data standing in for the Svelte stores and Tauri `invoke` calls. The first
//! screen follows the state of the Tauri reference captures (track, queue, mixes,
//! sidebar); the sections below the fold use the PNG covers taken over from the former
//! native example (see `assets/SOURCES.md`).
//!
//! Rangées par zone de l'écran ; les templates les lisent toutes par `data::NOM`.
mod header;
mod home;
mod player;
mod sidebar;

pub use header::*;
pub use home::*;
pub use player::*;
pub use sidebar::*;
