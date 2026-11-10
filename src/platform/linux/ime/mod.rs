//! Méthodes d'entrée par D-Bus pour les fenêtres X11 (sous Wayland, le compositeur les
//! relaie par `zwp_text_input_v3`) : IBus (`ibus.rs`), le défaut de GNOME et de la plupart
//! des distributions. Le client ne dépend pas du système de fenêtres : la boucle lui passe
//! les touches, le focus du champ et la position du caret, et applique ses demandes.
mod ibus;

pub(super) use ibus::Ibus;
mod request;
pub(crate) use request::*;
