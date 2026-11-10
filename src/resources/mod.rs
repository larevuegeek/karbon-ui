//! Image resources: an in-memory image cache bounded by the pixel allocations its
//! entries retain, plus bounded loading and inspection of PNG/JPEG files.
mod image_loader;
pub use image_loader::{
    EncodedImageInfo, ImageDecoder, ImageFormat, ImageLoadOptions, LoadedImage, inspect_image,
    load_image_file,
};
mod image_cache;
pub use image_cache::*;
