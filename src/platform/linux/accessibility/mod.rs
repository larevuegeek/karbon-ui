//! Pont AT-SPI (lecteur d'écran Orca) : l'application s'enregistre sur le bus
//! d'accessibilité (`org.a11y.Bus.GetAddress`, puis `Socket.Embed` auprès du registre)
//! et répond aux appels D-Bus sur ses objets : application, une fenêtre (`frame`) par
//! fenêtre du moteur, et les éléments de l'instantané accessible (`UiRuntime::accessibility`,
//! renouvelé quand sa version change). Interfaces : Accessible, Application, Component,
//! Action, Value, Text, EditableText, et les propriétés D-Bus. Les actions sont rendues au
//! moteur par `UiRuntime::accessibility_action`.
//!
//! Les événements (focus, fenêtre active, texte, valeur) ne sont calculés et émis que si
//! un client les écoute (`Registry.GetRegisteredEvents`, signaux `EventListener…`), comme
//! les notifications de macOS seulement quand un lecteur d'écran est actif.
mod bridge;
mod events;
mod interfaces;
mod properties;
#[cfg(test)]
mod tests;
mod text;
mod tree;
pub(in crate::platform::linux) use bridge::*;
