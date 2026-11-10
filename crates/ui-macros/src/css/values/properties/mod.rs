//! Traduction d'une propriété CSS (valeur déjà résolue) en propriétés du moteur.
//!
//! Chaque famille (`spacing`, `boxes`, `position`, `flex_grid`, `borders`, `paint`, `text`,
//! `transforms`, `scroll`) reconnaît ses propriétés et passe les autres à la suivante ;
//! une propriété qu'aucune ne reconnaît est refusée.
mod borders;
mod boxes;
mod flex_grid;
mod paint;
mod position;
mod scroll;
mod spacing;
mod text;
mod transforms;
pub(super) use spacing::NAMES as SPACING;
mod dispatch;
pub use dispatch::*;
