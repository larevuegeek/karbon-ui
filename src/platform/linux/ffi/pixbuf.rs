//! gdk-pixbuf : décodage des images (PNG, JPEG, GIF, WebP… selon les modules installés)
//! en pixels RGB ou RGBA non prémultipliés.
use super::glib::{GError, Object, clear_error};
use std::ffi::{c_int, c_void};

#[link(name = "gdk_pixbuf-2.0")]
unsafe extern "C" {
    fn gdk_pixbuf_loader_new() -> *mut c_void;
    fn gdk_pixbuf_loader_write(
        loader: *mut c_void,
        buffer: *const u8,
        count: usize,
        error: *mut *mut GError,
    ) -> c_int;
    fn gdk_pixbuf_loader_close(loader: *mut c_void, error: *mut *mut GError) -> c_int;
    fn gdk_pixbuf_loader_get_pixbuf(loader: *mut c_void) -> *mut c_void;
    fn gdk_pixbuf_get_width(pixbuf: *const c_void) -> c_int;
    fn gdk_pixbuf_get_height(pixbuf: *const c_void) -> c_int;
    fn gdk_pixbuf_get_rowstride(pixbuf: *const c_void) -> c_int;
    fn gdk_pixbuf_get_n_channels(pixbuf: *const c_void) -> c_int;
    fn gdk_pixbuf_get_has_alpha(pixbuf: *const c_void) -> c_int;
    fn gdk_pixbuf_get_bits_per_sample(pixbuf: *const c_void) -> c_int;
    fn gdk_pixbuf_get_byte_length(pixbuf: *const c_void) -> usize;
    fn gdk_pixbuf_read_pixels(pixbuf: *const c_void) -> *const u8;
}
/// Image décodée : première image du flux, 8 bits par canal.
pub(in crate::platform::linux) struct Pixbuf {
    object: Object,
    pub width: usize,
    pub height: usize,
    pub stride: usize,
    pub channels: usize,
}
impl Pixbuf {
    /// Décode `bytes` ; `None` si aucun module ne reconnaît les données ou si leur
    /// format n'est pas RGB/RGBA 8 bits.
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        // SAFETY : constructeur ; chargeur possédé.
        let loader = unsafe { Object::from_full(gdk_pixbuf_loader_new()) }?;
        let mut error = std::ptr::null_mut();
        // SAFETY : chargeur valide ; octets lus pendant l'appel.
        let written = unsafe {
            gdk_pixbuf_loader_write(loader.as_ptr(), bytes.as_ptr(), bytes.len(), &mut error)
        };
        clear_error(error);
        let mut error = std::ptr::null_mut();
        // SAFETY : le chargeur doit être fermé, même après une erreur d'écriture.
        let closed = unsafe { gdk_pixbuf_loader_close(loader.as_ptr(), &mut error) };
        clear_error(error);
        if written == 0 || closed == 0 {
            return None;
        }
        // SAFETY : image empruntée au chargeur, référencée ici.
        let object = unsafe { Object::from_none(gdk_pixbuf_loader_get_pixbuf(loader.as_ptr())) }?;
        let raw = object.as_ptr();
        // SAFETY : image valide.
        let (width, height, stride, channels, alpha, bits, length) = unsafe {
            (
                gdk_pixbuf_get_width(raw),
                gdk_pixbuf_get_height(raw),
                gdk_pixbuf_get_rowstride(raw),
                gdk_pixbuf_get_n_channels(raw),
                gdk_pixbuf_get_has_alpha(raw) != 0,
                gdk_pixbuf_get_bits_per_sample(raw),
                gdk_pixbuf_get_byte_length(raw),
            )
        };
        let pixbuf = Self {
            object,
            width: usize::try_from(width).ok()?,
            height: usize::try_from(height).ok()?,
            stride: usize::try_from(stride).ok()?,
            channels: usize::try_from(channels).ok()?,
        };
        let valid = bits == 8
            && matches!((pixbuf.channels, alpha), (3, false) | (4, true))
            && pixbuf.width > 0
            && pixbuf.height > 0
            && pixbuf.stride >= pixbuf.width * pixbuf.channels
            && length >= (pixbuf.height - 1) * pixbuf.stride + pixbuf.width * pixbuf.channels;
        valid.then_some(pixbuf)
    }
    /// Octets de l'image (`height` lignes de `stride` octets, la dernière peut être courte).
    pub fn bytes(&self) -> &[u8] {
        // SAFETY : image valide ; longueur contrôlée par `decode`.
        unsafe {
            let raw = self.object.as_ptr();
            std::slice::from_raw_parts(gdk_pixbuf_read_pixels(raw), gdk_pixbuf_get_byte_length(raw))
        }
    }
}
