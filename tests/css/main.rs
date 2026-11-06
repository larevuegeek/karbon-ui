//! Contrat CSS du moteur, indépendant de la plateforme : sélecteurs et états, valeurs
//! (couleurs, unités, variables), texte, mise en page, requêtes média et de conteneur,
//! peinture, animations. Chaque module compile sa propre feuille (`<module>.css`, à côté
//! de lui) et passe par `UiRuntime`, `styling::resolve` + `layout::arrange` ou le
//! harnais headless : tout backend (Windows, macOS, Linux) doit les satisfaire.
//!
//! `cargo test --test css`
mod layout;
mod motion;
mod paint;
mod responsive;
mod selectors;
mod support;
mod text;
mod values;
