//! Component shadows are rasterized only when their shape/style changes.
//! Three separable box passes approximate the Gaussian; painting reuses the image.
//! Shadows without blur (rings such as `0 0 0 1px`, one-side borders such as
//! `inset 0 -1px 0`) are drawn as strokes and fills instead: an image of the whole
//! element for a one-pixel line cost megabytes of pixels and GPU memory.
mod raster;
mod shadows;
#[cfg(test)]
mod tests;
mod vector;
pub use shadows::*;
