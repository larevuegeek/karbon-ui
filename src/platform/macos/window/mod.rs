//! Boucle d'événements AppKit et fenêtres natives.
//!
//! La boucle est écrite à la main (`nextEventMatchingMask:untilDate:…`) : l'attente se
//! termine au prochain réveil demandé par le runtime (animations, infobulles), ce qui
//! tient lieu de minuterie, ou à un événement « application » posté par les tâches de
//! fond (réveil thread-safe). Entre deux événements, la boucle livre les tâches, exécute
//! les commandes de fenêtre et peint les fenêtres invalidées.
//!
//! Les méthodes Objective-C (`view.rs`) retrouvent leur session par identité ; aucun
//! emprunt des sessions n'est tenu pendant un appel AppKit qui peut ré-entrer (fermeture,
//! zoom, déplacement de la fenêtre).
pub(super) mod accessible;
mod change;
mod dialog;
mod event_loop;
mod events;
mod menu;
mod session;
mod text;
mod view;
pub(crate) use event_loop::*;
