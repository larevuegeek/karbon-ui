//! `<svg>` en ligne compilé : formes, chemins, transformations et attributs de
//! présentation résolus ici, puis émis comme un `SvgImage` statique. Les arcs, les
//! quadratiques et les primitives (cercle, rectangle arrondi…) deviennent des cubiques
//! absolues ; les transformations des groupes sont appliquées aux points.
//!
//! `builder` parcourt l'arbre, `paint` lit les attributs de présentation, `path` la
//! géométrie (chemins et formes), `transform` les matrices et `scanner` les nombres.
mod builder;
mod document;
mod paint;
mod path;
mod scanner;
mod transform;
pub use document::*;
