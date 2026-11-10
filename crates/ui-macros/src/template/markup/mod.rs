//! Lecture du balisage HTML des templates : texte libre, balises, attributs,
//! entités, expressions Rust entre accolades et blocs `{#if}` / `{#each}`.
//!
//! Deux sources produisent la même suite de « pièces » : les jetons d'un `ui!`
//! (dont on reconstitue le texte et les espaces grâce aux positions des jetons) et
//! le texte brut d'un fichier `.ui`. Les accolades deviennent des blocs de jetons
//! Rust ; tout le reste est lu caractère par caractère.
//!
//! Organisation : `location` (positions et fichiers pour les diagnostics), `ast`
//! (nœuds produits), `pieces` (découpage des deux sources), `entities` (entités
//! HTML), `parser` (balises, attributs, texte) et `blocks` (`{#if}`, `{#each}`,
//! `{@…}`).
mod ast;
mod blocks;
mod entities;
mod location;
mod parser;
mod pieces;
pub use ast::{Attr, Each, Element, Node, Value};
pub use location::{Loc, register_file};
mod reader;
pub use reader::*;
