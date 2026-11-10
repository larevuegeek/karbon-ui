//! Objets COM via `#[implement]` : identité canonique par nœud, accès concurrent aux snapshots.
//!
//! `#[implement]` génère un QueryInterface figé ; pour n'exposer que les patterns réellement
//! pris en charge, chaque profil de capacités a sa propre classe (`classes`). Les combinaisons
//! rares retombent sur `FullProvider`, dont les méthodes valident toujours l'état du nœud.
//! `common` lit le nœud courant pour les propriétés et la navigation communes.
mod classes;
mod common;

pub(super) use classes::create;
mod capabilities;
pub(crate) use capabilities::*;
