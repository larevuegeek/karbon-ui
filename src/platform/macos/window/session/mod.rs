//! Session d'une fenêtre : `NSWindow` et sa vue, runtime UI de l'application, renderer,
//! tâches de fond ; préparation, entrées et présentation de l'image.
mod lifecycle;
mod paint;
pub(crate) use lifecycle::*;
