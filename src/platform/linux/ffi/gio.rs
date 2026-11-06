//! GIO : réglages du bureau (GSettings, base dconf de GNOME, Cinnamon, MATE…). Un schéma
//! absent est cherché d'abord (`g_settings_new` sur un schéma inconnu interrompt le
//! processus), de même qu'un schéma sans chemin (relocalisable) et une clé absente.
use super::glib::{Gboolean, Object};
use std::ffi::{CStr, c_char, c_void};

#[link(name = "gio-2.0")]
unsafe extern "C" {
    fn g_settings_schema_source_get_default() -> *mut c_void;
    fn g_settings_schema_source_lookup(
        source: *mut c_void,
        schema_id: *const c_char,
        recursive: Gboolean,
    ) -> *mut c_void;
    fn g_settings_schema_has_key(schema: *mut c_void, name: *const c_char) -> Gboolean;
    fn g_settings_schema_get_path(schema: *mut c_void) -> *const c_char;
    fn g_settings_schema_unref(schema: *mut c_void);
    fn g_settings_new_full(
        schema: *mut c_void,
        backend: *mut c_void,
        path: *const c_char,
    ) -> *mut c_void;
    fn g_settings_get_string(settings: *mut c_void, key: *const c_char) -> *mut c_char;
    fn g_settings_get_boolean(settings: *mut c_void, key: *const c_char) -> Gboolean;
}
#[link(name = "glib-2.0")]
unsafe extern "C" {
    fn g_free(memory: *mut c_void);
}
/// Réglages d'un schéma installé.
pub(in crate::platform::linux) struct Settings {
    /// GObject `GSettings` (libéré après le schéma, qu'il référence).
    settings: Object,
    schema: *mut c_void,
}
impl Settings {
    /// Réglages du schéma `id` ; `None` s'il n'est pas installé ou s'il n'a pas de chemin
    /// (un schéma relocalisable ouvert sans chemin interromprait le processus).
    pub fn open(id: &CStr) -> Option<Self> {
        // SAFETY : source par défaut possédée par GIO ; schéma possédé, libéré par `Drop`
        // ou ci-dessous ; réglages possédés par `Object`.
        unsafe {
            let source = g_settings_schema_source_get_default();
            if source.is_null() {
                return None;
            }
            let schema = g_settings_schema_source_lookup(source, id.as_ptr(), 1);
            if schema.is_null() {
                return None;
            }
            if g_settings_schema_get_path(schema).is_null() {
                g_settings_schema_unref(schema);
                return None;
            }
            let settings = Object::from_full(g_settings_new_full(
                schema,
                std::ptr::null_mut(),
                std::ptr::null(),
            ));
            let Some(settings) = settings else {
                g_settings_schema_unref(schema);
                return None;
            };
            Some(Self { settings, schema })
        }
    }
    fn has(&self, key: &CStr) -> bool {
        // SAFETY : schéma valide ; nom terminé par un octet nul.
        unsafe { g_settings_schema_has_key(self.schema, key.as_ptr()) != 0 }
    }
    /// Valeur texte de `key`, si la clé existe.
    pub fn string(&self, key: &CStr) -> Option<String> {
        if !self.has(key) {
            return None;
        }
        // SAFETY : clé du schéma ; chaîne possédée, libérée ici.
        unsafe {
            let raw = g_settings_get_string(self.settings.as_ptr(), key.as_ptr());
            if raw.is_null() {
                return None;
            }
            let value = CStr::from_ptr(raw).to_string_lossy().into_owned();
            g_free(raw.cast());
            Some(value)
        }
    }
    /// Valeur booléenne de `key`, si la clé existe.
    pub fn boolean(&self, key: &CStr) -> Option<bool> {
        // SAFETY : clé du schéma.
        self.has(key)
            .then(|| unsafe { g_settings_get_boolean(self.settings.as_ptr(), key.as_ptr()) != 0 })
    }
}
impl Drop for Settings {
    fn drop(&mut self) {
        // SAFETY : schéma possédé, libéré une fois (les réglages, qui le référencent,
        // sont libérés ensuite par `Object`).
        unsafe { g_settings_schema_unref(self.schema) };
    }
}
