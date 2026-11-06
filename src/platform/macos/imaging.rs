//! Décodage d'images et export PNG par ImageIO, en BGRA prémultiplié sRGB.
use super::{
    bitmap::{self, Bitmap},
    ffi::{cf_string, failure, required},
};
use crate::{
    ImageError,
    resources::{ImageDecoder, ImageLoadOptions, inspect_image},
    ui::scene::Image,
};
use objc2_core_foundation::{CFData, CFMutableData, CFRetained, CGFloat, CGPoint, CGRect, CGSize};
use objc2_core_graphics::{CGContext, CGImage, CGInterpolationQuality};
use objc2_image_io::{CGImageDestination, CGImageSource};

/// Ressources passées à `Application::load` : décodage sur le thread UI et décodeur
/// de fond sans état natif.
pub(super) struct Imaging;
impl crate::application::Resources for Imaging {
    fn decode_image(&self, bytes: &[u8]) -> crate::Result<std::sync::Arc<Image>> {
        Ok(std::sync::Arc::new(decode(bytes)?))
    }
    fn image_decoder(&self) -> Option<std::sync::Arc<dyn ImageDecoder>> {
        Some(std::sync::Arc::new(BackgroundDecoder))
    }
}
/// Décodeur des tâches de fond : chaque appel crée ses propres objets ImageIO.
struct BackgroundDecoder;
impl ImageDecoder for BackgroundDecoder {
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
        let source = open(bytes)?;
        let (width, height) = (
            CGImage::width(Some(&source)),
            CGImage::height(Some(&source)),
        );
        options.check_dimensions(width as u32, height as u32)?;
        let ratio = (f64::from(options.thumbnail_edge) / width.max(height) as f64).min(1.);
        let target_width = (width as f64 * ratio).round().max(1.) as usize;
        let target_height = (height as f64 * ratio).round().max(1.) as usize;
        pixels(&source, target_width, target_height)
    }
}
/// Image entière, limitée à 4096 pixels par côté comme sous Windows.
pub(super) fn decode(bytes: &[u8]) -> crate::Result<Image> {
    let source = open(bytes)?;
    let (width, height) = (
        CGImage::width(Some(&source)),
        CGImage::height(Some(&source)),
    );
    if width == 0 || height == 0 || width > 4096 || height > 4096 {
        return Err(ImageError::TooLarge {
            reason: "decoded images are limited to 4096 pixels per edge",
        }
        .into());
    }
    pixels(&source, width, height)
}
/// Première image du flux ; des données refusées par ImageIO deviennent `Decode`.
fn open(bytes: &[u8]) -> crate::Result<CFRetained<CGImage>> {
    if bytes.is_empty() || u32::try_from(bytes.len()).is_err() {
        return Err(ImageError::Decode.into());
    }
    let data = CFData::from_bytes(bytes);
    // SAFETY : données CF possédées ; aucune option.
    let source = unsafe { CGImageSource::with_data(&data, None) }.ok_or(ImageError::Decode)?;
    // SAFETY : source valide ; index 0 vérifié par ImageIO (None si absent).
    unsafe { source.image_at_index(0, None) }.ok_or_else(|| ImageError::Decode.into())
}
/// Dessine `source` à `width` × `height` dans une surface BGRA prémultipliée sRGB.
fn pixels(source: &CGImage, width: usize, height: usize) -> crate::Result<Image> {
    let surface = Bitmap::new(width, height)?;
    CGContext::set_interpolation_quality(Some(&surface.context), CGInterpolationQuality::High);
    CGContext::draw_image(
        Some(&surface.context),
        CGRect::new(
            CGPoint::new(0., 0.),
            CGSize::new(width as CGFloat, height as CGFloat),
        ),
        Some(source),
    );
    Image::new(width as u32, height as u32, surface.into_pixels())
}
/// Encode `image` en PNG (alpha conservé) et l'écrit dans `path`.
pub(super) fn write_png(image: &Image, path: &std::path::Path) -> crate::Result<()> {
    let source =
        bitmap::image_from_bgra(image.width as usize, image.height as usize, &image.pixels)?;
    let data = required(CFMutableData::new(None, 0), "CFDataCreateMutable")?;
    let kind = cf_string("public.png");
    // SAFETY : données mutables possédées, type UTI valide, une image.
    let destination = unsafe { CGImageDestination::with_data(&data, &kind, 1, None) }
        .ok_or_else(|| failure("CGImageDestinationCreateWithData"))?;
    // SAFETY : destination et image valides ; aucune propriété.
    unsafe { destination.add_image(&source, None) };
    // SAFETY : une image ajoutée ci-dessus.
    if !unsafe { destination.finalize() } {
        return Err(failure("CGImageDestinationFinalize"));
    }
    drop(destination);
    std::fs::write(path, data.to_vec())?;
    Ok(())
}
