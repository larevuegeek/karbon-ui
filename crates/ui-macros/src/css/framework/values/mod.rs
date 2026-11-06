//! Valeurs des feuilles de frameworks : normalisation (`rem` → px), substitution des
//! variables connues à la compilation, découpage en jetons pour l'exécution et
//! classement d'une déclaration (statique, dynamique ou ignorée).
mod classify;
mod longhands;
mod normalize;
mod tokens;

pub use classify::{Kind, classify};
pub use longhands::{coverage, logical};
pub use normalize::{references, rem_to_px, substitute};
pub use tokens::{token_slice, tokenize};
