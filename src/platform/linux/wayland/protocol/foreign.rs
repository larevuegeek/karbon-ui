//! Fenêtre exportée pour un autre client (`xdg-foreign`) : la poignée rend le dialogue de
//! fichiers du portail modal à la fenêtre (`wayland:<poignée>`). Transcrit de
//! `unstable/xdg-foreign/xdg-foreign-unstable-v2.xml` et `-v1.xml` (wayland-protocols) :
//! les deux versions ont les mêmes messages, seuls les noms changent.
use crate::platform::linux::ffi::wayland::{Interface, Message, interface, message};

pub(in crate::platform::linux) static ZXDG_EXPORTER_V2: Interface =
    interface(c"zxdg_exporter_v2", 1, &EXPORTER_REQUESTS, &EXPORTER_EVENTS);
pub(in crate::platform::linux) static ZXDG_EXPORTER_V1: Interface =
    interface(c"zxdg_exporter_v1", 1, &EXPORTER_REQUESTS, &EXPORTER_EVENTS);
static EXPORTER_REQUESTS: [Message; 2] = [
    message(c"destroy", c""),
    // `export_toplevel` (v2), `export` (v1) : même signature.
    message(c"export_toplevel", c"no"),
];
static EXPORTER_EVENTS: [Message; 0] = [];
/// Numéros de `zxdg_exporter_v2` et `zxdg_exporter_v1` : requêtes, puis événements (`EV_`).
pub(in crate::platform::linux) mod zxdg_exporter {
    pub const DESTROY: u32 = 0;
    pub const EXPORT_TOPLEVEL: u32 = 1;
}
pub(in crate::platform::linux) static ZXDG_EXPORTED_V2: Interface =
    interface(c"zxdg_exported_v2", 1, &EXPORTED_REQUESTS, &EXPORTED_EVENTS);
pub(in crate::platform::linux) static ZXDG_EXPORTED_V1: Interface =
    interface(c"zxdg_exported_v1", 1, &EXPORTED_REQUESTS, &EXPORTED_EVENTS);
static EXPORTED_REQUESTS: [Message; 1] = [message(c"destroy", c"")];
static EXPORTED_EVENTS: [Message; 1] = [message(c"handle", c"s")];
/// Numéros de `zxdg_exported_v2` et `zxdg_exported_v1` : requêtes, puis événements (`EV_`).
pub(in crate::platform::linux) mod zxdg_exported {
    pub const DESTROY: u32 = 0;
    pub const EV_HANDLE: u32 = 0;
}
