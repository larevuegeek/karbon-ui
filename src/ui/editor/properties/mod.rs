//! Grille de propriétés (KUI-030) : sections pliables de lignes « libellé, contrôle »,
//! bulle d'aide par ligne, et les contrôles serrés qui y prennent place.
mod controls;
mod grid;
#[cfg(test)]
mod tests;

pub use controls::{checkbox, choice, color_swatch, read_only, text_field};
pub use grid::{Property, PropertyGrid, PropertySection};
