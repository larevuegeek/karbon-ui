//! Échelle des surfaces, transcrit de `stable/viewporter/viewporter.xml` et
//! `staging/fractional-scale/fractional-scale-v1.xml` (wayland-protocols).
use crate::platform::linux::ffi::wayland::{Interface, Message, interface, message};

pub(in crate::platform::linux) static WP_VIEWPORTER: Interface = interface(
    c"wp_viewporter",
    1,
    &WP_VIEWPORTER_REQUESTS,
    &WP_VIEWPORTER_EVENTS,
);
static WP_VIEWPORTER_REQUESTS: [Message; 2] =
    [message(c"destroy", c""), message(c"get_viewport", c"no")];
static WP_VIEWPORTER_EVENTS: [Message; 0] = [];
/// Numéros de `wp_viewporter` : requêtes, puis événements (`EV_`).
pub(in crate::platform::linux) mod wp_viewporter {
    pub const DESTROY: u32 = 0;
    pub const GET_VIEWPORT: u32 = 1;
}
pub(in crate::platform::linux) static WP_VIEWPORT: Interface = interface(
    c"wp_viewport",
    1,
    &WP_VIEWPORT_REQUESTS,
    &WP_VIEWPORT_EVENTS,
);
static WP_VIEWPORT_REQUESTS: [Message; 3] = [
    message(c"destroy", c""),
    message(c"set_source", c"ffff"),
    message(c"set_destination", c"ii"),
];
static WP_VIEWPORT_EVENTS: [Message; 0] = [];
/// Numéros de `wp_viewport` : requêtes, puis événements (`EV_`).
pub(in crate::platform::linux) mod wp_viewport {
    pub const DESTROY: u32 = 0;
    pub const SET_SOURCE: u32 = 1;
    pub const SET_DESTINATION: u32 = 2;
}
pub(in crate::platform::linux) static WP_FRACTIONAL_SCALE_MANAGER_V1: Interface = interface(
    c"wp_fractional_scale_manager_v1",
    1,
    &WP_FRACTIONAL_SCALE_MANAGER_V1_REQUESTS,
    &WP_FRACTIONAL_SCALE_MANAGER_V1_EVENTS,
);
static WP_FRACTIONAL_SCALE_MANAGER_V1_REQUESTS: [Message; 2] = [
    message(c"destroy", c""),
    message(c"get_fractional_scale", c"no"),
];
static WP_FRACTIONAL_SCALE_MANAGER_V1_EVENTS: [Message; 0] = [];
/// Numéros de `wp_fractional_scale_manager_v1` : requêtes, puis événements (`EV_`).
pub(in crate::platform::linux) mod wp_fractional_scale_manager_v1 {
    pub const DESTROY: u32 = 0;
    pub const GET_FRACTIONAL_SCALE: u32 = 1;
}
pub(in crate::platform::linux) static WP_FRACTIONAL_SCALE_V1: Interface = interface(
    c"wp_fractional_scale_v1",
    1,
    &WP_FRACTIONAL_SCALE_V1_REQUESTS,
    &WP_FRACTIONAL_SCALE_V1_EVENTS,
);
static WP_FRACTIONAL_SCALE_V1_REQUESTS: [Message; 1] = [message(c"destroy", c"")];
static WP_FRACTIONAL_SCALE_V1_EVENTS: [Message; 1] = [message(c"preferred_scale", c"u")];
/// Numéros de `wp_fractional_scale_v1` : requêtes, puis événements (`EV_`).
pub(in crate::platform::linux) mod wp_fractional_scale_v1 {
    pub const DESTROY: u32 = 0;
    pub const EV_PREFERRED_SCALE: u32 = 0;
}
