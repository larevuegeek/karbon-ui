//! Interaction de bout en bout par le harnais headless, indépendante de la plateforme :
//! menus et listes de choix écrits en template (placement, clavier, fermeture, focus,
//! accessibilité), champ à suggestions, et aperçus hors écran (inspection, survol).
//!
//! `cargo test --test interaction`
mod combobox;
mod menus;
mod previews;

use karbon_ui::ui::prelude::*;
css_file!(mod css, "menus.css");
