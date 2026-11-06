//! libwayland-client (connexion, objets, envoi des requêtes, réception des événements par
//! un répartiteur unique) et libwayland-cursor (thèmes de curseurs).
//!
//! Les interfaces du protocole de base sont exportées par libwayland-client ; celles des
//! extensions (`xdg-shell`…) sont décrites par `wayland/protocol/`. Les événements passent
//! par `wl_proxy_add_dispatcher` : un seul répartiteur décode les arguments de tous les
//! objets, sans table de fonctions par interface.
use std::ffi::{CStr, c_char, c_int, c_uint, c_void};

/// Message d'une interface : nom, signature des arguments et interfaces des objets.
#[repr(C)]
pub(in crate::platform::linux) struct Message {
    name: *const c_char,
    signature: *const c_char,
    types: *const *const Interface,
}
/// Description d'une interface (`struct wl_interface`).
#[repr(C)]
pub(in crate::platform::linux) struct Interface {
    name: *const c_char,
    version: c_int,
    method_count: c_int,
    methods: *const Message,
    event_count: c_int,
    events: *const Message,
}
impl Interface {
    /// Nom de l'interface (chaîne C statique).
    pub fn name(&self) -> *const c_char {
        self.name
    }
}
impl Message {
    /// Signature des arguments (lettres de type, préfixe de version, `?` : nullable).
    pub fn signature(&self) -> &CStr {
        // SAFETY : chaîne C statique des tables du protocole (ou de libwayland).
        unsafe { CStr::from_ptr(self.signature) }
    }
}
// SAFETY : tables statiques immuables, lues seulement (aussi par libwayland).
unsafe impl Sync for Message {}
// SAFETY : idem.
unsafe impl Sync for Interface {}
/// Interfaces des arguments objets : aucune n'est donnée. libwayland ne s'en sert que
/// pour vérifier le type d'un objet reçu et créer l'objet d'un `new_id` d'événement ; les
/// extensions utilisées n'en ont pas, et les nouveaux objets des requêtes reçoivent leur
/// interface à l'envoi. libwayland lit une entrée par argument : la table en compte
/// `UNTYPED_ARGS`, et `message` refuse à la compilation une signature plus longue.
const UNTYPED_ARGS: usize = 8;
#[repr(transparent)]
struct Untyped([*const Interface; UNTYPED_ARGS]);
// SAFETY : pointeurs nuls, jamais modifiés.
unsafe impl Sync for Untyped {}
static UNTYPED: Untyped = Untyped([std::ptr::null(); UNTYPED_ARGS]);

pub(in crate::platform::linux) const fn message(
    name: &'static CStr,
    signature: &'static CStr,
) -> Message {
    assert!(
        signature.count_bytes() <= UNTYPED_ARGS,
        "message signature longer than the UNTYPED table"
    );
    Message {
        name: name.as_ptr(),
        signature: signature.as_ptr(),
        types: UNTYPED.0.as_ptr(),
    }
}
pub(in crate::platform::linux) const fn interface(
    name: &'static CStr,
    version: c_int,
    requests: &'static [Message],
    events: &'static [Message],
) -> Interface {
    Interface {
        name: name.as_ptr(),
        version,
        method_count: requests.len() as c_int,
        methods: requests.as_ptr(),
        event_count: events.len() as c_int,
        events: events.as_ptr(),
    }
}
/// Argument d'un événement (`union wl_argument`).
#[repr(C)]
#[derive(Clone, Copy)]
pub(in crate::platform::linux) union Argument {
    pub i: i32,
    pub u: u32,
    /// Nombre à virgule fixe 24.8.
    pub f: i32,
    pub s: *const c_char,
    pub o: *mut c_void,
    pub a: *const Array,
    pub h: i32,
}
/// Tableau d'octets d'un argument (`struct wl_array`).
#[repr(C)]
pub(in crate::platform::linux) struct Array {
    pub size: usize,
    pub alloc: usize,
    pub data: *mut c_void,
}
pub(in crate::platform::linux) type Dispatcher = unsafe extern "C" fn(
    data: *const c_void,
    target: *mut c_void,
    opcode: u32,
    message: *const Message,
    args: *mut Argument,
) -> c_int;
/// L'objet est détruit par la requête envoyée (destructeur du protocole).
pub(in crate::platform::linux) const MARSHAL_DESTROY: u32 = 1;

#[link(name = "wayland-client")]
unsafe extern "C" {
    pub(in crate::platform::linux) static wl_registry_interface: Interface;
    pub(in crate::platform::linux) static wl_compositor_interface: Interface;
    pub(in crate::platform::linux) static wl_surface_interface: Interface;
    pub(in crate::platform::linux) static wl_region_interface: Interface;
    pub(in crate::platform::linux) static wl_callback_interface: Interface;
    pub(in crate::platform::linux) static wl_shm_interface: Interface;
    pub(in crate::platform::linux) static wl_shm_pool_interface: Interface;
    pub(in crate::platform::linux) static wl_buffer_interface: Interface;
    pub(in crate::platform::linux) static wl_seat_interface: Interface;
    pub(in crate::platform::linux) static wl_pointer_interface: Interface;
    pub(in crate::platform::linux) static wl_keyboard_interface: Interface;
    pub(in crate::platform::linux) static wl_output_interface: Interface;
    pub(in crate::platform::linux) static wl_data_device_manager_interface: Interface;
    pub(in crate::platform::linux) static wl_data_device_interface: Interface;
    pub(in crate::platform::linux) static wl_data_source_interface: Interface;
    pub(in crate::platform::linux) fn wl_display_connect(name: *const c_char) -> *mut c_void;
    pub(in crate::platform::linux) fn wl_display_disconnect(display: *mut c_void);
    pub(in crate::platform::linux) fn wl_display_get_fd(display: *mut c_void) -> c_int;
    pub(in crate::platform::linux) fn wl_display_flush(display: *mut c_void) -> c_int;
    pub(in crate::platform::linux) fn wl_display_prepare_read(display: *mut c_void) -> c_int;
    pub(in crate::platform::linux) fn wl_display_read_events(display: *mut c_void) -> c_int;
    pub(in crate::platform::linux) fn wl_display_cancel_read(display: *mut c_void);
    pub(in crate::platform::linux) fn wl_display_dispatch_pending(display: *mut c_void) -> c_int;
    pub(in crate::platform::linux) fn wl_display_roundtrip(display: *mut c_void) -> c_int;
    pub(in crate::platform::linux) fn wl_display_get_error(display: *mut c_void) -> c_int;
    pub(in crate::platform::linux) fn wl_proxy_marshal_flags(
        proxy: *mut c_void,
        opcode: u32,
        interface: *const Interface,
        version: u32,
        flags: u32,
        ...
    ) -> *mut c_void;
    pub(in crate::platform::linux) fn wl_proxy_add_dispatcher(
        proxy: *mut c_void,
        dispatcher: Dispatcher,
        data: *const c_void,
        user: *mut c_void,
    ) -> c_int;
    pub(in crate::platform::linux) fn wl_proxy_destroy(proxy: *mut c_void);
    pub(in crate::platform::linux) fn wl_proxy_get_version(proxy: *mut c_void) -> u32;
}
/// Image d'un curseur (`struct wl_cursor_image`), en pixels du thème.
#[repr(C)]
pub(in crate::platform::linux) struct CursorImage {
    pub width: u32,
    pub height: u32,
    pub hotspot_x: u32,
    pub hotspot_y: u32,
    pub delay: u32,
}
/// Curseur d'un thème (`struct wl_cursor`) ; ses images appartiennent au thème.
#[repr(C)]
pub(in crate::platform::linux) struct Cursor {
    pub image_count: c_uint,
    pub images: *mut *mut CursorImage,
    pub name: *mut c_char,
}
#[link(name = "wayland-cursor")]
unsafe extern "C" {
    pub(in crate::platform::linux) fn wl_cursor_theme_load(
        name: *const c_char,
        size: c_int,
        shm: *mut c_void,
    ) -> *mut c_void;
    pub(in crate::platform::linux) fn wl_cursor_theme_destroy(theme: *mut c_void);
    pub(in crate::platform::linux) fn wl_cursor_theme_get_cursor(
        theme: *mut c_void,
        name: *const c_char,
    ) -> *mut Cursor;
    pub(in crate::platform::linux) fn wl_cursor_image_get_buffer(
        image: *mut CursorImage,
    ) -> *mut c_void;
}
