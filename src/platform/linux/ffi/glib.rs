//! GLib et GObject : comptage de références des objets, listes chaînées et erreurs.
use std::{
    ffi::{c_char, c_int, c_void},
    ptr::NonNull,
};

pub(in crate::platform::linux) type Gboolean = c_int;
/// Liste simplement chaînée de GLib (`GSList`).
#[repr(C)]
pub(in crate::platform::linux) struct GSList {
    pub data: *mut c_void,
    pub next: *mut GSList,
}
/// Erreur renvoyée par les fonctions GLib (`GError`).
#[repr(C)]
pub(in crate::platform::linux) struct GError {
    pub domain: u32,
    pub code: c_int,
    pub message: *mut c_char,
}
#[link(name = "gobject-2.0")]
unsafe extern "C" {
    fn g_object_ref(object: *mut c_void) -> *mut c_void;
    fn g_object_unref(object: *mut c_void);
}
#[link(name = "glib-2.0")]
unsafe extern "C" {
    pub(in crate::platform::linux) fn g_error_free(error: *mut GError);
}
/// Référence possédée sur un GObject (`PangoLayout`, `PangoContext`, `GdkPixbuf`…).
pub(in crate::platform::linux) struct Object(NonNull<c_void>);
impl Object {
    /// Prend possession de la référence renvoyée par un constructeur (« transfer full »).
    ///
    /// # Safety
    ///
    /// `raw` est nul ou un GObject vivant dont l'appelant cède une référence.
    pub unsafe fn from_full(raw: *mut c_void) -> Option<Self> {
        NonNull::new(raw).map(Self)
    }
    /// Ajoute une référence à un objet emprunté (« transfer none »).
    ///
    /// # Safety
    ///
    /// `raw` est nul ou un GObject vivant.
    pub unsafe fn from_none(raw: *mut c_void) -> Option<Self> {
        // SAFETY : objet vivant d'après l'appelant.
        NonNull::new(raw)
            .map(|object| Self(unsafe { NonNull::new_unchecked(g_object_ref(object.as_ptr())) }))
    }
    pub fn as_ptr(&self) -> *mut c_void {
        self.0.as_ptr()
    }
}
impl Clone for Object {
    fn clone(&self) -> Self {
        // SAFETY : objet vivant tant que `self` existe ; g_object_ref renvoie son argument.
        unsafe { g_object_ref(self.0.as_ptr()) };
        Self(self.0)
    }
}
impl Drop for Object {
    fn drop(&mut self) {
        // SAFETY : une référence possédée est rendue une fois.
        unsafe { g_object_unref(self.0.as_ptr()) };
    }
}
/// Libère une erreur GLib éventuelle (le message n'est pas exposé).
pub(in crate::platform::linux) fn clear_error(error: *mut GError) {
    if !error.is_null() {
        // SAFETY : erreur allouée par GLib, rendue une fois.
        unsafe { g_error_free(error) };
    }
}
