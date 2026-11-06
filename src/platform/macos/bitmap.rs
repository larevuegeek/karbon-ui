//! Surfaces bitmap Core Graphics dont le tampon appartient à Rust : BGRA prémultiplié
//! en sRGB (format de `Image` et des rendus) et masques en niveaux de gris.
use super::ffi::{failure, required, srgb};
use crate::ui::scene::Image;
use objc2_core_foundation::{CFData, CFRetained};
use objc2_core_graphics::{
    CGBitmapContextCreate, CGBitmapContextCreateImage, CGBitmapInfo, CGColorRenderingIntent,
    CGColorSpace, CGContext, CGDataProvider, CGImage, CGImageAlphaInfo, CGImageByteOrderInfo,
};

/// `kCGImageAlphaPremultipliedFirst | kCGBitmapByteOrder32Little` : octets B, G, R, A.
pub(super) const BGRA: u32 =
    CGImageAlphaInfo::PremultipliedFirst.0 | CGImageByteOrderInfo::Order32Little.0;

/// Contexte bitmap et son tampon. L'ordre des champs fixe l'ordre de destruction : le
/// contexte, qui écrit dans `data`, disparaît avant lui.
pub(super) struct Bitmap {
    pub context: CFRetained<CGContext>,
    pub width: usize,
    pub height: usize,
    /// Octets par pixel : 4 (BGRA) ou 1 (gris).
    channels: usize,
    /// Tampon écrit par Core Graphics via le pointeur donné à la création : un `Vec`
    /// (pointeur brut interne) plutôt qu'un `Box`, dont l'exclusivité serait violée.
    data: Vec<u8>,
}
impl Bitmap {
    /// Surface BGRA prémultipliée transparente de `width` × `height` pixels.
    pub fn new(width: usize, height: usize) -> crate::Result<Self> {
        Self::create(width, height, 4, &*srgb()?, BGRA)
    }
    /// Masque 8 bits en niveaux de gris sans alpha (`CGContextClipToMask`), noir.
    pub fn gray(width: usize, height: usize) -> crate::Result<Self> {
        let space = required(
            CGColorSpace::new_device_gray(),
            "CGColorSpaceCreateDeviceGray",
        )?;
        Self::create(width, height, 1, &space, CGImageAlphaInfo::None.0)
    }
    fn create(
        width: usize,
        height: usize,
        channels: usize,
        space: &CGColorSpace,
        info: u32,
    ) -> crate::Result<Self> {
        if width == 0 || height == 0 || width > 16384 || height > 16384 {
            return Err(crate::Error::invalid_input("invalid bitmap size"));
        }
        let mut data = vec![0u8; width * height * channels];
        // SAFETY : tampon de `width * height * channels` octets, possédé par la structure
        // renvoyée et détruit après le contexte.
        let context = unsafe {
            CGBitmapContextCreate(
                data.as_mut_ptr().cast(),
                width,
                height,
                8,
                width * channels,
                Some(space),
                info,
            )
        };
        Ok(Self {
            context: required(context, "CGBitmapContextCreate")?,
            width,
            height,
            channels,
            data,
        })
    }
    pub fn pixels(&self) -> &[u8] {
        &self.data
    }
    pub fn pixels_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }
    pub fn row_bytes(&self) -> usize {
        self.width * self.channels
    }
    /// Copie immuable du contenu courant.
    pub fn image(&self) -> crate::Result<CFRetained<CGImage>> {
        required(
            CGBitmapContextCreateImage(Some(&self.context)),
            "CGBitmapContextCreateImage",
        )
    }
    pub fn into_pixels(self) -> Vec<u8> {
        let Self { context, data, .. } = self;
        drop(context);
        data
    }
}
/// Image Core Graphics BGRA prémultipliée sur `provider` (`width * height * 4` octets).
fn image_from_provider(
    width: usize,
    height: usize,
    provider: &CGDataProvider,
) -> crate::Result<CFRetained<CGImage>> {
    // SAFETY : fournisseur de `width * height * 4` octets ; aucune table de décodage.
    let image = unsafe {
        CGImage::new(
            width,
            height,
            8,
            32,
            width * 4,
            Some(&*srgb()?),
            CGBitmapInfo(BGRA),
            Some(provider),
            std::ptr::null(),
            true,
            CGColorRenderingIntent::RenderingIntentDefault,
        )
    };
    image.ok_or_else(|| failure("CGImageCreate"))
}
/// Image Core Graphics BGRA prémultipliée copiée depuis `pixels`.
pub(super) fn image_from_bgra(
    width: usize,
    height: usize,
    pixels: &[u8],
) -> crate::Result<CFRetained<CGImage>> {
    if width == 0 || height == 0 || pixels.len() != width * height * 4 {
        return Err(crate::Error::invalid_input("invalid image buffer size"));
    }
    let data = CFData::from_bytes(pixels);
    let provider = required(
        CGDataProvider::with_cf_data(Some(&data)),
        "CGDataProviderCreateWithCFData",
    )?;
    image_from_provider(width, height, &provider)
}
/// Libère la référence à l'`Image` retenue par un fournisseur sans copie.
unsafe extern "C-unwind" fn release(
    info: *mut std::ffi::c_void,
    _: std::ptr::NonNull<std::ffi::c_void>,
    _: usize,
) {
    // SAFETY : `info` vient de `Box::into_raw` dans `image`, libéré une seule fois.
    drop(unsafe { Box::from_raw(info.cast::<std::sync::Arc<Image>>()) });
}
/// Image Core Graphics d'une `Image` du moteur, sans copie des pixels : l'image Core
/// Graphics retient l'`Arc` jusqu'à sa libération.
pub(super) fn image(image: &std::sync::Arc<Image>) -> crate::Result<CFRetained<CGImage>> {
    let (width, height) = (image.width as usize, image.height as usize);
    if width == 0 || height == 0 || image.pixels.len() != width * height * 4 {
        return Err(crate::Error::invalid_input("invalid image buffer size"));
    }
    let info = Box::into_raw(Box::new(std::sync::Arc::clone(image)));
    // SAFETY : les pixels restent valides tant que l'`Arc` retenu par `info` vit ;
    // `release` le libère quand Core Graphics n'en a plus besoin.
    let provider = unsafe {
        CGDataProvider::with_data(
            info.cast(),
            image.pixels.as_ptr().cast(),
            image.pixels.len(),
            Some(release),
        )
    };
    let Some(provider) = provider else {
        // SAFETY : fournisseur non créé, `info` n'a pas été confié à Core Graphics.
        drop(unsafe { Box::from_raw(info) });
        return Err(failure("CGDataProviderCreateWithData"));
    };
    image_from_provider(width, height, &provider)
}
