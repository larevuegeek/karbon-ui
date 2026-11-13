//! Session d'une fenêtre : `NSWindow` et sa vue, runtime UI de l'application, renderer,
//! tâches de fond ; préparation, entrées et présentation de l'image.
mod frame_link;
mod lifecycle;
mod pacing;
mod paint;
pub(crate) use lifecycle::*;
