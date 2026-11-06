//! Saisie par méthode d'entrée, transcrit de
//! `unstable/text-input/text-input-unstable-v3.xml` (wayland-protocols).
use crate::platform::linux::ffi::wayland::{Interface, Message, interface, message};

pub(in crate::platform::linux) static ZWP_TEXT_INPUT_V3: Interface = interface(
    c"zwp_text_input_v3",
    1,
    &ZWP_TEXT_INPUT_V3_REQUESTS,
    &ZWP_TEXT_INPUT_V3_EVENTS,
);
static ZWP_TEXT_INPUT_V3_REQUESTS: [Message; 8] = [
    message(c"destroy", c""),
    message(c"enable", c""),
    message(c"disable", c""),
    message(c"set_surrounding_text", c"sii"),
    message(c"set_text_change_cause", c"u"),
    message(c"set_content_type", c"uu"),
    message(c"set_cursor_rectangle", c"iiii"),
    message(c"commit", c""),
];
static ZWP_TEXT_INPUT_V3_EVENTS: [Message; 6] = [
    message(c"enter", c"o"),
    message(c"leave", c"o"),
    message(c"preedit_string", c"?sii"),
    message(c"commit_string", c"?s"),
    message(c"delete_surrounding_text", c"uu"),
    message(c"done", c"u"),
];
/// Numéros de `zwp_text_input_v3` : requêtes, puis événements (`EV_`).
pub(in crate::platform::linux) mod zwp_text_input_v3 {
    pub const DESTROY: u32 = 0;
    pub const ENABLE: u32 = 1;
    pub const DISABLE: u32 = 2;
    pub const SET_SURROUNDING_TEXT: u32 = 3;
    pub const SET_TEXT_CHANGE_CAUSE: u32 = 4;
    pub const SET_CONTENT_TYPE: u32 = 5;
    pub const SET_CURSOR_RECTANGLE: u32 = 6;
    pub const COMMIT: u32 = 7;
    pub const EV_ENTER: u32 = 0;
    pub const EV_LEAVE: u32 = 1;
    pub const EV_PREEDIT_STRING: u32 = 2;
    pub const EV_COMMIT_STRING: u32 = 3;
    pub const EV_DELETE_SURROUNDING_TEXT: u32 = 4;
    pub const EV_DONE: u32 = 5;
}
pub(in crate::platform::linux) static ZWP_TEXT_INPUT_MANAGER_V3: Interface = interface(
    c"zwp_text_input_manager_v3",
    1,
    &ZWP_TEXT_INPUT_MANAGER_V3_REQUESTS,
    &ZWP_TEXT_INPUT_MANAGER_V3_EVENTS,
);
static ZWP_TEXT_INPUT_MANAGER_V3_REQUESTS: [Message; 2] =
    [message(c"destroy", c""), message(c"get_text_input", c"no")];
static ZWP_TEXT_INPUT_MANAGER_V3_EVENTS: [Message; 0] = [];
/// Numéros de `zwp_text_input_manager_v3` : requêtes, puis événements (`EV_`).
pub(in crate::platform::linux) mod zwp_text_input_manager_v3 {
    pub const DESTROY: u32 = 0;
    pub const GET_TEXT_INPUT: u32 = 1;
}
