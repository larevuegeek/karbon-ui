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
mod support;
pub(super) mod vulkan;
pub(super) mod wayland;
pub(super) mod xcb;
pub(super) mod xkb;
pub(crate) use support::*;
