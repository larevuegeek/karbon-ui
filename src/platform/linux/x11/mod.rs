//! Boucle d'événements X11 (XCB, chargé au lancement) : fenêtres de premier niveau
//! dessinées par Vulkan (chaîne d'échange XCB) ou par Cairo (`xcb_put_image`), minuteries,
//! réveil des tâches de fond et accessibilité AT-SPI. Choisie quand il n'y a pas de
//! compositeur Wayland (`WAYLAND_DISPLAY` absent), ou par `KARBON_BACKEND=x11` (sous
//! XWayland par exemple).
//!
//! La boucle attend (`poll`) la connexion X, un `eventfd` écrit par les tâches de fond et
//! le bus d'accessibilité, jusqu'au prochain réveil demandé. Contrairement à Wayland, X11
//! n'a pas de rappel de trame : une fenêtre affichée est peinte dès qu'elle a changé, au
//! rythme de l'écran sous Vulkan (« fifo »).
mod clipboard;
mod connection;
mod event_loop;
mod events;
mod ime;
mod input;
#[cfg(test)]
mod tests;
mod window;
mod xinput;
pub(crate) use event_loop::*;
