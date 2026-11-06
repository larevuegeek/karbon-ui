//! Requêtes, réponses et structures du serveur (`xcb_setup_t`, écrans, profondeurs,
//! visuels) transcrites de `xcb/xcb.h` et `xcb/xproto.h`. Les champs que le backend ne lit
//! pas (remplissage, numéros de séquence) sont gardés pour la disposition.
#![allow(dead_code)]
use std::ffi::c_int;

/// Requête sans réponse.
#[repr(C)]
#[derive(Clone, Copy)]
pub(in crate::platform::linux) struct xcb_void_cookie_t {
    pub sequence: u32,
}
/// Requête avec réponse (même disposition pour toutes).
#[repr(C)]
#[derive(Clone, Copy)]
pub(in crate::platform::linux) struct xcb_cookie_t {
    pub sequence: u32,
}
#[repr(C)]
pub(in crate::platform::linux) struct xcb_generic_event_t {
    pub response_type: u8,
    pub pad0: u8,
    pub sequence: u16,
    pub pad: [u32; 7],
    pub full_sequence: u32,
}
#[repr(C)]
pub(in crate::platform::linux) struct xcb_generic_error_t {
    pub response_type: u8,
    pub error_code: u8,
    pub sequence: u16,
    pub resource_id: u32,
    pub minor_code: u16,
    pub major_code: u8,
    pub pad0: u8,
    pub pad: [u32; 5],
    pub full_sequence: u32,
}
#[repr(C)]
pub(in crate::platform::linux) struct xcb_translate_coordinates_reply_t {
    pub response_type: u8,
    pub same_screen: u8,
    pub sequence: u16,
    pub length: u32,
    pub child: u32,
    pub dst_x: i16,
    pub dst_y: i16,
}
#[repr(C)]
pub(in crate::platform::linux) struct xcb_query_extension_reply_t {
    pub response_type: u8,
    pub pad0: u8,
    pub sequence: u16,
    pub length: u32,
    pub present: u8,
    pub major_opcode: u8,
    pub first_event: u8,
    pub first_error: u8,
}
#[repr(C)]
pub(in crate::platform::linux) struct xcb_input_xi_query_version_reply_t {
    pub response_type: u8,
    pub pad0: u8,
    pub sequence: u16,
    pub length: u32,
    pub major_version: u16,
    pub minor_version: u16,
    pub pad1: [u8; 20],
}
#[repr(C)]
pub(in crate::platform::linux) struct xcb_intern_atom_reply_t {
    pub response_type: u8,
    pub pad0: u8,
    pub sequence: u16,
    pub length: u32,
    pub atom: u32,
}
#[repr(C)]
pub(in crate::platform::linux) struct xcb_get_selection_owner_reply_t {
    pub response_type: u8,
    pub pad0: u8,
    pub sequence: u16,
    pub length: u32,
    pub owner: u32,
}
#[repr(C)]
pub(in crate::platform::linux) struct xcb_get_property_reply_t {
    pub response_type: u8,
    pub format: u8,
    pub sequence: u16,
    pub length: u32,
    pub type_: u32,
    pub bytes_after: u32,
    pub value_len: u32,
    pub pad0: [u8; 12],
}
#[repr(C)]
pub(in crate::platform::linux) struct xcb_setup_t {
    pub status: u8,
    pub pad0: u8,
    pub protocol_major_version: u16,
    pub protocol_minor_version: u16,
    pub length: u16,
    pub release_number: u32,
    pub resource_id_base: u32,
    pub resource_id_mask: u32,
    pub motion_buffer_size: u32,
    pub vendor_len: u16,
    pub maximum_request_length: u16,
    pub roots_len: u8,
    pub pixmap_formats_len: u8,
    pub image_byte_order: u8,
    pub bitmap_format_bit_order: u8,
    pub bitmap_format_scanline_unit: u8,
    pub bitmap_format_scanline_pad: u8,
    pub min_keycode: u8,
    pub max_keycode: u8,
    pub pad1: [u8; 4],
}
#[repr(C)]
pub(in crate::platform::linux) struct xcb_screen_t {
    pub root: u32,
    pub default_colormap: u32,
    pub white_pixel: u32,
    pub black_pixel: u32,
    pub current_input_masks: u32,
    pub width_in_pixels: u16,
    pub height_in_pixels: u16,
    pub width_in_millimeters: u16,
    pub height_in_millimeters: u16,
    pub min_installed_maps: u16,
    pub max_installed_maps: u16,
    pub root_visual: u32,
    pub backing_stores: u8,
    pub save_unders: u8,
    pub root_depth: u8,
    pub allowed_depths_len: u8,
}
#[repr(C)]
pub(in crate::platform::linux) struct xcb_screen_iterator_t {
    pub data: *mut xcb_screen_t,
    pub rem: c_int,
    pub index: c_int,
}
#[repr(C)]
pub(in crate::platform::linux) struct xcb_depth_t {
    pub depth: u8,
    pub pad0: u8,
    pub visuals_len: u16,
    pub pad1: [u8; 4],
}
#[repr(C)]
pub(in crate::platform::linux) struct xcb_depth_iterator_t {
    pub data: *mut xcb_depth_t,
    pub rem: c_int,
    pub index: c_int,
}
#[repr(C)]
pub(in crate::platform::linux) struct xcb_visualtype_t {
    pub visual_id: u32,
    pub class: u8,
    pub bits_per_rgb_value: u8,
    pub colormap_entries: u16,
    pub red_mask: u32,
    pub green_mask: u32,
    pub blue_mask: u32,
    pub pad0: [u8; 4],
}
#[repr(C)]
pub(in crate::platform::linux) struct xcb_visualtype_iterator_t {
    pub data: *mut xcb_visualtype_t,
    pub rem: c_int,
    pub index: c_int,
}

const _: () = assert!(size_of::<xcb_generic_event_t>() == 36);
const _: () = assert!(size_of::<xcb_generic_error_t>() == 36);
const _: () = assert!(size_of::<xcb_intern_atom_reply_t>() == 12);
const _: () = assert!(size_of::<xcb_get_selection_owner_reply_t>() == 12);
const _: () = assert!(size_of::<xcb_get_property_reply_t>() == 32);
const _: () = assert!(size_of::<xcb_setup_t>() == 40);
const _: () = assert!(size_of::<xcb_screen_t>() == 40);
const _: () = assert!(size_of::<xcb_screen_iterator_t>() == 16);
const _: () = assert!(size_of::<xcb_depth_t>() == 8);
const _: () = assert!(size_of::<xcb_visualtype_t>() == 24);
const _: () = assert!(size_of::<xcb_query_extension_reply_t>() == 12);
const _: () = assert!(size_of::<xcb_translate_coordinates_reply_t>() == 16);
const _: () = assert!(size_of::<xcb_input_xi_query_version_reply_t>() == 32);
