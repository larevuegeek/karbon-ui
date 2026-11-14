//! Décodage d’images via GDI+, fourni par Windows (API plate de windows-sys).
use crate::ImageError;
use crate::resources::{ImageDecoder, ImageLoadOptions, inspect_image};
use crate::ui::scene::Image;
use std::ptr::{null, null_mut};
use windows::{
    Win32::{System::Com::IStream, UI::Shell::SHCreateMemStream},
    core::Interface,
};
use windows_sys::{
    Win32::Graphics::GdiPlus::{
        self, BitmapData, GdipBitmapLockBits, GdipBitmapUnlockBits, GdipCreateBitmapFromScan0,
        GdipDeleteGraphics, GdipDisposeImage, GdipDrawImageRectI, GdipGetImageGraphicsContext,
        GdipGetImageHeight, GdipGetImageWidth, GdipGraphicsClear, GdipLoadImageFromStream,
        GdipSaveImageToFile, GdipSetInterpolationMode, GdiplusShutdown, GdiplusStartup,
        GdiplusStartupInput, GpBitmap, GpGraphics, GpImage, ImageLockModeRead,
        InterpolationModeHighQualityBicubic, Status,
    },
    core::GUID,
};

// PixelFormat32bppPARGB : macro de GdiPlusPixelFormats.h, absente des métadonnées.
const PIXEL_FORMAT_32BPP_PARGB: i32 = 0x000E_200B;
// CLSID de l'encodeur PNG intégré (ImageCodecInfo::Clsid, stable depuis GDI+ 1.0).
const PNG_ENCODER: GUID = GUID::from_u128(0x557cf406_1a04_11d3_9a73_0000f81ef32e);

fn check(status: Status, operation: &'static str) -> crate::Result<()> {
    if status != GdiPlus::Ok {
        Err(crate::Error::Platform {
            operation,
            code: status_hresult(status),
        })
    } else {
        Ok(())
    }
}
/// HRESULT le plus proche d'un statut GDI+ (les statuts GDI+ ne sont pas des HRESULT).
fn status_hresult(status: Status) -> i32 {
    const E_NOTIMPL: u32 = 0x8000_4001;
    const E_ABORT: u32 = 0x8000_4004;
    const E_FAIL: u32 = 0x8000_4005;
    const E_ACCESSDENIED: u32 = 0x8007_0005;
    const E_OUTOFMEMORY: u32 = 0x8007_000E;
    const E_INVALIDARG: u32 = 0x8007_0057;
    const FILE_NOT_FOUND: u32 = 0x8007_0002;
    (match status {
        GdiPlus::Win32Error => {
            let code = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
            return super::ffi::hresult_from_win32(code as u32);
        }
        GdiPlus::InvalidParameter => E_INVALIDARG,
        GdiPlus::OutOfMemory => E_OUTOFMEMORY,
        GdiPlus::NotImplemented => E_NOTIMPL,
        GdiPlus::Aborted => E_ABORT,
        GdiPlus::AccessDenied => E_ACCESSDENIED,
        GdiPlus::FileNotFound => FILE_NOT_FOUND,
        _ => E_FAIL,
    }) as i32
}
pub(super) struct DrawingEngine(usize);
// Un GpBitmap est un GpImage (héritage C++) : la même instance sert aux deux API.
struct NativeImage(*mut GpImage);
impl NativeImage {
    fn bitmap(&self) -> *mut GpBitmap {
        self.0.cast()
    }
}
impl Drop for NativeImage {
    fn drop(&mut self) {
        // SAFETY : image possédée, créée par GDI+ et libérée une seule fois.
        unsafe {
            GdipDisposeImage(self.0);
        }
    }
}
struct NativeGraphics(*mut GpGraphics);
impl Drop for NativeGraphics {
    fn drop(&mut self) {
        // SAFETY : contexte possédé, créé par GDI+ et libéré une seule fois.
        unsafe {
            GdipDeleteGraphics(self.0);
        }
    }
}
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
        let _apartment = super::ffi::ComApartment::initialize()?;
        let engine = DrawingEngine::initialize()?;
        engine.thumbnail(bytes, options)
    }
}
impl crate::application::Resources for DrawingEngine {
    fn decode_image(&self, bytes: &[u8]) -> crate::Result<std::sync::Arc<Image>> {
        Ok(std::sync::Arc::new(self.decode(bytes)?))
    }
    fn image_decoder(&self) -> Option<std::sync::Arc<dyn ImageDecoder>> {
        Some(std::sync::Arc::new(BackgroundDecoder))
    }
}
impl Drop for DrawingEngine {
    fn drop(&mut self) {
        // SAFETY : jeton obtenu par GdiplusStartup, rendu une seule fois.
        unsafe { GdiplusShutdown(self.0) };
    }
}
impl DrawingEngine {
    pub(super) fn initialize() -> crate::Result<Self> {
        let input = GdiplusStartupInput {
            GdiplusVersion: 1,
            DebugEventCallback: 0,
            SuppressBackgroundThread: 0,
            SuppressExternalCodecs: 0,
        };
        let mut token = 0;
        // SAFETY : entrée locale valide ; sortie nulle autorisée sans thread supprimé.
        check(
            unsafe { GdiplusStartup(&mut token, &input, null_mut()) },
            "GdiplusStartup",
        )?;
        Ok(Self(token))
    }
    pub(super) fn decode(&self, bytes: &[u8]) -> crate::Result<Image> {
        let (_stream, image, width, height) = Self::open(bytes)?;
        if width == 0 || height == 0 || width > 4096 || height > 4096 {
            return Err(ImageError::TooLarge {
                reason: "decoded images are limited to 4096 pixels per edge",
            }
            .into());
        }
        Self::pixels(&image, width, height)
    }
    /// Image réduite (jamais agrandie) à `edge` pixels sur son grand côté, en qualité
    /// bicubique : icônes de la fenêtre et de la zone de notification.
    pub(super) fn scaled(&self, bytes: &[u8], edge: u32) -> crate::Result<Image> {
        self.thumbnail(
            bytes,
            ImageLoadOptions {
                thumbnail_edge: edge.clamp(1, 2048),
                ..ImageLoadOptions::default()
            },
        )
    }
    fn thumbnail(&self, bytes: &[u8], options: ImageLoadOptions) -> crate::Result<Image> {
        let (_stream, source, width, height) = Self::open(bytes)?;
        options.check_dimensions(width, height)?;
        let ratio = (options.thumbnail_edge as f64 / width.max(height) as f64).min(1.);
        let target_width = (width as f64 * ratio).round().max(1.) as u32;
        let target_height = (height as f64 * ratio).round().max(1.) as u32;
        let mut bitmap = null_mut();
        // SAFETY : pixels nuls et pas nul : GDI+ alloue lui-même le bitmap.
        check(
            unsafe {
                GdipCreateBitmapFromScan0(
                    target_width as i32,
                    target_height as i32,
                    0,
                    PIXEL_FORMAT_32BPP_PARGB,
                    null(),
                    &mut bitmap,
                )
            },
            "GdipCreateBitmapFromScan0",
        )?;
        let bitmap = NativeImage(bitmap.cast());
        let mut graphics = null_mut();
        // SAFETY : image valide possédée par `bitmap`.
        check(
            unsafe { GdipGetImageGraphicsContext(bitmap.0, &mut graphics) },
            "GdipGetImageGraphicsContext",
        )?;
        let graphics = NativeGraphics(graphics);
        // SAFETY : contexte valide possédé par `graphics`.
        check(
            unsafe { GdipGraphicsClear(graphics.0, 0) },
            "GdipGraphicsClear",
        )?;
        // SAFETY : idem, mode issu de l'énumération InterpolationMode.
        check(
            unsafe { GdipSetInterpolationMode(graphics.0, InterpolationModeHighQualityBicubic) },
            "GdipSetInterpolationMode",
        )?;
        // SAFETY : contexte et image source valides, possédés par cette fonction.
        check(
            unsafe {
                GdipDrawImageRectI(
                    graphics.0,
                    source.0,
                    0,
                    0,
                    target_width as i32,
                    target_height as i32,
                )
            },
            "GdipDrawImageRectI",
        )?;
        drop(graphics);
        Self::pixels(&bitmap, target_width, target_height)
    }
    fn open(bytes: &[u8]) -> crate::Result<(IStream, NativeImage, u32, u32)> {
        u32::try_from(bytes.len()).map_err(|_| ImageError::TooLarge {
            reason: "encoded image larger than 4 GiB",
        })?;
        // SAFETY : SHCreateMemStream copie les octets du slice dans un IStream possédé.
        let stream =
            unsafe { SHCreateMemStream(Some(bytes)) }.ok_or_else(|| crate::Error::Platform {
                operation: "SHCreateMemStream",
                code: windows::Win32::Foundation::E_OUTOFMEMORY.0,
            })?;
        let mut raw = null_mut();
        // SAFETY : flux IStream valide pendant toute la vie de l'image renvoyée.
        let status = unsafe { GdipLoadImageFromStream(stream.as_raw(), &mut raw) };
        // Données refusées par le décodeur (GDI+ signale aussi OutOfMemory pour un fichier
        // corrompu) ; seul un échec Win32 reste une erreur de plateforme.
        if status == GdiPlus::Win32Error {
            check(status, "GdipLoadImageFromStream")?;
        } else if status != GdiPlus::Ok {
            return Err(ImageError::Decode.into());
        }
        let image = NativeImage(raw);
        let (mut width, mut height) = (0, 0);
        // SAFETY : image valide ; Windows écrit dans un u32 local.
        check(
            unsafe { GdipGetImageWidth(image.0, &mut width) },
            "GdipGetImageWidth",
        )?;
        // SAFETY : idem.
        check(
            unsafe { GdipGetImageHeight(image.0, &mut height) },
            "GdipGetImageHeight",
        )?;
        Ok((stream, image, width, height))
    }
    fn pixels(image: &NativeImage, width: u32, height: u32) -> crate::Result<Image> {
        let rect = GdiPlus::Rect {
            X: 0,
            Y: 0,
            Width: width as i32,
            Height: height as i32,
        };
        let mut data = BitmapData::default();
        // SAFETY : rectangle et structure locaux ; GDI+ remplit `data` jusqu'au déverrouillage.
        check(
            unsafe {
                GdipBitmapLockBits(
                    image.bitmap(),
                    &rect,
                    ImageLockModeRead as u32,
                    PIXEL_FORMAT_32BPP_PARGB,
                    &mut data,
                )
            },
            "GdipBitmapLockBits",
        )?;
        struct Lock<'a>(*mut GpBitmap, &'a mut BitmapData);
        impl Drop for Lock<'_> {
            fn drop(&mut self) {
                // SAFETY : même bitmap et même structure que le verrouillage réussi.
                unsafe { GdipBitmapUnlockBits(self.0, self.1) };
            }
        }
        let lock = Lock(image.bitmap(), &mut data);
        let scan = lock.1.Scan0.cast::<u8>();
        if scan.is_null() || lock.1.Stride.unsigned_abs() < width * 4 {
            return Err(ImageError::Decode.into());
        }
        let row = width as usize * 4;
        let mut pixels = vec![0; row * height as usize];
        for y in 0..height as usize {
            // SAFETY : chaque ligne verrouillée contient au moins `row` octets, à `Stride`
            // octets de la précédente ; la destination a `height` lignes de `row` octets.
            unsafe {
                std::ptr::copy_nonoverlapping(
                    scan.offset(y as isize * lock.1.Stride as isize),
                    pixels.as_mut_ptr().add(y * row),
                    row,
                )
            };
        }
        Image::new(width, height, pixels)
    }
}

/// Encoder une image rendue en PNG dans `path` (encodeur GDI+), sans modifier ses pixels.
/// Erreurs documentées par `diagnostics::write_png`.
pub(super) fn write_png(
    image: &crate::ui::scene::Image,
    path: &std::path::Path,
) -> crate::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    if image.width == 0
        || image.height == 0
        || image.width > 16384
        || image.height > 16384
        || image.pixels.len() != image.width as usize * image.height as usize * 4
    {
        return Err(ImageError::InvalidPixels.into());
    }
    let path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    if path[..path.len() - 1].contains(&0) {
        return Err(crate::Error::invalid_input(
            "PNG path contains a NUL character",
        ));
    }
    let _engine = DrawingEngine::initialize()?;
    let mut pixels = image.pixels.clone();
    let mut raw = null_mut();
    // SAFETY : `pixels` contient largeur * hauteur pixels de 4 octets et survit au bitmap,
    // libéré avant la fin de cette fonction.
    check(
        unsafe {
            GdipCreateBitmapFromScan0(
                image.width as i32,
                image.height as i32,
                image.width as i32 * 4,
                PIXEL_FORMAT_32BPP_PARGB,
                pixels.as_mut_ptr(),
                &mut raw,
            )
        },
        "GdipCreateBitmapFromScan0 (preview)",
    )?;
    let bitmap = NativeImage(raw.cast());
    // SAFETY : chemin terminé par zéro, CLSID local, paramètres d'encodeur absents.
    check(
        unsafe { GdipSaveImageToFile(bitmap.0, path.as_ptr(), &PNG_ENCODER, null()) },
        "GdipSaveImageToFile (PNG preview)",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn jpeg_files_decode_to_aspect_preserving_thumbnails() {
        use crate::{resources::load_image_file, task::CancellationToken};
        use std::os::windows::ffi::OsStrExt;
        // CLSID de l'encodeur JPEG intégré.
        const JPEG_ENCODER: GUID = GUID::from_u128(0x557cf401_1a04_11d3_9a73_0000f81ef32e);
        struct TempFile(std::path::PathBuf);
        impl Drop for TempFile {
            fn drop(&mut self) {
                let _ = std::fs::remove_file(&self.0);
            }
        }
        let _engine = DrawingEngine::initialize().unwrap();
        let mut pixels = vec![255; 320 * 160 * 4];
        let mut bitmap = null_mut();
        // SAFETY : `pixels` couvre 320 * 160 pixels de 4 octets et survit au bitmap.
        check(
            unsafe {
                GdipCreateBitmapFromScan0(
                    320,
                    160,
                    320 * 4,
                    PIXEL_FORMAT_32BPP_PARGB,
                    pixels.as_mut_ptr(),
                    &mut bitmap,
                )
            },
            "test bitmap",
        )
        .unwrap();
        let bitmap = NativeImage(bitmap.cast());
        let path = std::env::temp_dir().join(format!(
            "karbon-ui-jpeg-test-{}-{}.jpg",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        assert!(!path.exists());
        let path = TempFile(path);
        let wide: Vec<_> = path.0.as_os_str().encode_wide().chain(Some(0)).collect();
        // SAFETY : chemin terminé par zéro, CLSID constant, paramètres d'encodeur absents.
        check(
            unsafe { GdipSaveImageToFile(bitmap.0, wide.as_ptr(), &JPEG_ENCODER, null()) },
            "test JPEG encoder",
        )
        .unwrap();
        let result = load_image_file(
            &BackgroundDecoder,
            &path.0,
            ImageLoadOptions {
                thumbnail_edge: 96,
                ..Default::default()
            },
            &CancellationToken::new(),
        )
        .unwrap();
        assert_eq!(result.source.format, crate::resources::ImageFormat::Jpeg);
        assert_eq!((result.source.width, result.source.height), (320, 160));
        assert_eq!((result.image.width, result.image.height), (96, 48));
    }
}
