//! Surfaces image Cairo sur des pixels possédés par le backend, en BGRA prémultiplié
//! (format ARGB32 de Cairo en mémoire petit-boutiste, celui du moteur).
use super::ffi::{
    cairo::{
        self, FORMAT_A8, FORMAT_ARGB32, RawSurface, STATUS_SUCCESS,
        cairo_image_surface_create_for_data, cairo_surface_destroy, cairo_surface_flush,
        cairo_surface_mark_dirty, cairo_surface_status,
    },
    failure,
};
use crate::ui::scene::Image;
use std::{ptr::NonNull, sync::Arc};

/// Surface dont Cairo lit ou écrit les pixels.
pub(super) trait Surface {
    fn raw(&self) -> *mut RawSurface;
    /// Taille en pixels.
    fn size(&self) -> (usize, usize);
    /// Pixels (opérations de Cairo terminées) et octets par ligne.
    fn bytes(&self) -> (&[u8], usize);
}
/// Crée une surface image sur `data` (au moins `stride × height` octets), qui doit
/// survivre à la surface.
fn wrap(
    data: *mut u8,
    format: std::ffi::c_int,
    width: usize,
    height: usize,
    stride: usize,
) -> crate::Result<NonNull<RawSurface>> {
    // SAFETY : tampon valide pour `stride × height` octets d'après l'appelant.
    let raw = unsafe {
        cairo_image_surface_create_for_data(
            data,
            format,
            width as i32,
            height as i32,
            stride as i32,
        )
    };
    let surface = NonNull::new(raw).ok_or_else(|| failure("cairo_image_surface_create"))?;
    // SAFETY : surface valide (éventuellement en erreur).
    if unsafe { cairo_surface_status(raw) } != STATUS_SUCCESS {
        // SAFETY : surface possédée, détruite une fois.
        unsafe { cairo_surface_destroy(raw) };
        return Err(failure("cairo_image_surface_create"));
    }
    Ok(surface)
}
fn check(width: usize, height: usize) -> crate::Result<()> {
    if width == 0 || height == 0 || width > 16384 || height > 16384 {
        return Err(crate::Error::invalid_input("invalid surface size"));
    }
    Ok(())
}
/// Surface et pixels possédés (transparents à la création).
pub(super) struct Bitmap {
    surface: NonNull<RawSurface>,
    pub width: usize,
    pub height: usize,
    stride: usize,
    data: Vec<u8>,
}
impl Bitmap {
    /// Surface BGRA prémultipliée.
    pub fn new(width: usize, height: usize) -> crate::Result<Self> {
        check(width, height)?;
        Self::create(
            vec![0; width * height * 4],
            width,
            height,
            FORMAT_ARGB32,
            width * 4,
        )
    }
    /// Surface d'alpha seul (masques de glyphes du renderer Vulkan).
    pub fn alpha(width: usize, height: usize) -> crate::Result<Self> {
        check(width, height)?;
        let stride = width.div_ceil(4) * 4;
        Self::create(vec![0; stride * height], width, height, FORMAT_A8, stride)
    }
    /// Surface BGRA prémultipliée sur `pixels` (`width × height × 4` octets).
    pub fn from_pixels(width: usize, height: usize, pixels: Vec<u8>) -> crate::Result<Self> {
        check(width, height)?;
        if pixels.len() != width * height * 4 {
            return Err(crate::Error::invalid_input("invalid pixel buffer"));
        }
        Self::create(pixels, width, height, FORMAT_ARGB32, width * 4)
    }
    fn create(
        mut data: Vec<u8>,
        width: usize,
        height: usize,
        format: std::ffi::c_int,
        stride: usize,
    ) -> crate::Result<Self> {
        // Le tampon d'un `Vec` ne bouge pas tant qu'il n'est ni agrandi ni libéré.
        let surface = wrap(data.as_mut_ptr(), format, width, height, stride)?;
        Ok(Self {
            surface,
            width,
            height,
            stride,
            data,
        })
    }
    /// Contexte de dessin sur la surface, qui l'emprunte : les pixels ne peuvent être
    /// rendus (`into_pixels`) qu'une fois le contexte terminé.
    pub fn context(&self) -> crate::Result<cairo::Context<'_>> {
        cairo::Context::new(self)
    }
    /// Pixels après les dessins en cours (`stride` octets par ligne).
    pub fn pixels(&self) -> &[u8] {
        // SAFETY : surface valide ; termine les opérations en attente sur les pixels.
        unsafe { cairo_surface_flush(self.raw()) };
        &self.data
    }
    /// Modifie les pixels hors de Cairo, puis le prévient.
    pub fn edit(&mut self, f: impl FnOnce(&mut [u8])) {
        // SAFETY : idem.
        unsafe { cairo_surface_flush(self.raw()) };
        f(&mut self.data);
        // SAFETY : les pixels ont changé hors de Cairo.
        unsafe { cairo_surface_mark_dirty(self.raw()) };
    }
    pub fn row_bytes(&self) -> usize {
        self.stride
    }
    /// Pixels BGRA prémultipliés de la surface.
    pub fn into_pixels(self) -> Vec<u8> {
        // SAFETY : surface valide.
        unsafe { cairo_surface_flush(self.raw()) };
        let mut this = std::mem::ManuallyDrop::new(self);
        // SAFETY : la surface est détruite avant que le tampon ne soit rendu ; `this`
        // n'est plus utilisé ensuite.
        unsafe {
            cairo_surface_destroy(this.surface.as_ptr());
            std::mem::take(&mut this.data)
        }
    }
}
impl Surface for Bitmap {
    fn raw(&self) -> *mut RawSurface {
        self.surface.as_ptr()
    }
    fn size(&self) -> (usize, usize) {
        (self.width, self.height)
    }
    fn bytes(&self) -> (&[u8], usize) {
        (self.pixels(), self.stride)
    }
}
impl Drop for Bitmap {
    fn drop(&mut self) {
        // SAFETY : référence possédée ; les contextes et motifs créés dessus empruntent
        // le `Bitmap` (`Context<'a>`, `Pattern<'a>`) : aucun ne lui survit.
        unsafe { cairo_surface_destroy(self.raw()) };
    }
}
/// Surface en lecture seule sur les pixels d'une image du moteur, sans copie : l'`Arc`
/// garde les pixels vivants, et Cairo ne fait que les lire (source d'un dessin).
pub(super) struct ImageSurface {
    surface: NonNull<RawSurface>,
    image: Arc<Image>,
}
impl ImageSurface {
    pub fn new(image: &Arc<Image>) -> crate::Result<Self> {
        let (width, height) = (image.width as usize, image.height as usize);
        check(width, height)?;
        if image.pixels.len() != width * height * 4 {
            return Err(crate::Error::invalid_input("invalid image buffer"));
        }
        // Cairo demande un pointeur mutable mais n'écrit jamais dans une source.
        let surface = wrap(
            image.pixels.as_ptr().cast_mut(),
            FORMAT_ARGB32,
            width,
            height,
            width * 4,
        )?;
        Ok(Self {
            surface,
            image: Arc::clone(image),
        })
    }
}
impl Surface for ImageSurface {
    fn raw(&self) -> *mut RawSurface {
        self.surface.as_ptr()
    }
    fn size(&self) -> (usize, usize) {
        (self.image.width as usize, self.image.height as usize)
    }
    fn bytes(&self) -> (&[u8], usize) {
        (&self.image.pixels, self.image.width as usize * 4)
    }
}
impl Drop for ImageSurface {
    fn drop(&mut self) {
        // SAFETY : référence possédée, rendue une fois ; les motifs qui la référencent
        // ne dépassent pas la durée d'un dessin.
        unsafe { cairo_surface_destroy(self.raw()) };
    }
}
/// Exécute `f` avec une surface temporaire en lecture seule sur les pixels de `image`,
/// détruite avant le retour (aucun motif ne doit la garder au-delà de `f`).
pub(super) fn with_image<R>(
    image: &Image,
    f: impl FnOnce(*mut RawSurface) -> R,
) -> crate::Result<R> {
    let (width, height) = (image.width as usize, image.height as usize);
    check(width, height)?;
    if image.pixels.len() != width * height * 4 {
        return Err(crate::Error::invalid_input("invalid image buffer"));
    }
    let surface = wrap(
        image.pixels.as_ptr().cast_mut(),
        FORMAT_ARGB32,
        width,
        height,
        width * 4,
    )?;
    let result = f(surface.as_ptr());
    // SAFETY : référence possédée, rendue une fois, avant la fin de l'emprunt.
    unsafe { cairo_surface_destroy(surface.as_ptr()) };
    Ok(result)
}
