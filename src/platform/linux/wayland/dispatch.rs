//! Événements du protocole : nature de chaque objet, répartiteur unique attaché par
//! libwayland (frontière C : décode les arguments d'après la signature du message et met
//! l'événement en file) et file lue par la boucle hors de tout appel à libwayland.
use super::display::Ptr;
use crate::platform::linux::ffi::wayland::{Argument, Message, wl_proxy_add_dispatcher};
use std::{
    cell::RefCell,
    ffi::{CStr, c_char, c_int, c_void},
    os::fd::{FromRawFd, OwnedFd},
};

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
    /// Fenêtre exportée (`xdg-foreign`) : sa poignée.
    Exported,
    /// Retour d'affichage d'une image (`wp_presentation`, sonde).
    Feedback,
    /// Objet sans événement utile (région, viewport, pool).
    Silent,
}
const KINDS: [Kind; 20] = [
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
    Kind::Exported,
    Kind::Feedback,
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
