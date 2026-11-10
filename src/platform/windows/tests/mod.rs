//! Tests du backend sur des fenêtres cachées, sans interaction avec le bureau utilisateur.
//! Un fichier par thème : la suite minimale qu'un backend natif doit reproduire.
mod change;
mod images;
mod input;
mod offscreen;
mod presentation;
mod support;
mod tasks;
mod timers;
mod window;
use support::*;
