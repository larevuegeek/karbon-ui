//! Tests de contrat du moteur, indépendants de la plateforme : chaque backend
//! (Windows, headless, futurs macOS/Linux) doit les satisfaire. Les outils partagés
//! sont dans `support`.
mod animation;
mod components;
mod drag_drop;
mod layout;
mod overlays;
mod runtime;
mod styling;
mod support;

// Chemin utilisé par le test DirectWrite du backend Windows (`verify_text_fits`).
#[cfg(windows)]
pub(crate) use layout::text_fit;
