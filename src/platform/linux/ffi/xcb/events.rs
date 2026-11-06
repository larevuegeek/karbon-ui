//! Événements du serveur X (`xcb/xproto.h`) : structures de 32 octets (un même type sert
//! aux touches, boutons et déplacements), types d'événements (`response_type & 0x7f`) et
//! modes des changements de focus. Les champs que le backend ne lit pas sont gardés pour
//! la disposition.
#![allow(dead_code)]

#[repr(C)]
pub(in crate::platform::linux) struct xcb_expose_event_t {
    pub response_type: u8,
    pub pad0: u8,
    pub sequence: u16,
    pub window: u32,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub count: u16,
    pub pad1: [u8; 2],
}
#[repr(C)]
pub(in crate::platform::linux) struct xcb_configure_notify_event_t {
    pub response_type: u8,
    pub pad0: u8,
    pub sequence: u16,
    pub event: u32,
    pub window: u32,
    pub above_sibling: u32,
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
    pub border_width: u16,
    pub override_redirect: u8,
    pub pad1: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub(in crate::platform::linux) struct xcb_client_message_event_t {
    pub response_type: u8,
    pub format: u8,
    pub sequence: u16,
    pub window: u32,
    pub type_: u32,
    pub data: [u32; 5],
}
#[repr(C)]
pub(in crate::platform::linux) struct xcb_focus_in_event_t {
    pub response_type: u8,
    pub detail: u8,
    pub sequence: u16,
    pub event: u32,
    pub mode: u8,
    pub pad0: [u8; 3],
}
#[repr(C)]
pub(in crate::platform::linux) struct xcb_property_notify_event_t {
    pub response_type: u8,
    pub pad0: u8,
    pub sequence: u16,
    pub window: u32,
    pub atom: u32,
    pub time: u32,
    pub state: u8,
    pub pad1: [u8; 3],
}
#[repr(C)]
pub(in crate::platform::linux) struct xcb_map_notify_event_t {
    pub response_type: u8,
    pub pad0: u8,
    pub sequence: u16,
    pub event: u32,
    pub window: u32,
    pub override_redirect: u8,
    pub pad1: [u8; 3],
}
/// Bouton du pointeur ; touche et déplacement ont la même disposition.
#[repr(C)]
pub(in crate::platform::linux) struct xcb_button_press_event_t {
    pub response_type: u8,
    pub detail: u8,
    pub sequence: u16,
    pub time: u32,
    pub root: u32,
    pub event: u32,
    pub child: u32,
    pub root_x: i16,
    pub root_y: i16,
    pub event_x: i16,
    pub event_y: i16,
    pub state: u16,
    pub same_screen: u8,
    pub pad0: u8,
}
/// Touche enfoncée ou relâchée (`detail` : code de touche).
pub(in crate::platform::linux) type xcb_key_press_event_t = xcb_button_press_event_t;
/// Déplacement du pointeur.
pub(in crate::platform::linux) type xcb_motion_notify_event_t = xcb_button_press_event_t;
#[repr(C)]
pub(in crate::platform::linux) struct xcb_enter_notify_event_t {
    pub response_type: u8,
    pub detail: u8,
    pub sequence: u16,
    pub time: u32,
    pub root: u32,
    pub event: u32,
    pub child: u32,
    pub root_x: i16,
    pub root_y: i16,
    pub event_x: i16,
    pub event_y: i16,
    pub state: u16,
    pub mode: u8,
    pub same_screen_focus: u8,
}
#[repr(C)]
pub(in crate::platform::linux) struct xcb_selection_request_event_t {
    pub response_type: u8,
    pub pad0: u8,
    pub sequence: u16,
    pub time: u32,
    pub owner: u32,
    pub requestor: u32,
    pub selection: u32,
    pub target: u32,
    pub property: u32,
}
#[repr(C)]
pub(in crate::platform::linux) struct xcb_selection_notify_event_t {
    pub response_type: u8,
    pub pad0: u8,
    pub sequence: u16,
    pub time: u32,
    pub requestor: u32,
    pub selection: u32,
    pub target: u32,
    pub property: u32,
}
#[repr(C)]
pub(in crate::platform::linux) struct xcb_selection_clear_event_t {
    pub response_type: u8,
    pub pad0: u8,
    pub sequence: u16,
    pub time: u32,
    pub owner: u32,
    pub selection: u32,
}
// Types d'événements (`response_type & 0x7f`).
pub(in crate::platform::linux) const XCB_KEY_PRESS: u8 = 2;
pub(in crate::platform::linux) const XCB_KEY_RELEASE: u8 = 3;
pub(in crate::platform::linux) const XCB_BUTTON_PRESS: u8 = 4;
pub(in crate::platform::linux) const XCB_BUTTON_RELEASE: u8 = 5;
pub(in crate::platform::linux) const XCB_MOTION_NOTIFY: u8 = 6;
pub(in crate::platform::linux) const XCB_ENTER_NOTIFY: u8 = 7;
pub(in crate::platform::linux) const XCB_LEAVE_NOTIFY: u8 = 8;
pub(in crate::platform::linux) const XCB_FOCUS_IN: u8 = 9;
pub(in crate::platform::linux) const XCB_FOCUS_OUT: u8 = 10;
pub(in crate::platform::linux) const XCB_EXPOSE: u8 = 12;
pub(in crate::platform::linux) const XCB_UNMAP_NOTIFY: u8 = 18;
pub(in crate::platform::linux) const XCB_MAP_NOTIFY: u8 = 19;
pub(in crate::platform::linux) const XCB_CONFIGURE_NOTIFY: u8 = 22;
pub(in crate::platform::linux) const XCB_PROPERTY_NOTIFY: u8 = 28;
pub(in crate::platform::linux) const XCB_SELECTION_CLEAR: u8 = 29;
pub(in crate::platform::linux) const XCB_SELECTION_REQUEST: u8 = 30;
pub(in crate::platform::linux) const XCB_SELECTION_NOTIFY: u8 = 31;
pub(in crate::platform::linux) const XCB_CLIENT_MESSAGE: u8 = 33;
/// Événement générique d'une extension (XInput 2).
pub(in crate::platform::linux) const XCB_GE_GENERIC: u8 = 35;
// Mode d'un changement de focus (`xcb_notify_mode_t`) : saisie du clavier par un autre
// client, et son relâchement.
pub(in crate::platform::linux) const XCB_NOTIFY_MODE_GRAB: u8 = 1;
pub(in crate::platform::linux) const XCB_NOTIFY_MODE_UNGRAB: u8 = 2;
/// Détail d'un changement de focus (`xcb_notify_detail_t`) : focus suivant le pointeur.
pub(in crate::platform::linux) const XCB_NOTIFY_DETAIL_POINTER: u8 = 5;

const _: () = assert!(size_of::<xcb_expose_event_t>() == 20);
const _: () = assert!(size_of::<xcb_configure_notify_event_t>() == 28);
const _: () = assert!(size_of::<xcb_client_message_event_t>() == 32);
const _: () = assert!(size_of::<xcb_focus_in_event_t>() == 12);
const _: () = assert!(size_of::<xcb_property_notify_event_t>() == 20);
const _: () = assert!(size_of::<xcb_map_notify_event_t>() == 16);
const _: () = assert!(size_of::<xcb_button_press_event_t>() == 32);
const _: () = assert!(size_of::<xcb_selection_request_event_t>() == 28);
const _: () = assert!(size_of::<xcb_selection_notify_event_t>() == 24);
const _: () = assert!(size_of::<xcb_selection_clear_event_t>() == 16);
const _: () = assert!(size_of::<xcb_enter_notify_event_t>() == 32);
