//! libdbus-1 : connexions privées (bus de session, bus d'accessibilité), messages, et
//! écriture ou lecture de leurs arguments. Les connexions n'ont pas de boucle propre : la
//! boucle du backend surveille leur descripteur et lit les messages sans attendre.
use std::{
    ffi::{CStr, CString, c_char, c_int, c_uint, c_void},
    ptr::NonNull,
};

pub(in crate::platform::linux) const METHOD_CALL: c_int = 1;
pub(in crate::platform::linux) const SIGNAL: c_int = 4;
const SESSION: c_int = 0;
#[repr(C)]
struct RawConnection {
    _private: [u8; 0],
}
#[repr(C)]
struct RawMessage {
    _private: [u8; 0],
}
/// Itérateur de libdbus (`DBusMessageIter`, 72 octets alignés sur 8 : trois pointeurs,
/// onze `int` et deux pointeurs, sur une cible 64 bits).
#[repr(C, align(8))]
struct Iter([u8; 72]);
const _: () = assert!(
    size_of::<usize>() == 8,
    "DBusMessageIter and DBusError are transcribed for 64-bit targets"
);
/// Erreur de libdbus (`DBusError`).
#[repr(C)]
struct Error {
    name: *const c_char,
    message: *const c_char,
    flags: c_uint,
    padding: *mut c_void,
}
#[link(name = "dbus-1")]
unsafe extern "C" {
    fn dbus_error_init(error: *mut Error);
    fn dbus_error_free(error: *mut Error);
    fn dbus_bus_get_private(kind: c_int, error: *mut Error) -> *mut RawConnection;
    fn dbus_connection_open_private(
        address: *const c_char,
        error: *mut Error,
    ) -> *mut RawConnection;
    fn dbus_bus_register(connection: *mut RawConnection, error: *mut Error) -> c_uint;
    fn dbus_bus_get_unique_name(connection: *mut RawConnection) -> *const c_char;
    fn dbus_bus_add_match(connection: *mut RawConnection, rule: *const c_char, error: *mut Error);
    fn dbus_connection_set_exit_on_disconnect(connection: *mut RawConnection, exit: c_uint);
    fn dbus_connection_close(connection: *mut RawConnection);
    fn dbus_connection_unref(connection: *mut RawConnection);
    fn dbus_connection_get_unix_fd(connection: *mut RawConnection, fd: *mut c_int) -> c_uint;
    fn dbus_connection_read_write(connection: *mut RawConnection, timeout: c_int) -> c_uint;
    fn dbus_connection_pop_message(connection: *mut RawConnection) -> *mut RawMessage;
    fn dbus_connection_send(
        connection: *mut RawConnection,
        message: *mut RawMessage,
        serial: *mut u32,
    ) -> c_uint;
    fn dbus_connection_send_with_reply_and_block(
        connection: *mut RawConnection,
        message: *mut RawMessage,
        timeout: c_int,
        error: *mut Error,
    ) -> *mut RawMessage;
    fn dbus_connection_flush(connection: *mut RawConnection);
    fn dbus_message_new_method_call(
        destination: *const c_char,
        path: *const c_char,
        interface: *const c_char,
        method: *const c_char,
    ) -> *mut RawMessage;
    fn dbus_message_new_method_return(call: *mut RawMessage) -> *mut RawMessage;
    fn dbus_message_new_error(
        call: *mut RawMessage,
        name: *const c_char,
        text: *const c_char,
    ) -> *mut RawMessage;
    fn dbus_message_new_signal(
        path: *const c_char,
        interface: *const c_char,
        name: *const c_char,
    ) -> *mut RawMessage;
    fn dbus_message_unref(message: *mut RawMessage);
    fn dbus_message_get_type(message: *mut RawMessage) -> c_int;
    fn dbus_message_get_path(message: *mut RawMessage) -> *const c_char;
    fn dbus_message_get_interface(message: *mut RawMessage) -> *const c_char;
    fn dbus_message_get_member(message: *mut RawMessage) -> *const c_char;
    fn dbus_message_get_no_reply(message: *mut RawMessage) -> c_uint;
    fn dbus_message_iter_init(message: *mut RawMessage, iter: *mut Iter) -> c_uint;
    fn dbus_message_iter_init_append(message: *mut RawMessage, iter: *mut Iter);
    fn dbus_message_iter_append_basic(iter: *mut Iter, kind: c_int, value: *const c_void)
    -> c_uint;
    fn dbus_message_iter_open_container(
        iter: *mut Iter,
        kind: c_int,
        signature: *const c_char,
        sub: *mut Iter,
    ) -> c_uint;
    fn dbus_message_iter_close_container(iter: *mut Iter, sub: *mut Iter) -> c_uint;
    fn dbus_message_iter_get_arg_type(iter: *mut Iter) -> c_int;
    fn dbus_message_iter_get_basic(iter: *mut Iter, value: *mut c_void);
    fn dbus_message_iter_next(iter: *mut Iter) -> c_uint;
    fn dbus_message_iter_recurse(iter: *mut Iter, sub: *mut Iter);
}
/// Erreur locale, initialisée et libérée autour d'un appel ; son texte (`nom : message`)
/// en cas d'échec.
fn with_error_text<R>(f: impl FnOnce(*mut Error) -> R) -> (R, Option<String>) {
    let mut error = Error {
        name: std::ptr::null(),
        message: std::ptr::null(),
        flags: 0,
        padding: std::ptr::null_mut(),
    };
    // SAFETY : erreur locale valide pendant l'appel, libérée ensuite.
    unsafe { dbus_error_init(&mut error) };
    let result = f(&mut error);
    let failure = (!error.name.is_null()).then(|| {
        let name = text(error.name).unwrap_or_default();
        match text(error.message) {
            Some(message) => format!("{name}: {message}"),
            None => name,
        }
    });
    // SAFETY : idem.
    unsafe { dbus_error_free(&mut error) };
    (result, failure)
}
fn with_error<R>(f: impl FnOnce(*mut Error) -> R) -> (R, bool) {
    let (result, failure) = with_error_text(f);
    (result, failure.is_some())
}
fn text(value: *const c_char) -> Option<String> {
    // SAFETY : chaîne possédée par le message, copiée aussitôt.
    (!value.is_null()).then(|| {
        unsafe { CStr::from_ptr(value) }
            .to_string_lossy()
            .into_owned()
    })
}
fn c(value: &str) -> CString {
    CString::new(value.replace('\0', "")).unwrap_or_default()
}
/// Connexion privée possédée.
pub(in crate::platform::linux) struct Connection(NonNull<RawConnection>);
impl Connection {
    fn own(raw: *mut RawConnection) -> Option<Self> {
        let connection = Self(NonNull::new(raw)?);
        // SAFETY : connexion valide ; une déconnexion ne doit pas quitter le processus.
        unsafe { dbus_connection_set_exit_on_disconnect(connection.raw(), 0) };
        Some(connection)
    }
    fn raw(&self) -> *mut RawConnection {
        self.0.as_ptr()
    }
    /// Bus de session de l'utilisateur.
    pub fn session() -> Option<Self> {
        // SAFETY : connexion privée possédée par le résultat.
        let (raw, failed) = with_error(|e| unsafe { dbus_bus_get_private(SESSION, e) });
        if failed {
            return None;
        }
        Self::own(raw)
    }
    /// Bus d'adresse `address` (bus d'accessibilité), enregistré auprès de son démon.
    pub fn open(address: &str) -> Option<Self> {
        let address = c(address);
        // SAFETY : adresse lue pendant l'appel ; connexion possédée.
        let (raw, failed) =
            with_error(|e| unsafe { dbus_connection_open_private(address.as_ptr(), e) });
        if failed {
            return None;
        }
        let connection = Self::own(raw)?;
        // SAFETY : connexion valide.
        let (registered, failed) =
            with_error(|e| unsafe { dbus_bus_register(connection.raw(), e) });
        (registered != 0 && !failed).then_some(connection)
    }
    /// Nom unique de la connexion sur son bus (`:1.42`).
    pub fn unique_name(&self) -> String {
        // SAFETY : connexion enregistrée ; chaîne possédée par la connexion.
        text(unsafe { dbus_bus_get_unique_name(self.raw()) }).unwrap_or_default()
    }
    pub fn add_match(&self, rule: &str) {
        let rule = c(rule);
        // SAFETY : règle lue pendant l'appel.
        with_error(|e| unsafe { dbus_bus_add_match(self.raw(), rule.as_ptr(), e) });
    }
    pub fn fd(&self) -> Option<c_int> {
        let mut fd = -1;
        // SAFETY : sortie locale.
        (unsafe { dbus_connection_get_unix_fd(self.raw(), &mut fd) } != 0 && fd >= 0).then_some(fd)
    }
    /// Appel bloquant (au plus `timeout` ms) ; `None` en cas d'erreur ou de délai.
    pub fn call(&self, message: &Message, timeout: c_int) -> Option<Message> {
        self.request(message, timeout).ok()
    }
    /// Appel bloquant (au plus `timeout` ms) ; l'erreur D-Bus en texte en cas d'échec.
    pub fn request(&self, message: &Message, timeout: c_int) -> Result<Message, String> {
        // SAFETY : connexion et message valides ; réponse possédée.
        let (reply, failure) = with_error_text(|e| unsafe {
            dbus_connection_send_with_reply_and_block(self.raw(), message.0.as_ptr(), timeout, e)
        });
        if let Some(failure) = failure {
            return Err(failure);
        }
        NonNull::new(reply)
            .map(Message)
            .ok_or_else(|| "no reply".to_owned())
    }
    pub fn send(&self, message: &Message) {
        // SAFETY : connexion et message valides.
        unsafe { dbus_connection_send(self.raw(), message.0.as_ptr(), std::ptr::null_mut()) };
    }
    /// Lit et écrit ce qui est prêt, sans attendre.
    pub fn pump(&self) {
        // SAFETY : connexion valide ; délai nul.
        unsafe { dbus_connection_read_write(self.raw(), 0) };
    }
    /// Attend au plus `timeout` ms que des messages arrivent (thread propre à la
    /// connexion) ; `false` quand la connexion est perdue.
    pub fn wait(&self, timeout: c_int) -> bool {
        // SAFETY : connexion valide.
        unsafe { dbus_connection_read_write(self.raw(), timeout) != 0 }
    }
    pub fn pop(&self) -> Option<Message> {
        // SAFETY : connexion valide ; message possédé.
        NonNull::new(unsafe { dbus_connection_pop_message(self.raw()) }).map(Message)
    }
    pub fn flush(&self) {
        // SAFETY : connexion valide.
        unsafe { dbus_connection_flush(self.raw()) };
    }
}
impl Drop for Connection {
    fn drop(&mut self) {
        // SAFETY : connexion privée possédée : fermée puis rendue.
        unsafe {
            dbus_connection_close(self.raw());
            dbus_connection_unref(self.raw());
        }
    }
}
/// Message possédé.
pub(in crate::platform::linux) struct Message(NonNull<RawMessage>);
impl Message {
    pub fn method_call(
        destination: &str,
        path: &str,
        interface: &str,
        method: &str,
    ) -> Option<Self> {
        let (d, p, i, m) = (c(destination), c(path), c(interface), c(method));
        // SAFETY : chaînes lues pendant l'appel ; message possédé.
        NonNull::new(unsafe {
            dbus_message_new_method_call(d.as_ptr(), p.as_ptr(), i.as_ptr(), m.as_ptr())
        })
        .map(Self)
    }
    pub fn signal(path: &str, interface: &str, name: &str) -> Option<Self> {
        let (p, i, n) = (c(path), c(interface), c(name));
        // SAFETY : idem.
        NonNull::new(unsafe { dbus_message_new_signal(p.as_ptr(), i.as_ptr(), n.as_ptr()) })
            .map(Self)
    }
    /// Réponse à cet appel ; `None` si l'appelant n'en attend pas.
    pub fn reply(&self) -> Option<Self> {
        // SAFETY : message valide.
        if unsafe { dbus_message_get_no_reply(self.0.as_ptr()) } != 0 {
            return None;
        }
        // SAFETY : appel valide ; réponse possédée.
        NonNull::new(unsafe { dbus_message_new_method_return(self.0.as_ptr()) }).map(Self)
    }
    /// Réponse d'erreur ; `None` si l'appelant n'attend pas de réponse.
    pub fn error(&self, name: &str, description: &str) -> Option<Self> {
        // SAFETY : message valide.
        if unsafe { dbus_message_get_no_reply(self.0.as_ptr()) } != 0 {
            return None;
        }
        let (n, t) = (c(name), c(description));
        // SAFETY : appel valide ; chaînes lues pendant l'appel.
        NonNull::new(unsafe { dbus_message_new_error(self.0.as_ptr(), n.as_ptr(), t.as_ptr()) })
            .map(Self)
    }
    pub fn kind(&self) -> c_int {
        // SAFETY : message valide.
        unsafe { dbus_message_get_type(self.0.as_ptr()) }
    }
    pub fn path(&self) -> String {
        // SAFETY : message valide.
        text(unsafe { dbus_message_get_path(self.0.as_ptr()) }).unwrap_or_default()
    }
    pub fn interface(&self) -> String {
        // SAFETY : idem.
        text(unsafe { dbus_message_get_interface(self.0.as_ptr()) }).unwrap_or_default()
    }
    pub fn member(&self) -> String {
        // SAFETY : idem.
        text(unsafe { dbus_message_get_member(self.0.as_ptr()) }).unwrap_or_default()
    }
    pub fn writer(&mut self) -> Writer<'_> {
        let mut root = Box::new(Iter([0; 72]));
        // SAFETY : message possédé ; itérateur à adresse stable (boîte).
        unsafe { dbus_message_iter_init_append(self.0.as_ptr(), &mut *root) };
        Writer {
            stack: vec![(root, true)],
            _message: std::marker::PhantomData,
        }
    }
    pub fn reader(&self) -> Reader<'_> {
        let mut iter = Box::new(Iter([0; 72]));
        // SAFETY : message valide ; itérateur à adresse stable.
        let any = unsafe { dbus_message_iter_init(self.0.as_ptr(), &mut *iter) } != 0;
        Reader {
            iter,
            done: !any,
            _message: std::marker::PhantomData,
        }
    }
}
impl Drop for Message {
    fn drop(&mut self) {
        // SAFETY : référence possédée, rendue une fois.
        unsafe { dbus_message_unref(self.0.as_ptr()) };
    }
}
/// Écriture des arguments d'un message ; les conteneurs ouverts sont empilés, avec un
/// drapeau : `false` si libdbus n'a pas pu l'ouvrir (il n'est alors pas refermé).
pub(in crate::platform::linux) struct Writer<'a> {
    // Boîtes : libdbus garde l'adresse d'un itérateur ouvert, qui ne doit pas bouger
    // quand la pile grandit.
    stack: Vec<(Box<Iter>, bool)>,
    _message: std::marker::PhantomData<&'a mut Message>,
}
impl Writer<'_> {
    fn top(&mut self) -> *mut Iter {
        &mut *self.stack.last_mut().expect("root iterator").0
    }
    fn basic(&mut self, kind: u8, value: *const c_void) -> &mut Self {
        // SAFETY : itérateur ouvert ; valeur du type `kind` lue pendant l'appel.
        unsafe { dbus_message_iter_append_basic(self.top(), c_int::from(kind), value) };
        self
    }
    pub fn string(&mut self, value: &str) -> &mut Self {
        let value = c(value);
        let pointer = value.as_ptr();
        self.basic(b's', (&raw const pointer).cast())
    }
    pub fn path(&mut self, value: &str) -> &mut Self {
        let value = c(value);
        let pointer = value.as_ptr();
        self.basic(b'o', (&raw const pointer).cast())
    }
    pub fn byte(&mut self, value: u8) -> &mut Self {
        self.basic(b'y', (&raw const value).cast())
    }
    pub fn i32(&mut self, value: i32) -> &mut Self {
        self.basic(b'i', (&raw const value).cast())
    }
    pub fn u32(&mut self, value: u32) -> &mut Self {
        self.basic(b'u', (&raw const value).cast())
    }
    pub fn i16(&mut self, value: i16) -> &mut Self {
        self.basic(b'n', (&raw const value).cast())
    }
    pub fn f64(&mut self, value: f64) -> &mut Self {
        self.basic(b'd', (&raw const value).cast())
    }
    pub fn bool(&mut self, value: bool) -> &mut Self {
        let value = c_uint::from(value);
        self.basic(b'b', (&raw const value).cast())
    }
    /// Ouvre un conteneur : tableau (`a`, signature des éléments), structure (`r`),
    /// entrée de dictionnaire (`e`) ou variante (`v`, signature du contenu).
    pub fn open(&mut self, kind: u8, signature: Option<&str>) -> &mut Self {
        let signature = signature.map(c);
        let mut sub = Box::new(Iter([0; 72]));
        // SAFETY : itérateur parent ouvert ; enfant à adresse stable, fermé par `close`.
        let opened = unsafe {
            dbus_message_iter_open_container(
                self.top(),
                c_int::from(kind),
                signature.as_ref().map_or(std::ptr::null(), |s| s.as_ptr()),
                &mut *sub,
            )
        } != 0;
        // Un échec (mémoire) laisse l'enfant non initialisé : `close` ne le refermera pas.
        self.stack.push((sub, opened));
        self
    }
    pub fn close(&mut self) -> &mut Self {
        if self.stack.len() > 1 {
            let (mut sub, opened) = self.stack.pop().expect("open container");
            if opened {
                // SAFETY : enfant ouvert par `open` sur le parent courant.
                unsafe { dbus_message_iter_close_container(self.top(), &mut *sub) };
            }
        }
        self
    }
    /// Référence d'objet AT-SPI `(so)` : nom de bus et chemin.
    pub fn reference(&mut self, bus: &str, path: &str) -> &mut Self {
        self.open(b'r', None).string(bus).path(path).close()
    }
}
/// Lecture des arguments d'un message, dans l'ordre.
pub(in crate::platform::linux) struct Reader<'a> {
    iter: Box<Iter>,
    done: bool,
    _message: std::marker::PhantomData<&'a Message>,
}
impl Reader<'_> {
    /// Type de l'argument courant (`0` à la fin).
    pub fn kind(&mut self) -> u8 {
        if self.done {
            return 0;
        }
        // SAFETY : itérateur valide.
        unsafe { dbus_message_iter_get_arg_type(&mut *self.iter) as u8 }
    }
    fn advance(&mut self) {
        // SAFETY : itérateur valide.
        self.done = unsafe { dbus_message_iter_next(&mut *self.iter) } == 0;
    }
    fn basic<T: Copy>(&mut self, kind: u8, zero: T) -> Option<T> {
        if self.kind() != kind {
            return None;
        }
        let mut value = zero;
        // SAFETY : argument du type `kind`, écrit dans une valeur de même taille.
        unsafe { dbus_message_iter_get_basic(&mut *self.iter, (&raw mut value).cast()) };
        self.advance();
        Some(value)
    }
    pub fn string(&mut self) -> Option<String> {
        let kind = self.kind();
        if !matches!(kind, b's' | b'o') {
            return None;
        }
        text(self.basic::<*const c_char>(kind, std::ptr::null())?)
    }
    pub fn i32(&mut self) -> Option<i32> {
        self.basic(b'i', 0i32)
    }
    pub fn u32(&mut self) -> Option<u32> {
        self.basic(b'u', 0u32)
    }
    pub fn f64(&mut self) -> Option<f64> {
        self.basic(b'd', 0f64)
    }
    pub fn bool(&mut self) -> Option<bool> {
        // `dbus_bool_t` : entier de 32 bits.
        self.basic(b'b', 0u32).map(|v| v != 0)
    }
    /// Passe l'argument courant, quel que soit son type.
    pub fn skip(&mut self) {
        if !self.done {
            self.advance();
        }
    }
    /// Entre dans le conteneur courant (structure, variante, tableau) et passe au suivant.
    pub fn enter(&mut self) -> Option<Reader<'_>> {
        if !matches!(self.kind(), b'r' | b'v' | b'a' | b'e') {
            return None;
        }
        let mut sub = Box::new(Iter([0; 72]));
        // SAFETY : itérateur sur un conteneur ; enfant à adresse stable.
        unsafe { dbus_message_iter_recurse(&mut *self.iter, &mut *sub) };
        self.advance();
        // SAFETY : enfant valide.
        let done = unsafe { dbus_message_iter_get_arg_type(&mut *sub) } == 0;
        Some(Reader {
            iter: sub,
            done,
            _message: std::marker::PhantomData,
        })
    }
}
