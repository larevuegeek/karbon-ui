//! Feuilles de frameworks, indépendantes de la plateforme : utilitaires Tailwind dans les
//! templates, Bootstrap 5.3 compilé en mode `framework` (variables, composants,
//! contrôles dessinés en CSS, démo pilotée comme un utilisateur) et cascade générique
//! du mode framework.
//!
//! `cargo test --test frameworks`
mod bootstrap;
mod framework_mode;
mod support;
mod tailwind;
