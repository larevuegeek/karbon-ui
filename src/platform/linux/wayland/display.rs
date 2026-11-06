//! Connexion au compositeur : envoi des requêtes, répartiteur unique des événements (mis en
//! file, traités par la boucle hors de tout appel à libwayland) et objets globaux liés.
use super::protocol::{self, wl_registry};
use crate::platform::linux::ffi::{
    c_string,
    wayland::{
        Argument, Interface, Message, wl_display_connect, wl_display_disconnect,
        wl_display_get_error, wl_display_roundtrip, wl_proxy_add_dispatcher, wl_registry_interface,
    },
};
use std::{
    cell::RefCell,
    ffi::{CStr, c_char, c_int, c_void},
    os::fd::{FromRawFd, OwnedFd},
};

/// Objet du protocole (`wl_proxy`), possédé par la boucle.
pub(super) type Ptr = *mut c_void;
/// Emplacement d'un `new_id` dans les arguments d'une requête.
pub(super) const NEW: Ptr = std::ptr::null_mut();

/// Envoie une requête sur `proxy` (version de l'objet), avec ses arguments C dans
/// l'ordre du protocole.
macro_rules! request {
    ($proxy:expr, $opcode:expr $(, $arg:expr)* $(,)?) => {{
        let proxy: $crate::platform::linux::wayland::display::Ptr = $proxy;
        // SAFETY : objet vivant possédé par la boucle ; arguments conformes à la
        // signature du message (vérifiée par libwayland à l'envoi).
        let _ = unsafe {
            $crate::platform::linux::ffi::wayland::wl_proxy_marshal_flags(
                proxy,
                $opcode,
                std::ptr::null(),
                $crate::platform::linux::ffi::wayland::wl_proxy_get_version(proxy),
                0
                $(, $arg)*
            )
        };
    }};
}
/// Envoie une requête qui crée un objet d'interface `$interface` (même version que
/// `proxy`) et lui attache le répartiteur avec la nature `$kind`.
macro_rules! create {
    ($proxy:expr, $opcode:expr, $interface:expr, $kind:expr $(, $arg:expr)* $(,)?) => {{
        let proxy: $crate::platform::linux::wayland::display::Ptr = $proxy;
        // SAFETY : comme `request!` ; l'objet créé appartient à l'appelant.
        let object = unsafe {
            $crate::platform::linux::ffi::wayland::wl_proxy_marshal_flags(
                proxy,
                $opcode,
                $interface,
                $crate::platform::linux::ffi::wayland::wl_proxy_get_version(proxy),
                0
                $(, $arg)*
            )
        };
        $crate::platform::linux::wayland::display::listen(object, $kind);
        object
    }};
}
/// Envoie la requête destructrice `$opcode` de `proxy`, qui n'est plus utilisable ensuite.
macro_rules! destroy {
    ($proxy:expr, $opcode:expr $(, $arg:expr)* $(,)?) => {{
        let proxy: $crate::platform::linux::wayland::display::Ptr = $proxy;
        if !proxy.is_null() {
            // SAFETY : objet vivant, détruit par cette requête et oublié par l'appelant.
            unsafe {
                $crate::platform::linux::ffi::wayland::wl_proxy_marshal_flags(
                    proxy,
                    $opcode,
                    std::ptr::null(),
                    $crate::platform::linux::ffi::wayland::wl_proxy_get_version(proxy),
                    $crate::platform::linux::ffi::wayland::MARSHAL_DESTROY
                    $(, $arg)*
                )
            };
        }
    }};
}
pub(super) use {create, destroy, request};

/// Nature d'un objet, transmise au répartiteur avec chaque événement.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Kind {
    Registry = 1,
    WmBase,
    XdgSurface,
    Toplevel,
    Surface,
    Callback,
    Buffer,
    Seat,
    Pointer,
    Keyboard,
    Output,
    DataDevice,
    DataOffer,
    DataSource,
    Fractional,
    Decoration,
    TextInput,
    /// Objet sans événement utile (région, viewport, pool).
    Silent,
}
const KINDS: [Kind; 18] = [
    Kind::Registry,
    Kind::WmBase,
    Kind::XdgSurface,
    Kind::Toplevel,
    Kind::Surface,
    Kind::Callback,
    Kind::Buffer,
    Kind::Seat,
    Kind::Pointer,
    Kind::Keyboard,
    Kind::Output,
    Kind::DataDevice,
    Kind::DataOffer,
    Kind::DataSource,
    Kind::Fractional,
    Kind::Decoration,
    Kind::TextInput,
    Kind::Silent,
];
/// Argument décodé d'un événement.
pub(super) enum Value {
    Int(i32),
    Uint(u32),
    Fixed(f64),
    Str(String),
    Object(Ptr),
    Array(Vec<u8>),
    Fd(Option<OwnedFd>),
}
/// Événement reçu : objet, nature, numéro et arguments dans l'ordre du protocole.
pub(super) struct Event {
    pub kind: Kind,
    pub target: Ptr,
    pub opcode: u32,
    pub args: Vec<Value>,
}
impl Event {
    pub fn uint(&self, i: usize) -> u32 {
        match self.args.get(i) {
            Some(Value::Uint(v)) => *v,
            Some(Value::Int(v)) => *v as u32,
            _ => 0,
        }
    }
    pub fn int(&self, i: usize) -> i32 {
        match self.args.get(i) {
            Some(Value::Int(v)) => *v,
            Some(Value::Uint(v)) => *v as i32,
            _ => 0,
        }
    }
    pub fn fixed(&self, i: usize) -> f64 {
        match self.args.get(i) {
            Some(Value::Fixed(v)) => *v,
            _ => 0.,
        }
    }
    pub fn string(&self, i: usize) -> &str {
        match self.args.get(i) {
            Some(Value::Str(v)) => v,
            _ => "",
        }
    }
    pub fn object(&self, i: usize) -> Ptr {
        match self.args.get(i) {
            Some(Value::Object(v)) => *v,
            _ => std::ptr::null_mut(),
        }
    }
    pub fn array(&self, i: usize) -> &[u8] {
        match self.args.get(i) {
            Some(Value::Array(v)) => v,
            _ => &[],
        }
    }
    pub fn take_fd(&mut self, i: usize) -> Option<OwnedFd> {
        match self.args.get_mut(i) {
            Some(Value::Fd(fd)) => fd.take(),
            _ => None,
        }
    }
}
thread_local! {
    static QUEUE: RefCell<Vec<Event>> = const { RefCell::new(Vec::new()) };
}
/// Événements reçus depuis le dernier appel.
pub(super) fn take_events() -> Vec<Event> {
    QUEUE.with(|q| std::mem::take(&mut *q.borrow_mut()))
}
/// Attache le répartiteur à `proxy` (objet fraîchement créé).
pub(super) fn listen(proxy: Ptr, kind: Kind) {
    if !proxy.is_null() {
        // SAFETY : objet vivant sans répartiteur ; la nature passe par la valeur du
        // pointeur de données, jamais déréférencé.
        unsafe {
            wl_proxy_add_dispatcher(
                proxy,
                dispatch,
                kind as usize as *const c_void,
                std::ptr::null_mut(),
            )
        };
    }
}
/// Répartiteur de tous les objets : décode les arguments d'après la signature du message
/// et met l'événement en file. Ne panique pas (frontière C) et n'appelle libwayland que
/// pour attacher le répartiteur à une offre de presse-papiers reçue.
unsafe extern "C" fn dispatch(
    data: *const c_void,
    target: *mut c_void,
    opcode: u32,
    message: *const Message,
    args: *mut Argument,
) -> c_int {
    let Some(kind) = KINDS.iter().copied().find(|k| *k as usize == data as usize) else {
        return 0;
    };
    // SAFETY : message valide de l'interface de l'objet.
    let signature = unsafe { (*message).signature() };
    let mut values = Vec::new();
    let mut index = 0;
    for code in signature.to_bytes() {
        // Préfixe de version et marque de nullité : aucun argument.
        if matches!(code, b'0'..=b'9' | b'?') {
            continue;
        }
        // SAFETY : un argument par lettre de type de la signature (libwayland).
        let arg = unsafe { *args.add(index) };
        let value = match code {
            // SAFETY : champ correspondant à la lettre de type.
            b'i' => Value::Int(unsafe { arg.i }),
            b'u' => Value::Uint(unsafe { arg.u }),
            b'f' => Value::Fixed(f64::from(unsafe { arg.f }) / 256.),
            b's' => Value::Str(string(unsafe { arg.s })),
            b'o' => Value::Object(unsafe { arg.o }),
            b'n' => {
                // Nouvel objet créé par libwayland : seule l'offre de presse-papiers
                // (`wl_data_device.data_offer`) en crée parmi les interfaces utilisées.
                let object = unsafe { arg.o };
                if kind == Kind::DataDevice {
                    listen(object, Kind::DataOffer);
                }
                Value::Object(object)
            }
            b'a' => Value::Array(array(unsafe { arg.a })),
            // SAFETY : descripteur reçu, dont l'événement transmet la propriété.
            b'h' => {
                Value::Fd((unsafe { arg.h } >= 0).then(|| unsafe { OwnedFd::from_raw_fd(arg.h) }))
            }
            _ => return 0,
        };
        values.push(value);
        index += 1;
    }
    let event = Event {
        kind,
        target,
        opcode,
        args: values,
    };
    let _ = QUEUE.try_with(|q| q.borrow_mut().push(event));
    0
}
fn string(value: *const c_char) -> String {
    if value.is_null() {
        return String::new();
    }
    // SAFETY : chaîne C valide pendant l'événement.
    unsafe { CStr::from_ptr(value) }
        .to_string_lossy()
        .into_owned()
}
fn array(value: *const crate::platform::linux::ffi::wayland::Array) -> Vec<u8> {
    if value.is_null() {
        return Vec::new();
    }
    // SAFETY : tableau valide pendant l'événement, `size` octets à `data`.
    unsafe {
        let a = &*value;
        if a.data.is_null() {
            Vec::new()
        } else {
            std::slice::from_raw_parts(a.data.cast::<u8>(), a.size).to_vec()
        }
    }
}
/// Objets globaux liés au démarrage.
pub(super) struct Globals {
    pub compositor: Ptr,
    pub shm: Ptr,
    pub wm_base: Ptr,
    /// Siège (clavier, souris) : nom global et objet.
    pub seat: Option<(u32, Ptr)>,
    /// Sorties : nom global, objet et échelle entière annoncée.
    pub outputs: Vec<(u32, Ptr, i32)>,
    pub data_manager: Ptr,
    pub viewporter: Ptr,
    pub fractional: Ptr,
    pub decoration: Ptr,
    pub text_input: Ptr,
}
pub(super) struct Display {
    pub raw: Ptr,
    registry: Ptr,
    pub globals: Globals,
}
impl Display {
    /// Connexion à `WAYLAND_DISPLAY` et liaison des objets globaux.
    pub fn connect() -> crate::Result<Self> {
        // SAFETY : nom nul : variable d'environnement `WAYLAND_DISPLAY`.
        let raw = unsafe { wl_display_connect(std::ptr::null()) };
        if raw.is_null() {
            return Err(crate::Error::Unsupported {
                reason: "no Wayland compositor (WAYLAND_DISPLAY); without an X server (DISPLAY) either, no window can be opened",
            });
        }
        let registry = create!(
            raw,
            protocol::wl_display::GET_REGISTRY,
            &raw const wl_registry_interface,
            Kind::Registry,
            NEW
        );
        let mut display = Self {
            raw,
            registry,
            globals: Globals {
                compositor: std::ptr::null_mut(),
                shm: std::ptr::null_mut(),
                wm_base: std::ptr::null_mut(),
                seat: None,
                outputs: Vec::new(),
                data_manager: std::ptr::null_mut(),
                viewporter: std::ptr::null_mut(),
                fractional: std::ptr::null_mut(),
                decoration: std::ptr::null_mut(),
                text_input: std::ptr::null_mut(),
            },
        };
        display.roundtrip()?;
        let pending = take_events();
        for event in pending {
            display.global(&event);
        }
        let g = &display.globals;
        if g.compositor.is_null() || g.shm.is_null() || g.wm_base.is_null() {
            return Err(crate::Error::Unsupported {
                reason: "the Wayland compositor lacks wl_compositor 4, wl_shm or xdg_wm_base",
            });
        }
        Ok(display)
    }
    /// Aller-retour : toutes les requêtes envoyées ont été traitées par le compositeur.
    pub fn roundtrip(&self) -> crate::Result<()> {
        // SAFETY : connexion valide.
        if unsafe { wl_display_roundtrip(self.raw) } < 0 {
            return Err(self.error("wl_display_roundtrip"));
        }
        Ok(())
    }
    /// Erreur de la connexion (protocole ou compositeur perdu).
    pub fn error(&self, operation: &'static str) -> crate::Error {
        // SAFETY : connexion valide.
        let code = unsafe { wl_display_get_error(self.raw) };
        crate::Error::Platform { operation, code }
    }
    /// Lie un objet global annoncé par le registre (versions plafonnées à celles écrites).
    fn bind(
        registry: Ptr,
        name: u32,
        interface: &Interface,
        wanted: u32,
        offered: u32,
        kind: Kind,
    ) -> Ptr {
        let version = wanted.min(offered);
        // SAFETY : registre valide ; signature `usun` (nom, interface, version, objet).
        let object = unsafe {
            crate::platform::linux::ffi::wayland::wl_proxy_marshal_flags(
                registry,
                wl_registry::BIND,
                interface,
                version,
                0,
                name,
                interface.name(),
                version,
                NEW,
            )
        };
        listen(object, kind);
        object
    }
    /// Événement du registre : nouvel objet global, ou objet retiré.
    pub fn global(&mut self, event: &Event) {
        use crate::platform::linux::ffi::wayland as w;
        if event.kind != Kind::Registry {
            return;
        }
        if event.opcode == wl_registry::EV_GLOBAL_REMOVE {
            let name = event.uint(0);
            // Sortie débranchée : son objet est oublié (lié en version 2, sans `release`).
            // Les fenêtres l'ont déjà quittée (`wl_surface.leave`) ; un identifiant
            // inconnu qu'elles garderaient ne correspond plus à aucune sortie.
            if let Some(index) = self.globals.outputs.iter().position(|(n, ..)| *n == name) {
                let (_, output, _) = self.globals.outputs.remove(index);
                // SAFETY : objet vivant, oublié ici.
                unsafe { w::wl_proxy_destroy(output) };
            }
            return;
        }
        let (name, interface, version) = (event.uint(0), event.string(1), event.uint(2));
        let registry = self.registry;
        let bind = |interface: &Interface, wanted: u32, kind: Kind| {
            Self::bind(registry, name, interface, wanted, version, kind)
        };
        let g = &mut self.globals;
        // SAFETY : interfaces exportées par libwayland-client (adresses seulement).
        unsafe {
            match interface {
                "wl_compositor" if version >= 4 => {
                    g.compositor = bind(&w::wl_compositor_interface, 6, Kind::Silent);
                }
                "wl_shm" => g.shm = bind(&w::wl_shm_interface, 1, Kind::Silent),
                "xdg_wm_base" => {
                    g.wm_base = bind(&protocol::XDG_WM_BASE, 6, Kind::WmBase);
                }
                "wl_seat" if g.seat.is_none() => {
                    g.seat = Some((name, bind(&w::wl_seat_interface, 8, Kind::Seat)));
                }
                "wl_output" => {
                    let output = bind(&w::wl_output_interface, 2, Kind::Output);
                    g.outputs.push((name, output, 1));
                }
                "wl_data_device_manager" => {
                    g.data_manager = bind(&w::wl_data_device_manager_interface, 3, Kind::Silent);
                }
                "wp_viewporter" => {
                    g.viewporter = bind(&protocol::WP_VIEWPORTER, 1, Kind::Silent);
                }
                "wp_fractional_scale_manager_v1" => {
                    g.fractional = bind(&protocol::WP_FRACTIONAL_SCALE_MANAGER_V1, 1, Kind::Silent);
                }
                "zxdg_decoration_manager_v1" => {
                    g.decoration = bind(&protocol::ZXDG_DECORATION_MANAGER_V1, 1, Kind::Silent);
                }
                "zwp_text_input_manager_v3" => {
                    g.text_input = bind(&protocol::ZWP_TEXT_INPUT_MANAGER_V3, 1, Kind::Silent);
                }
                _ => {}
            }
        }
    }
}
impl Drop for Display {
    fn drop(&mut self) {
        // SAFETY : connexion possédée ; les objets restants sont libérés avec elle.
        unsafe { wl_display_disconnect(self.raw) };
    }
}
/// Chaîne C d'un titre ou d'un identifiant (octets nuls retirés).
pub(super) fn text(value: &str) -> std::ffi::CString {
    c_string(&value.replace('\0', "")).unwrap_or_default()
}
