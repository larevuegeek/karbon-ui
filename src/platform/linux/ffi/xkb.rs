//! xkbcommon : disposition du clavier (envoyée par le compositeur Wayland, lue du serveur
//! X par xkbcommon-x11, ou compilée d'après ses noms), état des modificateurs, symboles et
//! texte des touches, séquences de composition (touches mortes, `Compose`).
use super::required;
use std::{
    ffi::{CStr, c_char, c_int},
    ptr::NonNull,
};

pub(in crate::platform::linux) type Keysym = u32;
const KEYMAP_FORMAT_TEXT_V1: c_int = 1;
const STATE_MODS_EFFECTIVE: c_int = 1 << 3;
const COMPOSE_FEED_ACCEPTED: c_int = 1;
const COMPOSE_COMPOSING: c_int = 1;
const COMPOSE_COMPOSED: c_int = 2;
const COMPOSE_CANCELLED: c_int = 3;

#[link(name = "xkbcommon")]
unsafe extern "C" {
    fn xkb_context_new(flags: c_int) -> *mut Raw;
    fn xkb_context_unref(context: *mut Raw);
    fn xkb_keymap_new_from_string(
        context: *mut Raw,
        string: *const c_char,
        format: c_int,
        flags: c_int,
    ) -> *mut Raw;
    fn xkb_keymap_new_from_names(
        context: *mut Raw,
        names: *const RuleNames,
        flags: c_int,
    ) -> *mut Raw;
    fn xkb_keymap_unref(keymap: *mut Raw);
    fn xkb_keymap_key_repeats(keymap: *mut Raw, key: u32) -> c_int;
    fn xkb_keymap_num_layouts(keymap: *mut Raw) -> u32;
    fn xkb_keymap_key_get_syms_by_level(
        keymap: *mut Raw,
        key: u32,
        layout: u32,
        level: u32,
        syms_out: *mut *const Keysym,
    ) -> c_int;
    fn xkb_state_new(keymap: *mut Raw) -> *mut Raw;
    fn xkb_state_unref(state: *mut Raw);
    fn xkb_state_update_mask(
        state: *mut Raw,
        depressed: u32,
        latched: u32,
        locked: u32,
        depressed_layout: u32,
        latched_layout: u32,
        locked_layout: u32,
    ) -> c_int;
    fn xkb_state_key_get_one_sym(state: *mut Raw, key: u32) -> Keysym;
    fn xkb_state_key_get_utf8(state: *mut Raw, key: u32, buffer: *mut c_char, size: usize)
    -> c_int;
    fn xkb_state_mod_name_is_active(state: *mut Raw, name: *const c_char, kind: c_int) -> c_int;
    fn xkb_compose_table_new_from_locale(
        context: *mut Raw,
        locale: *const c_char,
        flags: c_int,
    ) -> *mut Raw;
    fn xkb_compose_table_unref(table: *mut Raw);
    fn xkb_compose_state_new(table: *mut Raw, flags: c_int) -> *mut Raw;
    fn xkb_compose_state_unref(state: *mut Raw);
    fn xkb_compose_state_feed(state: *mut Raw, keysym: Keysym) -> c_int;
    fn xkb_compose_state_get_status(state: *mut Raw) -> c_int;
    fn xkb_compose_state_get_utf8(state: *mut Raw, buffer: *mut c_char, size: usize) -> c_int;
    fn xkb_compose_state_reset(state: *mut Raw);
}
#[repr(C)]
struct Raw {
    _private: [u8; 0],
}
/// Noms d'une disposition (`struct xkb_rule_names`) ; nul : valeur par défaut.
#[repr(C)]
struct RuleNames {
    rules: *const c_char,
    model: *const c_char,
    layout: *const c_char,
    variant: *const c_char,
    options: *const c_char,
}
/// Objet xkbcommon possédé, libéré par sa fonction `unref`.
struct Owned(NonNull<Raw>, unsafe extern "C" fn(*mut Raw));
impl Owned {
    fn new(
        raw: *mut Raw,
        unref: unsafe extern "C" fn(*mut Raw),
        operation: &'static str,
    ) -> crate::Result<Self> {
        Ok(Self(required(raw, operation)?, unref))
    }
    fn raw(&self) -> *mut Raw {
        self.0.as_ptr()
    }
}
impl Drop for Owned {
    fn drop(&mut self) {
        // SAFETY : référence possédée, rendue une fois.
        unsafe { (self.1)(self.0.as_ptr()) };
    }
}
/// Résultat de la composition pour une touche.
pub(in crate::platform::linux) enum Composed {
    /// Touche hors séquence : traitée normalement.
    Nothing,
    /// Touche absorbée par une séquence en cours ou annulée.
    Absorbed,
    /// Séquence terminée : texte produit.
    Text(String),
}
/// Clavier : contexte, disposition, état et composition (absente si la locale n'a pas de
/// table de composition).
pub(in crate::platform::linux) struct Keyboard {
    compose: Option<Owned>,
    _table: Option<Owned>,
    state: Owned,
    keymap: Owned,
    _context: Owned,
}
impl Keyboard {
    /// Disposition décrite par `text` (format XKB envoyé par `wl_keyboard.keymap`).
    pub fn new(text: &CStr, locale: &CStr) -> crate::Result<Self> {
        // SAFETY : contexte valide ; texte lu pendant l'appel.
        Self::build(locale, |context| unsafe {
            xkb_keymap_new_from_string(context, text.as_ptr(), KEYMAP_FORMAT_TEXT_V1, 0)
        })
    }
    /// Disposition compilée d'après ses noms (règles, modèle, dispositions, variantes,
    /// options : propriété `_XKB_RULES_NAMES` du serveur X) ; un nom vide prend la valeur
    /// par défaut.
    pub fn from_names(names: [&CStr; 5], locale: &CStr) -> crate::Result<Self> {
        let name = |n: &CStr| {
            if n.is_empty() {
                std::ptr::null()
            } else {
                n.as_ptr()
            }
        };
        let names = RuleNames {
            rules: name(names[0]),
            model: name(names[1]),
            layout: name(names[2]),
            variant: name(names[3]),
            options: name(names[4]),
        };
        // SAFETY : contexte valide ; noms lus pendant l'appel.
        Self::build(locale, |context| unsafe {
            xkb_keymap_new_from_names(context, &names, 0)
        })
    }
    /// Disposition rendue par `keymap` à partir d'un contexte xkbcommon (pointeur opaque
    /// `struct xkb_context *`), par exemple `xkb_x11_keymap_new_from_device`.
    pub fn from_context(
        locale: &CStr,
        keymap: impl FnOnce(*mut std::ffi::c_void) -> *mut std::ffi::c_void,
    ) -> crate::Result<Self> {
        Self::build(locale, |context| keymap(context.cast()).cast())
    }
    /// Contexte, disposition rendue par `keymap`, état et composition de `locale`.
    fn build(locale: &CStr, keymap: impl FnOnce(*mut Raw) -> *mut Raw) -> crate::Result<Self> {
        // SAFETY : constructeurs ; objets possédés, libérés par `Owned`.
        unsafe {
            let context = Owned::new(xkb_context_new(0), xkb_context_unref, "xkb_context_new")?;
            let keymap = Owned::new(keymap(context.raw()), xkb_keymap_unref, "xkb_keymap_new")?;
            let state = Owned::new(
                xkb_state_new(keymap.raw()),
                xkb_state_unref,
                "xkb_state_new",
            )?;
            let table = Owned::new(
                xkb_compose_table_new_from_locale(context.raw(), locale.as_ptr(), 0),
                xkb_compose_table_unref,
                "xkb_compose_table_new_from_locale",
            )
            .ok();
            let compose = table.as_ref().and_then(|t| {
                Owned::new(
                    xkb_compose_state_new(t.raw(), 0),
                    xkb_compose_state_unref,
                    "xkb_compose_state_new",
                )
                .ok()
            });
            Ok(Self {
                compose,
                _table: table,
                state,
                keymap,
                _context: context,
            })
        }
    }
    pub fn update(&self, depressed: u32, latched: u32, locked: u32, group: u32) {
        // SAFETY : état valide ; masques par valeur.
        unsafe { xkb_state_update_mask(self.state.raw(), depressed, latched, locked, 0, 0, group) };
    }
    /// Symbole de la touche `key` (code XKB = code evdev + 8) dans l'état courant.
    pub fn keysym(&self, key: u32) -> Keysym {
        // SAFETY : état valide.
        unsafe { xkb_state_key_get_one_sym(self.state.raw(), key) }
    }
    pub fn repeats(&self, key: u32) -> bool {
        // SAFETY : disposition valide.
        unsafe { xkb_keymap_key_repeats(self.keymap.raw(), key) != 0 }
    }
    /// Symbole latin (ASCII imprimable) de la touche `key` au premier niveau d'une de ses
    /// dispositions, quelle que soit celle en cours : les raccourcis d'édition valent par
    /// la touche physique sous une disposition cyrillique, grecque… `None` si aucun.
    pub fn latin_keysym(&self, key: u32) -> Option<Keysym> {
        // SAFETY : disposition valide ; `syms` désigne un tableau de la disposition,
        // valide tant qu'elle vit, lu aussitôt.
        unsafe {
            (0..xkb_keymap_num_layouts(self.keymap.raw())).find_map(|layout| {
                let mut syms: *const Keysym = std::ptr::null();
                let count =
                    xkb_keymap_key_get_syms_by_level(self.keymap.raw(), key, layout, 0, &mut syms);
                (count > 0 && !syms.is_null())
                    .then(|| *syms)
                    .filter(|sym| (0x20..=0x7e).contains(sym))
            })
        }
    }
    /// Texte produit par la touche (vide pour une touche de fonction).
    pub fn text(&self, key: u32) -> String {
        let mut buffer = [0u8; 64];
        // SAFETY : tampon local de 64 octets, terminé par un octet nul par xkbcommon.
        let n = unsafe {
            xkb_state_key_get_utf8(
                self.state.raw(),
                key,
                buffer.as_mut_ptr().cast(),
                buffer.len(),
            )
        };
        String::from_utf8_lossy(&buffer[..(n.max(0) as usize).min(buffer.len() - 1)]).into_owned()
    }
    /// Modificateur actif : `"Control"`, `"Mod1"` (Alt), `"Shift"`, `"Mod4"` (Super).
    pub fn active(&self, name: &CStr) -> bool {
        // SAFETY : état valide ; nom terminé par un octet nul.
        unsafe {
            xkb_state_mod_name_is_active(self.state.raw(), name.as_ptr(), STATE_MODS_EFFECTIVE) > 0
        }
    }
    /// Passe `keysym` à la composition.
    pub fn compose(&self, keysym: Keysym) -> Composed {
        let Some(compose) = &self.compose else {
            return Composed::Nothing;
        };
        // SAFETY : état de composition valide.
        unsafe {
            if xkb_compose_state_feed(compose.raw(), keysym) != COMPOSE_FEED_ACCEPTED {
                return Composed::Nothing;
            }
            match xkb_compose_state_get_status(compose.raw()) {
                COMPOSE_COMPOSING => Composed::Absorbed,
                COMPOSE_CANCELLED => {
                    xkb_compose_state_reset(compose.raw());
                    Composed::Absorbed
                }
                COMPOSE_COMPOSED => {
                    let mut buffer = [0u8; 64];
                    let n = xkb_compose_state_get_utf8(
                        compose.raw(),
                        buffer.as_mut_ptr().cast(),
                        buffer.len(),
                    );
                    xkb_compose_state_reset(compose.raw());
                    let text = &buffer[..(n.max(0) as usize).min(buffer.len() - 1)];
                    Composed::Text(String::from_utf8_lossy(text).into_owned())
                }
                _ => Composed::Nothing,
            }
        }
    }
    /// Abandonne une séquence en cours (perte du focus).
    pub fn reset_compose(&self) {
        if let Some(compose) = &self.compose {
            // SAFETY : état de composition valide.
            unsafe { xkb_compose_state_reset(compose.raw()) };
        }
    }
}
