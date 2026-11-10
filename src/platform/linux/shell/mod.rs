//! Ce que les boucles Wayland (`linux/wayland/`) et X11 (`linux/x11/`) partagent : une
//! seule boucle par thread, réveil depuis les tâches de fond (`eventfd`), clavier
//! xkbcommon (`keyboard.rs` : touches du moteur, raccourcis d'édition, touches mortes,
//! répétition), défilement (`scroll.rs`), composition des méthodes d'entrée
//! (`composition.rs`), barre de titre et bords du moteur (`chrome.rs`) et fenêtre vue du
//! moteur (`session.rs` : application, renderers, tâches, réveils).
pub(super) mod chrome;
pub(super) mod composition;
pub(super) mod facts;
pub(super) mod keyboard;
#[cfg(test)]
pub(super) mod probe;
mod runner;
pub(super) mod scroll;
pub(super) mod session;
pub(in crate::platform::linux) use runner::*;
