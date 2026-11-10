//! Backend Direct2D / DirectWrite sur texture D3D11 BGRA.
//!
//! GPU = D3D_DRIVER_TYPE_HARDWARE, logiciel = D3D_DRIVER_TYPE_WARP.
//! Les interfaces COM sont typées par la crate officielle `windows` (métadonnées du SDK).
//! Deux cibles : une texture relue par le CPU (rendu hors écran, chemin layered) ou le
//! back buffer d'une chaîne d'échange DirectComposition (présentation sans relecture).
mod backdrop;
mod canvas;
mod convert;
mod device;
mod effects;
mod filters;
mod formats;
mod frame;
mod geometry;
mod gradients;
mod images;
mod measure;
mod radials;
mod readback;
mod renderer;
mod shapes;
mod svg;
mod swapchain;
mod target;
#[cfg(test)]
mod tests;
mod typography;
pub(crate) use renderer::*;
