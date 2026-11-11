//! Boucle d'événements Wayland : connexion au compositeur, fenêtres `xdg_toplevel`
//! présentées par Vulkan (ou dessinées par Cairo dans des tampons `wl_shm` en repli),
//! pointeur, clavier, méthode d'entrée, presse-papiers, minuteries et réveil des tâches de
//! fond.
//!
//! La boucle attend (`poll`) la connexion et un `eventfd` écrit par les tâches de fond,
//! jusqu'au prochain réveil demandé (runtime, répétition d'une touche). Les événements du
//! compositeur sont mis en file par un répartiteur unique (`display.rs`) puis traités ici,
//! sans emprunt tenu pendant un appel à libwayland.
mod clipboard;
mod display;
mod event_loop;
mod events;
mod input;
mod presentation;
mod protocol;
mod seat;
mod shm;
#[cfg(test)]
mod tests;
mod text_input;
mod window;
pub(crate) use event_loop::*;
