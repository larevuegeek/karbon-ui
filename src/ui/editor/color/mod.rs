//! Sélecteur de couleur des éditeurs : un carré saturation et clarté, une barre de
//! teinte, une barre d'opacité au besoin, et la valeur en hexadécimal à taper. Les
//! conversions (teinte, saturation, valeur ; hexadécimal) sont dans `convert.rs`, l'état
//! des gestes dans `state.rs`, l'élément dans `view.rs`.
mod convert;
mod state;
#[cfg(test)]
mod tests;
mod view;

pub use convert::{Hsv, hex_of, parse_hex};
pub use state::{ColorEvent, ColorMsg, ColorPickerState, ColorReply};
pub use view::ColorPicker;
