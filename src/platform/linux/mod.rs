//! Backend Linux : Cairo, Pango (HarfBuzz, fontconfig, FreeType), gdk-pixbuf, Wayland
//! (libwayland-client, libwayland-cursor), X11 (XCB), xkbcommon, Vulkan et libdbus, par
//! des liaisons C écrites à la main (`ffi/`).
//!
//! Services hors écran (texte, polices embarquées, images, rastérisation, PNG, compteurs
//! du processus) ; deux boucles de fenêtres sur un socle commun (`shell/`) : Wayland s'il
//! y a un compositeur, sinon X11 (`KARBON_BACKEND` impose l'une ou l'autre) ; fenêtres
//! présentées par le renderer GPU Vulkan, Cairo en repli ; barre de titre et bords du
//! moteur quand le compositeur ne décore pas ; méthodes d'entrée (`zwp_text_input_v3`,
//! IBus) ; dialogues de fichiers par le portail XDG ; accessibilité AT-SPI.
//!
//! Carte des modules, en miroir de `platform/macos/` :
//!
//! | Module | Rôle |
//! | --- | --- |
//! | `shell/` | socle des boucles : garde, réveil des tâches, clavier xkbcommon, défilement, composition, barre du moteur, fenêtre vue du moteur |
//! | `wayland/` | boucle Wayland : connexion, protocoles transcrits, fenêtres, siège, presse-papiers, `zwp_text_input_v3` |
//! | `x11/` | boucle X11 : connexion XCB, fenêtres, entrées (XInput 2), presse-papiers, méthode d'entrée IBus |
//! | `ime/` | client IBus sur D-Bus (fenêtres X11) |
//! | `portal/` | portails XDG par D-Bus : dialogues de fichiers (`FileChooser`) |
//! | `accessibility/` | pont AT-SPI par D-Bus (Orca) : arbre, interfaces, événements |
//! | `vulkan/` | renderer GPU Vulkan : appareil, pipeline des quads, shaders SPIR-V, atlas, textures, filtres, présentation |
//! | `graphics/` | renderer Cairo/Pango : canvas, formes, dégradés, texte, filtres CPU |
//! | `fonts.rs` | polices embarquées (fontconfig privé, WOFF/WOFF2 décodés par FreeType), familles système et du bureau |
//! | `desktop.rs` | réglages du bureau (GSettings) : polices, animations |
//! | `imaging.rs` | décodage d'images par gdk-pixbuf et export PNG par Cairo |
//! | `bitmap.rs` | surfaces image Cairo sur des pixels possédés |
//! | `offscreen.rs` | services hors écran (`Offscreen`) et compteurs du processus |
//! | `ffi/` | liaisons GLib/GObject/GIO, Cairo, Pango, fontconfig/FreeType, gdk-pixbuf, Wayland, XCB, xkbcommon, libdbus, Vulkan |
//! | `tests/` | suite minimale du backend |
//!
//! Bibliothèques liées (paquets de développement pour compiler, bibliothèques partagées
//! à l'exécution) : `cairo`, `pango-1.0`, `pangocairo-1.0`, `pangoft2-1.0`, `fontconfig`,
//! `freetype` (avec brotli pour le WOFF2), `gdk_pixbuf-2.0`, `gio-2.0`, `gobject-2.0`,
//! `glib-2.0`, `wayland-client`, `wayland-cursor`, `xkbcommon`, `dbus-1` ;
//! `libvulkan.so.1`, `libxcb.so.1`, `libxcb-cursor.so.0` et `libxcb-xinput.so.0` sont
//! chargées au lancement (`dlopen`), sans paquet de développement.
mod accessibility;
mod bitmap;
mod desktop;
mod ffi;
mod fonts;
mod graphics;
mod imaging;
mod ime;
mod offscreen;
mod portal;
pub(crate) mod secrets;
mod shell;
#[cfg(test)]
mod tests;
mod tray;
// Renderer Vulkan : fenêtres (chemin par défaut) et rendus hors écran en mode `Gpu`.
mod backend;
mod vulkan;
mod wayland;
mod x11;
pub(crate) use backend::*;
