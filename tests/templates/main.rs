//! Templates compilés (`ui!`, `ui_file!`), indépendants de la plateforme : syntaxe et
//! blocs, composants et slots, balisage HTML et feuille par défaut, contrôles natifs
//! pilotés par le runtime, SVG en ligne, dossier d'icônes (`icons_dir!`), et diagnostics
//! du compilateur vus par une crate utilisatrice. Les templates inclus sont dans `ui/`,
//! les icônes dans `icons/`, les feuilles à côté de leur module.
//!
//! `cargo test --test templates`
mod aria;
mod compilation;
mod components;
mod exit;
mod html;
mod icon_folder;
mod pointer;
mod runtime;
mod support;
mod svg;
mod syntax;
mod textarea;
mod variable_paint;
