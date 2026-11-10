//! TextPattern des champs de saisie (une ou plusieurs lignes) : plages COM et géométrie
//! issue du même layout que le rendu.
//! Les plages elles-mêmes (`ITextRangeProvider`) sont dans `range`.
mod pattern;
mod range;
#[cfg(test)]
mod tests;
pub(crate) use pattern::*;
