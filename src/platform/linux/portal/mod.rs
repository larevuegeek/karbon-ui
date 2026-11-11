//! Portails XDG du bureau (`org.freedesktop.portal.Desktop`) : dialogues de fichiers
//! (`file_chooser.rs`) et thème (`appearance.rs`), communs à Wayland et X11 et indépendants
//! du toolkit du bureau (GNOME, KDE, wlroots…).
//!
//! Un dialogue reste ouvert le temps que l'utilisateur choisisse : il est demandé et
//! attendu sur un thread à lui, avec sa propre connexion au bus de session, pour que la
//! boucle de fenêtres continue de peindre. Le choix revient par un canal propre à la
//! fenêtre (`Dialogs`), que la boucle vide avec les résultats des tâches de fond après
//! avoir été réveillée (`eventfd` de `shell::Wake`).
mod appearance;
mod file_chooser;
#[cfg(test)]
mod tests;

pub(in crate::platform::linux) use appearance::{prefers_dark, watch};
mod dialogs;
pub(in crate::platform::linux) use dialogs::*;
