//! Protocoles Wayland utilisés, transcrits des fichiers XML de wayland et
//! wayland-protocols (rôle de `wayland-scanner`, sans outil ni crate à la compilation) :
//! tables des interfaces d'extension et numéros des requêtes et des événements.
//!
//! Les tables sont complètes pour chaque interface (toutes ses requêtes et tous ses
//! événements, dans l'ordre du protocole) : les numéros en dépendent, même pour les
//! messages que le backend n'envoie pas.
#[allow(dead_code)]
mod core;
#[allow(dead_code)]
mod decoration;
#[allow(dead_code)]
mod scale;
#[allow(dead_code)]
mod text_input;
#[allow(dead_code)]
mod xdg_shell;

pub(super) use self::core::*;
pub(super) use decoration::*;
pub(super) use scale::*;
pub(super) use text_input::*;
pub(super) use xdg_shell::*;
