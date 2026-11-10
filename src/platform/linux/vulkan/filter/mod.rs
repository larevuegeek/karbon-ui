//! Filtres CSS sur le GPU, miroir des filtres CPU du chemin Cairo
//! (`platform/shared/filters.rs`) : une chaîne de filtres devient une suite de passages
//! (`shaders/filter.frag`), chacun lisant la sortie du précédent dans des textures RGBA
//! 16 bits flottants (pas d'arrondi entre deux filtres, comme les plans flottants du CPU).
//! Copie adaptée de `macos/metal/filter/`.
//!
//! - flou : au-delà de 6 pixels d'écart type, calculé sur une image réduite d'un facteur
//!   k = ⌊σ/3⌋ ≤ 8 (moyenne des blocs, flou de σ/k, agrandissement bilinéaire) ; trois
//!   flous en boîte par axe, des largeurs de `boxes`, exactement comme le CPU (Metal
//!   calcule une vraie gaussienne, comme Core Graphics) ;
//! - ombre portée : alpha décalé d'un nombre entier de pixels et teinté, flouté, puis le
//!   contenu posé dessus ;
//! - matrices de couleur consécutives : un seul passage, bornées après chacune.
mod pipeline;
pub(super) use pipeline::{FORMAT, Filters as FilterPipeline, Record, record};
mod passes;
pub(crate) use passes::*;
