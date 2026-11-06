//! Décorations dessinées par le compositeur, transcrit de
//! `unstable/xdg-decoration/xdg-decoration-unstable-v1.xml` (wayland-protocols).
use crate::platform::linux::ffi::wayland::{Interface, Message, interface, message};

pub(in crate::platform::linux) static ZXDG_DECORATION_MANAGER_V1: Interface = interface(
    c"zxdg_decoration_manager_v1",
    1,
    &ZXDG_DECORATION_MANAGER_V1_REQUESTS,
    &ZXDG_DECORATION_MANAGER_V1_EVENTS,
);
static ZXDG_DECORATION_MANAGER_V1_REQUESTS: [Message; 2] = [
    message(c"destroy", c""),
    message(c"get_toplevel_decoration", c"no"),
];
static ZXDG_DECORATION_MANAGER_V1_EVENTS: [Message; 0] = [];
/// Numéros de `zxdg_decoration_manager_v1` : requêtes, puis événements (`EV_`).
pub(in crate::platform::linux) mod zxdg_decoration_manager_v1 {
    pub const DESTROY: u32 = 0;
    pub const GET_TOPLEVEL_DECORATION: u32 = 1;
}
pub(in crate::platform::linux) static ZXDG_TOPLEVEL_DECORATION_V1: Interface = interface(
    c"zxdg_toplevel_decoration_v1",
    1,
    &ZXDG_TOPLEVEL_DECORATION_V1_REQUESTS,
    &ZXDG_TOPLEVEL_DECORATION_V1_EVENTS,
);
static ZXDG_TOPLEVEL_DECORATION_V1_REQUESTS: [Message; 3] = [
    message(c"destroy", c""),
    message(c"set_mode", c"u"),
    message(c"unset_mode", c""),
];
static ZXDG_TOPLEVEL_DECORATION_V1_EVENTS: [Message; 1] = [message(c"configure", c"u")];
/// Numéros de `zxdg_toplevel_decoration_v1` : requêtes, puis événements (`EV_`).
pub(in crate::platform::linux) mod zxdg_toplevel_decoration_v1 {
    pub const DESTROY: u32 = 0;
    pub const SET_MODE: u32 = 1;
    pub const UNSET_MODE: u32 = 2;
    pub const EV_CONFIGURE: u32 = 0;
}
