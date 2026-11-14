//! Image resources: an in-memory image cache bounded by the pixel allocations its
//! entries retain, bounded loading and inspection of PNG/JPEG files, and
//! [`ImageLoader`], which loads images for the view in the background.
mod image_loader;
pub use image_loader::{
    EncodedImageInfo, ImageDecoder, ImageFormat, ImageLoadOptions, LoadedImage, inspect_image,
    load_image_file,
};
mod image_cache;
pub use image_cache::*;
mod images;
pub use images::ImageLoader;
