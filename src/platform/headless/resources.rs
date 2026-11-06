//! Images du backend headless : aucune décompression, des substituts de la bonne taille.
use crate::{
    ImageError,
    application::Resources,
    resources::{ImageDecoder, ImageLoadOptions, inspect_image},
    ui::scene::Image,
};
use std::sync::Arc;

/// Gris opaque des substituts (BGRA prémultiplié).
const PLACEHOLDER: [u8; 4] = [0x80, 0x80, 0x80, 0xff];

fn placeholder(width: u32, height: u32) -> crate::Result<Image> {
    Image::new(
        width,
        height,
        PLACEHOLDER.repeat(width as usize * height as usize),
    )
}

/// Services du backend headless : les en-têtes PNG/JPEG sont validés et lus, puis
/// l'image est remplacée par un aplat gris de mêmes dimensions (déterministe, sans
/// décodeur natif).
#[derive(Default)]
pub(super) struct HeadlessResources;
impl Resources for HeadlessResources {
    fn decode_image(&self, bytes: &[u8]) -> crate::Result<Arc<Image>> {
        let info = inspect_image(bytes)?;
        if info.width == 0 || info.height == 0 || info.width > 4096 || info.height > 4096 {
            return Err(ImageError::TooLarge {
                reason: "decoded images are limited to 4096 pixels per edge",
            }
            .into());
        }
        Ok(Arc::new(placeholder(info.width, info.height)?))
    }
    fn image_decoder(&self) -> Option<Arc<dyn ImageDecoder>> {
        Some(Arc::new(HeadlessDecoder))
    }
}

/// Décodeur transmissible aux tâches : mêmes limites et même taille de vignette que le
/// décodeur Windows.
struct HeadlessDecoder;
impl ImageDecoder for HeadlessDecoder {
    fn decode(&self, bytes: &[u8], options: ImageLoadOptions) -> crate::Result<Image> {
        options.validate()?;
        if bytes.len() > options.max_file_bytes {
            return Err(ImageError::TooLarge {
                reason: "encoded image larger than the file size limit",
            }
            .into());
        }
        let info = inspect_image(bytes)?;
        options.check_dimensions(info.width, info.height)?;
        let ratio = (options.thumbnail_edge as f64 / info.width.max(info.height) as f64).min(1.);
        placeholder(
            (info.width as f64 * ratio).round().max(1.) as u32,
            (info.height as f64 * ratio).round().max(1.) as u32,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn png_header(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
        bytes.extend(width.to_be_bytes());
        bytes.extend(height.to_be_bytes());
        bytes.extend([8, 6, 0, 0, 0, 0, 0, 0, 0]);
        bytes
    }
    #[test]
    fn placeholders_keep_the_encoded_size_and_thumbnail_limits() {
        let image = HeadlessResources.decode_image(&png_header(40, 10)).unwrap();
        assert_eq!((image.width, image.height), (40, 10));
        assert_eq!(&image.pixels[..4], &PLACEHOLDER);
        assert!(HeadlessResources.decode_image(b"not an image").is_err());
        assert!(
            HeadlessResources
                .decode_image(&png_header(5000, 1))
                .is_err()
        );
        let decoder = HeadlessResources.image_decoder().unwrap();
        let thumbnail = decoder
            .decode(&png_header(800, 400), ImageLoadOptions::default())
            .unwrap();
        assert_eq!((thumbnail.width, thumbnail.height), (192, 96));
    }
}
