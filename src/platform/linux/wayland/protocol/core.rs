//! Numéros des requêtes et des événements du protocole de base, transcrits de
//! `wayland.xml` ; les interfaces elles-mêmes sont exportées par libwayland-client.

/// Numéros de `wl_display` : requêtes, puis événements (`EV_`).
pub(in crate::platform::linux) mod wl_display {
    pub const SYNC: u32 = 0;
    pub const GET_REGISTRY: u32 = 1;
    pub const EV_ERROR: u32 = 0;
    pub const EV_DELETE_ID: u32 = 1;
}
/// Numéros de `wl_registry` : requêtes, puis événements (`EV_`).
pub(in crate::platform::linux) mod wl_registry {
    pub const BIND: u32 = 0;
    pub const EV_GLOBAL: u32 = 0;
    pub const EV_GLOBAL_REMOVE: u32 = 1;
}
/// Numéros de `wl_callback` : requêtes, puis événements (`EV_`).
pub(in crate::platform::linux) mod wl_callback {
    pub const EV_DONE: u32 = 0;
}
/// Numéros de `wl_compositor` : requêtes, puis événements (`EV_`).
pub(in crate::platform::linux) mod wl_compositor {
    pub const CREATE_SURFACE: u32 = 0;
    pub const CREATE_REGION: u32 = 1;
}
/// Numéros de `wl_shm_pool` : requêtes, puis événements (`EV_`).
pub(in crate::platform::linux) mod wl_shm_pool {
    pub const CREATE_BUFFER: u32 = 0;
    pub const DESTROY: u32 = 1;
    pub const RESIZE: u32 = 2;
}
/// Numéros de `wl_shm` : requêtes, puis événements (`EV_`).
pub(in crate::platform::linux) mod wl_shm {
    pub const CREATE_POOL: u32 = 0;
    pub const EV_FORMAT: u32 = 0;
}
/// Numéros de `wl_buffer` : requêtes, puis événements (`EV_`).
pub(in crate::platform::linux) mod wl_buffer {
    pub const DESTROY: u32 = 0;
    pub const EV_RELEASE: u32 = 0;
}
/// Numéros de `wl_data_offer` : requêtes, puis événements (`EV_`).
pub(in crate::platform::linux) mod wl_data_offer {
    pub const ACCEPT: u32 = 0;
    pub const RECEIVE: u32 = 1;
    pub const DESTROY: u32 = 2;
    pub const FINISH: u32 = 3;
    pub const SET_ACTIONS: u32 = 4;
    pub const EV_OFFER: u32 = 0;
    pub const EV_SOURCE_ACTIONS: u32 = 1;
    pub const EV_ACTION: u32 = 2;
}
/// Numéros de `wl_data_source` : requêtes, puis événements (`EV_`).
pub(in crate::platform::linux) mod wl_data_source {
    pub const OFFER: u32 = 0;
    pub const DESTROY: u32 = 1;
    pub const SET_ACTIONS: u32 = 2;
    pub const EV_TARGET: u32 = 0;
    pub const EV_SEND: u32 = 1;
    pub const EV_CANCELLED: u32 = 2;
    pub const EV_DND_DROP_PERFORMED: u32 = 3;
    pub const EV_DND_FINISHED: u32 = 4;
    pub const EV_ACTION: u32 = 5;
}
/// Numéros de `wl_data_device` : requêtes, puis événements (`EV_`).
pub(in crate::platform::linux) mod wl_data_device {
    pub const START_DRAG: u32 = 0;
    pub const SET_SELECTION: u32 = 1;
    pub const RELEASE: u32 = 2;
    pub const EV_DATA_OFFER: u32 = 0;
    pub const EV_ENTER: u32 = 1;
    pub const EV_LEAVE: u32 = 2;
    pub const EV_MOTION: u32 = 3;
    pub const EV_DROP: u32 = 4;
    pub const EV_SELECTION: u32 = 5;
}
/// Numéros de `wl_data_device_manager` : requêtes, puis événements (`EV_`).
pub(in crate::platform::linux) mod wl_data_device_manager {
    pub const CREATE_DATA_SOURCE: u32 = 0;
    pub const GET_DATA_DEVICE: u32 = 1;
}
/// Numéros de `wl_surface` : requêtes, puis événements (`EV_`).
pub(in crate::platform::linux) mod wl_surface {
    pub const DESTROY: u32 = 0;
    pub const ATTACH: u32 = 1;
    pub const DAMAGE: u32 = 2;
    pub const FRAME: u32 = 3;
    pub const SET_OPAQUE_REGION: u32 = 4;
    pub const SET_INPUT_REGION: u32 = 5;
    pub const COMMIT: u32 = 6;
    pub const SET_BUFFER_TRANSFORM: u32 = 7;
    pub const SET_BUFFER_SCALE: u32 = 8;
    pub const DAMAGE_BUFFER: u32 = 9;
    pub const OFFSET: u32 = 10;
    pub const EV_ENTER: u32 = 0;
    pub const EV_LEAVE: u32 = 1;
    pub const EV_PREFERRED_BUFFER_SCALE: u32 = 2;
    pub const EV_PREFERRED_BUFFER_TRANSFORM: u32 = 3;
}
/// Numéros de `wl_seat` : requêtes, puis événements (`EV_`).
pub(in crate::platform::linux) mod wl_seat {
    pub const GET_POINTER: u32 = 0;
    pub const GET_KEYBOARD: u32 = 1;
    pub const GET_TOUCH: u32 = 2;
    pub const RELEASE: u32 = 3;
    pub const EV_CAPABILITIES: u32 = 0;
    pub const EV_NAME: u32 = 1;
}
/// Numéros de `wl_pointer` : requêtes, puis événements (`EV_`).
pub(in crate::platform::linux) mod wl_pointer {
    pub const SET_CURSOR: u32 = 0;
    pub const RELEASE: u32 = 1;
    pub const EV_ENTER: u32 = 0;
    pub const EV_LEAVE: u32 = 1;
    pub const EV_MOTION: u32 = 2;
    pub const EV_BUTTON: u32 = 3;
    pub const EV_AXIS: u32 = 4;
    pub const EV_FRAME: u32 = 5;
    pub const EV_AXIS_SOURCE: u32 = 6;
    pub const EV_AXIS_STOP: u32 = 7;
    pub const EV_AXIS_DISCRETE: u32 = 8;
    pub const EV_AXIS_VALUE120: u32 = 9;
    pub const EV_AXIS_RELATIVE_DIRECTION: u32 = 10;
}
/// Numéros de `wl_keyboard` : requêtes, puis événements (`EV_`).
pub(in crate::platform::linux) mod wl_keyboard {
    pub const RELEASE: u32 = 0;
    pub const EV_KEYMAP: u32 = 0;
    pub const EV_ENTER: u32 = 1;
    pub const EV_LEAVE: u32 = 2;
    pub const EV_KEY: u32 = 3;
    pub const EV_MODIFIERS: u32 = 4;
    pub const EV_REPEAT_INFO: u32 = 5;
}
/// Numéros de `wl_output` : requêtes, puis événements (`EV_`).
pub(in crate::platform::linux) mod wl_output {
    pub const RELEASE: u32 = 0;
    pub const EV_GEOMETRY: u32 = 0;
    pub const EV_MODE: u32 = 1;
    pub const EV_DONE: u32 = 2;
    pub const EV_SCALE: u32 = 3;
    pub const EV_NAME: u32 = 4;
    pub const EV_DESCRIPTION: u32 = 5;
}
/// Numéros de `wl_subcompositor` : requêtes, puis événements (`EV_`).
pub(in crate::platform::linux) mod wl_subcompositor {
    pub const DESTROY: u32 = 0;
    pub const GET_SUBSURFACE: u32 = 1;
}
/// Numéros de `wl_subsurface` : requêtes, puis événements (`EV_`).
pub(in crate::platform::linux) mod wl_subsurface {
    pub const DESTROY: u32 = 0;
    pub const SET_POSITION: u32 = 1;
    pub const PLACE_ABOVE: u32 = 2;
    pub const PLACE_BELOW: u32 = 3;
    pub const SET_SYNC: u32 = 4;
    pub const SET_DESYNC: u32 = 5;
}
/// Numéros de `wl_region` : requêtes, puis événements (`EV_`).
pub(in crate::platform::linux) mod wl_region {
    pub const DESTROY: u32 = 0;
    pub const ADD: u32 = 1;
    pub const SUBTRACT: u32 = 2;
}
