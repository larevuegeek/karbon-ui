//! xdg-shell (stable), transcrit de `stable/xdg-shell/xdg-shell.xml` (wayland-protocols) :
//! interfaces `xdg_wm_base`, `xdg_surface` et `xdg_toplevel`.
use crate::platform::linux::ffi::wayland::{Interface, Message, interface, message};

pub(in crate::platform::linux) static XDG_WM_BASE: Interface = interface(
    c"xdg_wm_base",
    7,
    &XDG_WM_BASE_REQUESTS,
    &XDG_WM_BASE_EVENTS,
);
static XDG_WM_BASE_REQUESTS: [Message; 4] = [
    message(c"destroy", c""),
    message(c"create_positioner", c"n"),
    message(c"get_xdg_surface", c"no"),
    message(c"pong", c"u"),
];
static XDG_WM_BASE_EVENTS: [Message; 1] = [message(c"ping", c"u")];
/// Numéros de `xdg_wm_base` : requêtes, puis événements (`EV_`).
pub(in crate::platform::linux) mod xdg_wm_base {
    pub const DESTROY: u32 = 0;
    pub const CREATE_POSITIONER: u32 = 1;
    pub const GET_XDG_SURFACE: u32 = 2;
    pub const PONG: u32 = 3;
    pub const EV_PING: u32 = 0;
}
pub(in crate::platform::linux) static XDG_SURFACE: Interface = interface(
    c"xdg_surface",
    7,
    &XDG_SURFACE_REQUESTS,
    &XDG_SURFACE_EVENTS,
);
static XDG_SURFACE_REQUESTS: [Message; 5] = [
    message(c"destroy", c""),
    message(c"get_toplevel", c"n"),
    message(c"get_popup", c"n?oo"),
    message(c"set_window_geometry", c"iiii"),
    message(c"ack_configure", c"u"),
];
static XDG_SURFACE_EVENTS: [Message; 1] = [message(c"configure", c"u")];
/// Numéros de `xdg_surface` : requêtes, puis événements (`EV_`).
pub(in crate::platform::linux) mod xdg_surface {
    pub const DESTROY: u32 = 0;
    pub const GET_TOPLEVEL: u32 = 1;
    pub const GET_POPUP: u32 = 2;
    pub const SET_WINDOW_GEOMETRY: u32 = 3;
    pub const ACK_CONFIGURE: u32 = 4;
    pub const EV_CONFIGURE: u32 = 0;
}
pub(in crate::platform::linux) static XDG_TOPLEVEL: Interface = interface(
    c"xdg_toplevel",
    7,
    &XDG_TOPLEVEL_REQUESTS,
    &XDG_TOPLEVEL_EVENTS,
);
static XDG_TOPLEVEL_REQUESTS: [Message; 14] = [
    message(c"destroy", c""),
    message(c"set_parent", c"?o"),
    message(c"set_title", c"s"),
    message(c"set_app_id", c"s"),
    message(c"show_window_menu", c"ouii"),
    message(c"move", c"ou"),
    message(c"resize", c"ouu"),
    message(c"set_max_size", c"ii"),
    message(c"set_min_size", c"ii"),
    message(c"set_maximized", c""),
    message(c"unset_maximized", c""),
    message(c"set_fullscreen", c"?o"),
    message(c"unset_fullscreen", c""),
    message(c"set_minimized", c""),
];
static XDG_TOPLEVEL_EVENTS: [Message; 4] = [
    message(c"configure", c"iia"),
    message(c"close", c""),
    message(c"configure_bounds", c"4ii"),
    message(c"wm_capabilities", c"5a"),
];
/// Numéros de `xdg_toplevel` : requêtes, puis événements (`EV_`).
pub(in crate::platform::linux) mod xdg_toplevel {
    pub const DESTROY: u32 = 0;
    pub const SET_PARENT: u32 = 1;
    pub const SET_TITLE: u32 = 2;
    pub const SET_APP_ID: u32 = 3;
    pub const SHOW_WINDOW_MENU: u32 = 4;
    pub const MOVE: u32 = 5;
    pub const RESIZE: u32 = 6;
    pub const SET_MAX_SIZE: u32 = 7;
    pub const SET_MIN_SIZE: u32 = 8;
    pub const SET_MAXIMIZED: u32 = 9;
    pub const UNSET_MAXIMIZED: u32 = 10;
    pub const SET_FULLSCREEN: u32 = 11;
    pub const UNSET_FULLSCREEN: u32 = 12;
    pub const SET_MINIMIZED: u32 = 13;
    pub const EV_CONFIGURE: u32 = 0;
    pub const EV_CLOSE: u32 = 1;
    pub const EV_CONFIGURE_BOUNDS: u32 = 2;
    pub const EV_WM_CAPABILITIES: u32 = 3;
}
