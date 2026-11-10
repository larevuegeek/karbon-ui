//! Renderer Core Graphics / Core Text : une scène est dessinée sur une surface bitmap
//! BGRA prémultipliée sRGB (rastérisation CPU), comme le rendu hors écran Windows.
mod cache;
mod canvas;
mod geometry;
mod layers;
mod paint;
mod shared;
#[cfg(test)]
mod tests;
mod text;

pub(super) use crate::platform::shared::{
    linear_points, normalized, placement, reduction, scope_end, single,
};
pub(super) use text::{colored, same_font};
mod renderer;
pub(crate) use renderer::*;
