//! Décodage d'images par gdk-pixbuf et export PNG par Cairo, en BGRA prémultiplié sRGB.
use super::{
    bitmap::{self, Bitmap, ImageSurface, Surface},
    ffi::{
        cairo::{self, FILTER_GOOD, Matrix, Pattern, cairo_surface_write_to_png_stream},
        failure,
        pixbuf::Pixbuf,
    },
};
use crate::{
    ImageError,
    resources::{ImageDecoder, ImageLoadOptions, inspect_image},
    ui::scene::Image,
};
use std::{ffi::c_void, sync::Arc};

/// Ressources passées à `Application::load` : décodage sur le thread UI et décodeur
/// de fond sans état natif.
pub(super) struct Imaging;
impl crate::application::Resources for Imaging {
    fn decode_image(&self, bytes: &[u8]) -> crate::Result<Arc<Image>> {
        Ok(Arc::new(decode(bytes)?))
    }
    fn image_decoder(&self) -> Option<Arc<dyn ImageDecoder>> {
        Some(Arc::new(BackgroundDecoder))
    }
}
/// Décodeur des tâches de fond : chaque appel crée ses propres objets gdk-pixbuf et Cairo.
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
        options.check_dimensions(source.width as u32, source.height as u32)?;
        let edge = source.width.max(source.height) as f64;
        let ratio = (f64::from(options.thumbnail_edge) / edge).min(1.);
        let width = (source.width as f64 * ratio).round().max(1.) as usize;
        let height = (source.height as f64 * ratio).round().max(1.) as usize;
        let image = premultiplied(&source)?;
        if (width, height) == (source.width, source.height) {
            return Ok(image);
        }
        resize(&Arc::new(image), width, height)
    }
}
/// Image entière, limitée à 4096 pixels par côté comme sous Windows. Pour un PNG ou un
/// JPEG, les dimensions de l'en-tête sont vérifiées avant de décoder les pixels : un
/// fichier qui en annonce des milliers ne fait pas allouer gdk-pixbuf. Les autres formats
/// (GIF, WebP… selon les modules installés) sont décodés par gdk-pixbuf puis vérifiés.
pub(super) fn decode(bytes: &[u8]) -> crate::Result<Image> {
    const EDGE: u32 = 4096;
    let too_large = || ImageError::TooLarge {
        reason: "decoded images are limited to 4096 pixels per edge",
    };
    if let Ok(info) = inspect_image(bytes)
        && (info.width > EDGE || info.height > EDGE)
    {
        return Err(too_large().into());
    }
    let source = open(bytes)?;
    if source.width > EDGE as usize || source.height > EDGE as usize {
        return Err(too_large().into());
    }
    premultiplied(&source)
}
/// Première image du flux ; des données refusées par gdk-pixbuf deviennent `Decode`.
fn open(bytes: &[u8]) -> crate::Result<Pixbuf> {
    if bytes.is_empty() || u32::try_from(bytes.len()).is_err() {
        return Err(ImageError::Decode.into());
    }
    Pixbuf::decode(bytes).ok_or_else(|| ImageError::Decode.into())
}
/// Pixels RGB(A) non prémultipliés → BGRA prémultiplié (arrondi au plus proche).
fn premultiplied(source: &Pixbuf) -> crate::Result<Image> {
    let (width, height, channels) = (source.width, source.height, source.channels);
    let bytes = source.bytes();
    let mut pixels = Vec::with_capacity(width * height * 4);
    for y in 0..height {
        let row = &bytes[y * source.stride..y * source.stride + width * channels];
        for p in row.chunks_exact(channels) {
            let alpha = if channels == 4 { p[3] } else { 255 };
            let scale = |c: u8| ((u16::from(c) * u16::from(alpha) + 127) / 255) as u8;
            pixels.extend_from_slice(&[scale(p[2]), scale(p[1]), scale(p[0]), alpha]);
        }
    }
    Image::new(width as u32, height as u32, pixels)
}
/// `image` redimensionnée à `width` × `height` (filtre de Cairo pour les réductions,
/// sur les pixels prémultipliés).
pub(super) fn resize(image: &Arc<Image>, width: usize, height: usize) -> crate::Result<Image> {
    let source = ImageSurface::new(image)?;
    let target = Bitmap::new(width, height)?;
    draw_scaled(&target, &source, FILTER_GOOD)?;
    Image::new(width as u32, height as u32, target.into_pixels())
}
/// Dessine `source` étirée sur toute la surface `target`.
pub(super) fn draw_scaled(
    target: &Bitmap,
    source: &impl Surface,
    filter: std::ffi::c_int,
) -> crate::Result<()> {
    let (sw, sh) = source.size();
    let context = target.context()?;
    let pattern = Pattern::surface(source)?;
    pattern.set_filter(filter);
    pattern.set_extend(cairo::EXTEND_PAD);
    // Matrice du motif : des pixels de la cible vers ceux de la source.
    pattern.set_matrix(Matrix::scaling(
        sw as f64 / target.width as f64,
        sh as f64 / target.height as f64,
    ));
    context.set_source(&pattern);
    context.set_operator(cairo::OPERATOR_SOURCE);
    context.paint();
    if context.status() != cairo::STATUS_SUCCESS {
        return Err(failure("cairo_paint"));
    }
    Ok(())
}
/// Encode `image` en PNG (alpha conservé) et l'écrit dans `path`.
pub(super) fn write_png(image: &Image, path: &std::path::Path) -> crate::Result<()> {
    unsafe extern "C" fn collect(closure: *mut c_void, data: *const u8, length: u32) -> i32 {
        // SAFETY : `closure` est le `Vec` passé ci-dessous ; `data` contient `length`
        // octets valides pendant l'appel.
        unsafe {
            (*closure.cast::<Vec<u8>>())
                .extend_from_slice(std::slice::from_raw_parts(data, length as usize));
        }
        cairo::STATUS_SUCCESS
    }
    let mut bytes = Vec::new();
    let status = bitmap::with_image(image, |surface| {
        // SAFETY : surface valide pendant l'appel ; `bytes` survit à l'appel, seul
        // lecteur de `closure`.
        unsafe { cairo_surface_write_to_png_stream(surface, collect, (&raw mut bytes).cast()) }
    })?;
    if status != cairo::STATUS_SUCCESS {
        return Err(failure("cairo_surface_write_to_png_stream"));
    }
    std::fs::write(path, bytes)?;
    Ok(())
}
