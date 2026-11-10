//! Filtres CSS sur le GPU, miroir de `graphics/filters.rs` : une chaîne de filtres devient
//! une suite de passages (`filters.metal`), chacun lisant la sortie du précédent dans des
//! textures RGBA 16 bits flottants (pas d'arrondi entre deux filtres, comme les plans
//! flottants du chemin Core Graphics).
//!
//! - flou : écart type en pixels ; au-delà de 6, calculé sur une image réduite d'un
//!   facteur k = ⌊σ/3⌋ ≤ 8 (moyenne des blocs, flou de σ/k, agrandissement bilinéaire),
//!   exactement comme `blur` côté Core Graphics ; flou gaussien séparable en deux passages ;
//! - ombre portée : alpha décalé d'un nombre entier de pixels et teinté, flouté, puis le
//!   contenu posé dessus ;
//! - matrices de couleur consécutives : un seul passage, bornées après chacune.
mod kernels;
pub(super) use kernels::{FORMAT, Pipelines, encode};
mod passes;
pub(crate) use passes::*;
