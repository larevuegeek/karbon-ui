//! DirectWrite layouts shared by measurement and rendering, including font,
//! alignment, line spacing and decorations.
mod attributes;
mod cache;
mod ink;
mod line;
mod rich;
mod styled;
pub(crate) use cache::*;
pub(crate) use ink::Ink;
