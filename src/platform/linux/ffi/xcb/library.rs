use super::*;
pub(super) use std::{
    ffi::{CStr, c_char, c_int, c_void},
    sync::OnceLock,
};

pub(in crate::platform::linux) type xcb_connection_t = c_void;

/// Table de fonctions d'une bibliothèque ouverte par `dlopen` ; `None` si la bibliothèque
/// ou l'une des fonctions manque.
macro_rules! library {
    ($name:ident ($soname:literal) { $( fn $f:ident ( $( $a:ident : $t:ty ),* $(,)? ) $( -> $r:ty )? ; )* }) => {
        pub(in crate::platform::linux) struct $name {
            $( pub $f: unsafe extern "C" fn( $( $t ),* ) $( -> $r )?, )*
        }
        impl $name {
            /// Ouvre la bibliothèque (jamais refermée : ses fonctions servent tout le
            /// processus) et résout chaque fonction.
            fn open() -> Option<Self> {
                // SAFETY : nom statique terminé par un octet nul.
                let handle = unsafe {
                    libc::dlopen(concat!($soname, "\0").as_ptr().cast(), libc::RTLD_NOW | libc::RTLD_LOCAL)
                };
                if handle.is_null() {
                    return None;
                }
                Some(Self { $( $f: {
                    let name = CStr::from_bytes_with_nul(concat!(stringify!($f), "\0").as_bytes()).ok()?;
                    // SAFETY : bibliothèque ouverte ; nom terminé par un octet nul.
                    let address = unsafe { libc::dlsym(handle, name.as_ptr()) };
                    if address.is_null() {
                        // SAFETY : bibliothèque ouverte ci-dessus, inutilisée.
                        unsafe { libc::dlclose(handle) };
                        return None;
                    }
                    // SAFETY : fonction de ce nom, de la signature déclarée ici.
                    unsafe { std::mem::transmute::<*mut c_void, unsafe extern "C" fn( $( $t ),* ) $( -> $r )?>(address) }
                }, )* })
            }
        }
    };
}

library! {
    Xcb("libxcb.so.1") {
        fn xcb_connect(displayname: *const c_char, screenp: *mut c_int) -> *mut xcb_connection_t;
        fn xcb_disconnect(c: *mut xcb_connection_t);
        fn xcb_connection_has_error(c: *mut xcb_connection_t) -> c_int;
        fn xcb_get_setup(c: *mut xcb_connection_t) -> *const xcb_setup_t;
        fn xcb_setup_roots_iterator(r: *const xcb_setup_t) -> xcb_screen_iterator_t;
        fn xcb_screen_next(i: *mut xcb_screen_iterator_t);
        fn xcb_screen_allowed_depths_iterator(r: *const xcb_screen_t) -> xcb_depth_iterator_t;
        fn xcb_depth_next(i: *mut xcb_depth_iterator_t);
        fn xcb_depth_visuals_iterator(r: *const xcb_depth_t) -> xcb_visualtype_iterator_t;
        fn xcb_visualtype_next(i: *mut xcb_visualtype_iterator_t);
        fn xcb_generate_id(c: *mut xcb_connection_t) -> u32;
        fn xcb_flush(c: *mut xcb_connection_t) -> c_int;
        fn xcb_get_file_descriptor(c: *mut xcb_connection_t) -> c_int;
        fn xcb_get_maximum_request_length(c: *mut xcb_connection_t) -> u32;
        fn xcb_poll_for_event(c: *mut xcb_connection_t) -> *mut xcb_generic_event_t;
        fn xcb_poll_for_queued_event(c: *mut xcb_connection_t) -> *mut xcb_generic_event_t;
        fn xcb_ungrab_pointer(c: *mut xcb_connection_t, time: u32) -> xcb_void_cookie_t;
        fn xcb_free_cursor(c: *mut xcb_connection_t, cursor: u32) -> xcb_void_cookie_t;
        fn xcb_translate_coordinates(c: *mut xcb_connection_t, src_window: u32, dst_window: u32, src_x: i16, src_y: i16) -> xcb_cookie_t;
        fn xcb_translate_coordinates_reply(c: *mut xcb_connection_t, cookie: xcb_cookie_t, e: *mut *mut xcb_generic_error_t) -> *mut xcb_translate_coordinates_reply_t;
        fn xcb_get_extension_data(c: *mut xcb_connection_t, ext: *mut c_void) -> *const xcb_query_extension_reply_t;
        fn xcb_set_selection_owner(c: *mut xcb_connection_t, owner: u32, selection: u32, time: u32) -> xcb_void_cookie_t;
        fn xcb_get_selection_owner(c: *mut xcb_connection_t, selection: u32) -> xcb_cookie_t;
        fn xcb_get_selection_owner_reply(c: *mut xcb_connection_t, cookie: xcb_cookie_t, e: *mut *mut xcb_generic_error_t) -> *mut xcb_get_selection_owner_reply_t;
        fn xcb_convert_selection(c: *mut xcb_connection_t, requestor: u32, selection: u32, target: u32, property: u32, time: u32) -> xcb_void_cookie_t;
        fn xcb_intern_atom(c: *mut xcb_connection_t, only_if_exists: u8, name_len: u16, name: *const c_char) -> xcb_cookie_t;
        fn xcb_intern_atom_reply(c: *mut xcb_connection_t, cookie: xcb_cookie_t, e: *mut *mut xcb_generic_error_t) -> *mut xcb_intern_atom_reply_t;
        fn xcb_create_window(c: *mut xcb_connection_t, depth: u8, wid: u32, parent: u32, x: i16, y: i16, width: u16, height: u16, border_width: u16, class: u16, visual: u32, value_mask: u32, value_list: *const c_void) -> xcb_void_cookie_t;
        fn xcb_destroy_window(c: *mut xcb_connection_t, window: u32) -> xcb_void_cookie_t;
        fn xcb_map_window(c: *mut xcb_connection_t, window: u32) -> xcb_void_cookie_t;
        fn xcb_unmap_window(c: *mut xcb_connection_t, window: u32) -> xcb_void_cookie_t;
        fn xcb_configure_window(c: *mut xcb_connection_t, window: u32, value_mask: u16, value_list: *const c_void) -> xcb_void_cookie_t;
        fn xcb_change_window_attributes(c: *mut xcb_connection_t, window: u32, value_mask: u32, value_list: *const c_void) -> xcb_void_cookie_t;
        fn xcb_change_property(c: *mut xcb_connection_t, mode: u8, window: u32, property: u32, type_: u32, format: u8, data_len: u32, data: *const c_void) -> xcb_void_cookie_t;
        fn xcb_get_property(c: *mut xcb_connection_t, delete: u8, window: u32, property: u32, type_: u32, long_offset: u32, long_length: u32) -> xcb_cookie_t;
        fn xcb_get_property_reply(c: *mut xcb_connection_t, cookie: xcb_cookie_t, e: *mut *mut xcb_generic_error_t) -> *mut xcb_get_property_reply_t;
        fn xcb_get_property_value(r: *const xcb_get_property_reply_t) -> *mut c_void;
        fn xcb_get_property_value_length(r: *const xcb_get_property_reply_t) -> c_int;
        fn xcb_send_event(c: *mut xcb_connection_t, propagate: u8, destination: u32, event_mask: u32, event: *const c_char) -> xcb_void_cookie_t;
        fn xcb_create_colormap(c: *mut xcb_connection_t, alloc: u8, mid: u32, window: u32, visual: u32) -> xcb_void_cookie_t;
        fn xcb_free_colormap(c: *mut xcb_connection_t, cmap: u32) -> xcb_void_cookie_t;
        fn xcb_create_gc(c: *mut xcb_connection_t, cid: u32, drawable: u32, value_mask: u32, value_list: *const c_void) -> xcb_void_cookie_t;
        fn xcb_free_gc(c: *mut xcb_connection_t, gc: u32) -> xcb_void_cookie_t;
        fn xcb_put_image(c: *mut xcb_connection_t, format: u8, drawable: u32, gc: u32, width: u16, height: u16, dst_x: i16, dst_y: i16, left_pad: u8, depth: u8, data_len: u32, data: *const u8) -> xcb_void_cookie_t;
    }
}
/// Table XCB du processus ; `None` sans `libxcb.so.1`.
pub(in crate::platform::linux) fn xcb() -> Option<&'static Xcb> {
    static XCB: OnceLock<Option<Xcb>> = OnceLock::new();
    XCB.get_or_init(Xcb::open).as_ref()
}

library! {
    XcbCursor("libxcb-cursor.so.0") {
        fn xcb_cursor_context_new(conn: *mut xcb_connection_t, screen: *const xcb_screen_t, ctx: *mut *mut c_void) -> c_int;
        fn xcb_cursor_load_cursor(ctx: *mut c_void, name: *const c_char) -> u32;
        fn xcb_cursor_context_free(ctx: *mut c_void);
    }
}
/// Curseurs du thème (`libxcb-cursor.so.0`), s'ils sont installés.
pub(in crate::platform::linux) fn xcb_cursor() -> Option<&'static XcbCursor> {
    static CURSOR: OnceLock<Option<XcbCursor>> = OnceLock::new();
    CURSOR.get_or_init(XcbCursor::open).as_ref()
}

library! {
    XInputFns("libxcb-xinput.so.0") {
        fn xcb_input_xi_query_version(c: *mut xcb_connection_t, major: u16, minor: u16) -> xcb_cookie_t;
        fn xcb_input_xi_query_version_reply(c: *mut xcb_connection_t, cookie: xcb_cookie_t, e: *mut *mut xcb_generic_error_t) -> *mut xcb_input_xi_query_version_reply_t;
        fn xcb_input_xi_select_events(c: *mut xcb_connection_t, window: u32, num_mask: u16, masks: *const c_void) -> xcb_void_cookie_t;
        fn xcb_input_xi_query_device(c: *mut xcb_connection_t, deviceid: u16) -> xcb_cookie_t;
        fn xcb_input_xi_query_device_reply(c: *mut xcb_connection_t, cookie: xcb_cookie_t, e: *mut *mut xcb_generic_error_t) -> *mut u8;
    }
}
/// Bibliothèque XInput 2 (`libxcb-xinput.so.0`) : fonctions et identifiant de
/// l'extension (`xcb_input_id`, donnée de la bibliothèque, clé de
/// `xcb_get_extension_data`). L'état XInput de la boucle est `x11::xinput::XInput`.
pub(in crate::platform::linux) struct XInputLib {
    pub fns: XInputFns,
    pub id: *mut c_void,
}
// SAFETY : `id` désigne une donnée statique de la bibliothèque, jamais refermée.
unsafe impl Send for XInputLib {}
// SAFETY : idem ; XCB protège ses propres accès.
unsafe impl Sync for XInputLib {}
/// XInput 2 du processus, s'il est installé.
pub(in crate::platform::linux) fn xinput() -> Option<&'static XInputLib> {
    static XINPUT: OnceLock<Option<XInputLib>> = OnceLock::new();
    XINPUT
        .get_or_init(|| {
            let fns = XInputFns::open()?;
            // SAFETY : bibliothèque déjà ouverte (référence de plus, jamais rendue) ; nom
            // statique terminé par un octet nul.
            let id = unsafe {
                let handle = libc::dlopen(
                    c"libxcb-xinput.so.0".as_ptr(),
                    libc::RTLD_NOW | libc::RTLD_LOCAL,
                );
                if handle.is_null() {
                    return None;
                }
                libc::dlsym(handle, c"xcb_input_id".as_ptr())
            };
            (!id.is_null()).then_some(XInputLib { fns, id })
        })
        .as_ref()
}
