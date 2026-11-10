//! Renderer Cairo / Pango : une scène est dessinée sur une surface image BGRA
//! prémultipliée sRGB (rastérisation CPU), comme le rendu hors écran de macOS. Les
//! fenêtres ne redessinent que les zones modifiées (`shared::damage`).
mod cache;
mod canvas;
mod geometry;
pub(super) mod gpu;
mod layers;
mod paint;
mod renderer;
#[cfg(test)]
mod tests;
mod text;
pub(crate) use renderer::*;
