//! Utilitaires Tailwind v4 : chaque classe devient des déclarations CSS, écrites comme
//! les produit Tailwind (variables `--tw-*` comprises), puis résolues par élément dans
//! [`super::compile_set`]. Les longueurs sont converties en px (1rem = 16 px).
mod borders;
mod colors;
mod composed;
mod effects;
mod layout;
mod motion;
mod spacing;
mod statics;
mod transforms;
mod typography;
mod values;
pub(super) use values::{fmt, fmt_precise, px_length};
#[cfg(test)]
pub(super) use values::{rem, spaced};
mod dispatch;
pub use dispatch::*;
