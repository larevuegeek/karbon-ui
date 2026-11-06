//! Liaisons C écrites à la main (aucun crate tiers) vers les bibliothèques du bureau
//! Linux. GLib/GObject/GIO, Cairo, Pango, fontconfig, FreeType, gdk-pixbuf, libwayland,
//! xkbcommon et libdbus sont liées à l'édition de liens ; XCB (et ses extensions) et
//! Vulkan sont chargées au lancement (`dlopen`), pour qu'un système sans X11 ni Vulkan
//! démarre quand même. Chaque module déclare les fonctions utilisées et des enveloppes
//! sûres (propriété, libération) ; les pointeurs bruts ne sortent pas du backend.
pub(super) mod cairo;
pub(super) mod dbus;
pub(super) mod fontconfig;
pub(super) mod gio;
pub(super) mod glib;
pub(super) mod pango;
pub(super) mod pixbuf;
pub(super) mod vulkan;
pub(super) mod wayland;
pub(super) mod xcb;
pub(super) mod xkb;

/// Échec d'un appel natif qui ne fournit pas de code d'erreur : une sortie nulle devient
/// `Error::Platform` avec le code générique `E_FAIL`, comme sous macOS.
pub(super) fn failure(operation: &'static str) -> crate::Error {
    crate::Error::Platform {
        operation,
        code: 0x8000_4005_u32 as i32,
    }
}
/// Pointeur non nul obligatoire renvoyé par un appel natif.
pub(super) fn required<T>(
    value: *mut T,
    operation: &'static str,
) -> crate::Result<std::ptr::NonNull<T>> {
    std::ptr::NonNull::new(value).ok_or_else(|| failure(operation))
}
/// Chaîne C d'un nom (famille, attribut) ; un octet nul intérieur est refusé.
pub(super) fn c_string(value: &str) -> crate::Result<std::ffi::CString> {
    std::ffi::CString::new(value).map_err(|_| crate::Error::invalid_input("string contains NUL"))
}
