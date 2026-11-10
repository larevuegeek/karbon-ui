//! Pont NSAccessibility (VoiceOver) : un `NSAccessibilityElement` par élément de l'arbre
//! accessible du moteur, créé à la première interrogation d'un lecteur d'écran.
//!
//! Rôles, libellés, valeurs et aides sont lus dans l'instantané du moteur
//! (`UiRuntime::accessibility`, renouvelé quand sa version change) ; les actions
//! (appuyer, incrémenter, saisir une valeur, focaliser) lui sont renvoyées par
//! `UiRuntime::accessibility_action`. Lien avec la fenêtre : `window/accessible.rs`.
mod node;

pub(super) use node::Node;
mod element;
pub(crate) use element::*;
