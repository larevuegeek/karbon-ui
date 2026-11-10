//! Fenêtre Wayland : surface `xdg_toplevel`, taille et états reçus du compositeur,
//! échelle (fractionnaire par `wp_fractional_scale` ou entière), runtime de
//! l'application, renderers (Vulkan par défaut, Cairo en repli) et tâches de fond.
mod access;
mod change;
mod paint;

pub(super) use paint::shared_gpu;
mod surface;
pub(crate) use surface::*;
