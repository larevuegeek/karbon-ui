//! Constantes du protocole X (`xcb/xproto.h`) employées par le backend : masques
//! d'événements, attributs de fenêtre, classes, modes et atomes prédéfinis.

// Masques d'événements.
pub(in crate::platform::linux) const XCB_EVENT_MASK_KEY_PRESS: u32 = 1;
pub(in crate::platform::linux) const XCB_EVENT_MASK_KEY_RELEASE: u32 = 2;
pub(in crate::platform::linux) const XCB_EVENT_MASK_BUTTON_PRESS: u32 = 4;
pub(in crate::platform::linux) const XCB_EVENT_MASK_BUTTON_RELEASE: u32 = 8;
pub(in crate::platform::linux) const XCB_EVENT_MASK_ENTER_WINDOW: u32 = 16;
pub(in crate::platform::linux) const XCB_EVENT_MASK_LEAVE_WINDOW: u32 = 32;
pub(in crate::platform::linux) const XCB_EVENT_MASK_POINTER_MOTION: u32 = 64;
pub(in crate::platform::linux) const XCB_EVENT_MASK_EXPOSURE: u32 = 32768;
pub(in crate::platform::linux) const XCB_EVENT_MASK_STRUCTURE_NOTIFY: u32 = 131072;
pub(in crate::platform::linux) const XCB_EVENT_MASK_SUBSTRUCTURE_NOTIFY: u32 = 524288;
pub(in crate::platform::linux) const XCB_EVENT_MASK_SUBSTRUCTURE_REDIRECT: u32 = 1048576;
pub(in crate::platform::linux) const XCB_EVENT_MASK_FOCUS_CHANGE: u32 = 2097152;
pub(in crate::platform::linux) const XCB_EVENT_MASK_PROPERTY_CHANGE: u32 = 4194304;
// Attributs de fenêtre (`xcb_cw_t`).
pub(in crate::platform::linux) const XCB_CW_BACK_PIXMAP: u32 = 1;
pub(in crate::platform::linux) const XCB_CW_BORDER_PIXEL: u32 = 8;
pub(in crate::platform::linux) const XCB_CW_BIT_GRAVITY: u32 = 16;
pub(in crate::platform::linux) const XCB_CW_EVENT_MASK: u32 = 2048;
pub(in crate::platform::linux) const XCB_CW_COLORMAP: u32 = 8192;
pub(in crate::platform::linux) const XCB_CW_CURSOR: u32 = 16384;
pub(in crate::platform::linux) const XCB_CURRENT_TIME: u32 = 0;
// Masques d'état des événements d'entrée.
pub(in crate::platform::linux) const XCB_MOD_MASK_SHIFT: u16 = 1;
pub(in crate::platform::linux) const XCB_WINDOW_CLASS_INPUT_OUTPUT: u16 = 1;
pub(in crate::platform::linux) const XCB_WINDOW_CLASS_INPUT_ONLY: u16 = 2;
pub(in crate::platform::linux) const XCB_GRAVITY_NORTH_WEST: u32 = 1;
pub(in crate::platform::linux) const XCB_COLORMAP_ALLOC_NONE: u8 = 0;
pub(in crate::platform::linux) const XCB_PROP_MODE_REPLACE: u8 = 0;
pub(in crate::platform::linux) const XCB_IMAGE_FORMAT_Z_PIXMAP: u8 = 2;
pub(in crate::platform::linux) const XCB_IMAGE_ORDER_LSB_FIRST: u8 = 0;
pub(in crate::platform::linux) const XCB_VISUAL_CLASS_TRUE_COLOR: u8 = 4;
// Atomes prédéfinis.
pub(in crate::platform::linux) const XCB_ATOM_ATOM: u32 = 4;
pub(in crate::platform::linux) const XCB_ATOM_CARDINAL: u32 = 6;
pub(in crate::platform::linux) const XCB_ATOM_RESOURCE_MANAGER: u32 = 23;
pub(in crate::platform::linux) const XCB_ATOM_STRING: u32 = 31;
pub(in crate::platform::linux) const XCB_ATOM_WM_NAME: u32 = 39;
pub(in crate::platform::linux) const XCB_ATOM_WM_NORMAL_HINTS: u32 = 40;
pub(in crate::platform::linux) const XCB_ATOM_WM_SIZE_HINTS: u32 = 41;
pub(in crate::platform::linux) const XCB_ATOM_WM_CLASS: u32 = 67;
/// Événements XKB (`xcb_xkb_event_type_t`) : nouveau clavier, disposition changée.
pub(in crate::platform::linux) const XCB_XKB_EVENT_TYPE_NEW_KEYBOARD_NOTIFY: u16 = 1;
pub(in crate::platform::linux) const XCB_XKB_EVENT_TYPE_MAP_NOTIFY: u16 = 2;
/// Toutes les parties d'une disposition (`XkbAllMapComponentsMask`).
pub(in crate::platform::linux) const XCB_XKB_MAP_PART_ALL: u16 = 0xff;
/// États d'un `PropertyNotify`.
pub(in crate::platform::linux) const XCB_PROPERTY_NEW_VALUE: u8 = 0;
pub(in crate::platform::linux) const XCB_PROPERTY_DELETE: u8 = 1;
