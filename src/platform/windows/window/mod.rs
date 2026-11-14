//! Boucle d'événements Win32 et fenêtres natives : ouverture, commandes différées,
//! erreur de la boucle et réveil du thread UI.
pub(crate) mod actions;
pub(crate) mod change;
mod create;
mod dialog;
pub(crate) mod dwm;
pub(crate) mod icon;
mod menu_theme;
mod session;
pub(crate) mod shadow;
pub(crate) mod startup;
pub(crate) mod state;
pub(crate) mod tasks;
pub(crate) mod timer;
pub(crate) mod tray;

pub(crate) use session::Session;
mod event_loop;
pub(crate) use event_loop::*;
