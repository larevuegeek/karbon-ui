//! Secrets dans le trousseau du bureau (Secret Service : GNOME Keyring, KWallet…) par
//! libsecret, chargée au lancement (`dlopen`) : sans elle, `secrets` rend `Unsupported`.
//! Attributs de l'élément : `service` et `key`.
use super::ffi::{
    c_string, failure,
    glib::{GError, clear_error},
};
use std::ffi::{CString, c_char, c_int, c_void};
use std::sync::OnceLock;

#[repr(C)]
struct SchemaAttribute {
    name: *const c_char,
    kind: c_int,
}
/// `SecretSchema` (secret-schema.h), champs réservés compris.
#[repr(C)]
struct Schema {
    name: *const c_char,
    flags: c_int,
    attributes: [SchemaAttribute; 32],
    reserved: c_int,
    reserved_pointers: [*mut c_void; 7],
}

type Store = unsafe extern "C" fn(
    *const Schema,
    *mut c_void,
    *const c_char,
    *const c_char,
    *const c_char,
    *mut c_void,
    *mut *mut GError,
) -> c_int;
type Lookup =
    unsafe extern "C" fn(*const Schema, *mut c_void, *mut c_void, *mut *mut GError) -> *mut c_char;
type Clear =
    unsafe extern "C" fn(*const Schema, *mut c_void, *mut c_void, *mut *mut GError) -> c_int;
type Free = unsafe extern "C" fn(*mut c_char);

struct Library {
    store: Store,
    lookup: Lookup,
    clear: Clear,
    free: Free,
}

#[link(name = "glib-2.0")]
unsafe extern "C" {
    fn g_hash_table_new(hash: *const c_void, equal: *const c_void) -> *mut c_void;
    fn g_hash_table_insert(table: *mut c_void, key: *mut c_void, value: *mut c_void) -> c_int;
    fn g_hash_table_unref(table: *mut c_void);
    fn g_str_hash(value: *const c_void) -> u32;
    fn g_str_equal(a: *const c_void, b: *const c_void) -> c_int;
}

fn library() -> crate::Result<&'static Library> {
    static LIBRARY: OnceLock<Option<Library>> = OnceLock::new();
    LIBRARY
        .get_or_init(|| {
            // SAFETY : chargement d'une bibliothèque système ; les symboles ont les
            // signatures de libsecret 0.18+.
            unsafe {
                let handle = libc::dlopen(
                    c"libsecret-1.so.0".as_ptr(),
                    libc::RTLD_NOW | libc::RTLD_LOCAL,
                );
                if handle.is_null() {
                    return None;
                }
                let symbol = |name: &std::ffi::CStr| {
                    let address = libc::dlsym(handle, name.as_ptr());
                    (!address.is_null()).then_some(address)
                };
                Some(Library {
                    store: std::mem::transmute::<*mut c_void, Store>(symbol(
                        c"secret_password_storev_sync",
                    )?),
                    lookup: std::mem::transmute::<*mut c_void, Lookup>(symbol(
                        c"secret_password_lookupv_sync",
                    )?),
                    clear: std::mem::transmute::<*mut c_void, Clear>(symbol(
                        c"secret_password_clearv_sync",
                    )?),
                    free: std::mem::transmute::<*mut c_void, Free>(symbol(
                        c"secret_password_free",
                    )?),
                })
            }
        })
        .as_ref()
        .ok_or(crate::Error::Unsupported {
            reason: "libsecret (Secret Service) is not installed",
        })
}

/// Schéma et attributs d'une recherche ; les chaînes vivent avec lui.
struct Query {
    schema: Schema,
    attributes: *mut c_void,
    _strings: [CString; 5],
}
impl Query {
    fn new(service: &str, key: &str) -> crate::Result<Self> {
        let strings = [
            c"dev.karbon-ui.Secret".to_owned(),
            c"service".to_owned(),
            c"key".to_owned(),
            c_string(service)?,
            c_string(key)?,
        ];
        let empty = || SchemaAttribute {
            name: std::ptr::null(),
            kind: 0,
        };
        let mut attributes: [SchemaAttribute; 32] = std::array::from_fn(|_| empty());
        attributes[0].name = strings[1].as_ptr();
        attributes[1].name = strings[2].as_ptr();
        // SAFETY : table de chaînes de GLib ; clés et valeurs empruntées à `strings`, qui
        // vit avec la table.
        let table = unsafe {
            let table = g_hash_table_new(g_str_hash as *const c_void, g_str_equal as *const c_void);
            g_hash_table_insert(
                table,
                strings[1].as_ptr() as *mut c_void,
                strings[3].as_ptr() as *mut c_void,
            );
            g_hash_table_insert(
                table,
                strings[2].as_ptr() as *mut c_void,
                strings[4].as_ptr() as *mut c_void,
            );
            table
        };
        Ok(Self {
            schema: Schema {
                name: strings[0].as_ptr(),
                flags: 0,
                attributes,
                reserved: 0,
                reserved_pointers: [std::ptr::null_mut(); 7],
            },
            attributes: table,
            _strings: strings,
        })
    }
}
impl Drop for Query {
    fn drop(&mut self) {
        // SAFETY : table créée par `new`, libérée une fois.
        unsafe { g_hash_table_unref(self.attributes) };
    }
}

fn check(error: *mut GError, operation: &'static str) -> crate::Result<()> {
    if error.is_null() {
        return Ok(());
    }
    clear_error(error);
    Err(failure(operation))
}

pub(crate) fn set(service: &str, key: &str, secret: &str) -> crate::Result<()> {
    let library = library()?;
    let query = Query::new(service, key)?;
    let label = c_string(&format!("{service}: {key}"))?;
    let password = c_string(secret)?;
    let mut error = std::ptr::null_mut();
    // SAFETY : schéma, attributs et chaînes valides pendant l'appel synchrone.
    let ok = unsafe {
        (library.store)(
            &query.schema,
            query.attributes,
            std::ptr::null(),
            label.as_ptr(),
            password.as_ptr(),
            std::ptr::null_mut(),
            &mut error,
        )
    };
    check(error, "secret_password_storev_sync")?;
    if ok == 0 {
        return Err(failure("secret_password_storev_sync"));
    }
    Ok(())
}

pub(crate) fn get(service: &str, key: &str) -> crate::Result<Option<String>> {
    let library = library()?;
    let query = Query::new(service, key)?;
    let mut error = std::ptr::null_mut();
    // SAFETY : schéma et attributs valides pendant l'appel synchrone.
    let value = unsafe {
        (library.lookup)(
            &query.schema,
            query.attributes,
            std::ptr::null_mut(),
            &mut error,
        )
    };
    check(error, "secret_password_lookupv_sync")?;
    if value.is_null() {
        return Ok(None);
    }
    // SAFETY : chaîne C rendue par libsecret, libérée par sa fonction.
    let text = unsafe {
        let text = std::ffi::CStr::from_ptr(value).to_str().map(str::to_owned);
        (library.free)(value);
        text
    };
    text.map(Some)
        .map_err(|_| crate::Error::invalid_input("stored secret is not UTF-8"))
}

pub(crate) fn remove(service: &str, key: &str) -> crate::Result<()> {
    let library = library()?;
    let query = Query::new(service, key)?;
    let mut error = std::ptr::null_mut();
    // SAFETY : schéma et attributs valides pendant l'appel synchrone ; « rien à
    // supprimer » rend faux sans erreur.
    unsafe {
        (library.clear)(
            &query.schema,
            query.attributes,
            std::ptr::null_mut(),
            &mut error,
        )
    };
    check(error, "secret_password_clearv_sync")
}
