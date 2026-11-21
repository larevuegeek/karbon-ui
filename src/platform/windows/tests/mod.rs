//! Tests du backend sur des fenêtres cachées, sans interaction avec le bureau utilisateur.
//! Un fichier par thème : la suite minimale qu'un backend natif doit reproduire.
mod change;
mod close;
mod images;
mod input;
mod offscreen;
mod presentation;
mod shared_surface;
mod support;
mod system_keys;
mod tasks;
mod timers;
mod tray;
mod window;
use support::*;
