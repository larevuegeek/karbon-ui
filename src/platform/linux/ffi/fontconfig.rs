//! fontconfig (configuration et recherche des polices) et FreeType (décodage des
//! conteneurs WOFF et WOFF2, que HarfBuzz ne lit pas).
use super::required;
use std::{
    ffi::{CStr, c_char, c_int, c_long, c_ulong, c_void},
    ptr::NonNull,
};

#[repr(C)]
pub(in crate::platform::linux) struct RawConfig {
    _private: [u8; 0],
}
#[repr(C)]
struct RawPattern {
    _private: [u8; 0],
}
#[repr(C)]
struct RawObjectSet {
    _private: [u8; 0],
}
#[repr(C)]
struct FontSet {
    nfont: c_int,
    sfont: c_int,
    fonts: *mut *mut RawPattern,
}
const SET_APPLICATION: c_int = 1;
const RESULT_MATCH: c_int = 0;
const FAMILY: &CStr = c"family";
const FILE: &CStr = c"file";
const VARIABLE: &CStr = c"variable";

#[link(name = "fontconfig")]
unsafe extern "C" {
    fn FcInitLoadConfigAndFonts() -> *mut RawConfig;
    fn FcConfigDestroy(config: *mut RawConfig);
    fn FcConfigAppFontAddFile(config: *mut RawConfig, file: *const u8) -> c_int;
    fn FcConfigGetFonts(config: *mut RawConfig, set: c_int) -> *mut FontSet;
    fn FcPatternCreate() -> *mut RawPattern;
    fn FcPatternDestroy(pattern: *mut RawPattern);
    fn FcPatternGetString(
        pattern: *const RawPattern,
        object: *const c_char,
        n: c_int,
        value: *mut *mut u8,
    ) -> c_int;
    fn FcPatternGetBool(
        pattern: *const RawPattern,
        object: *const c_char,
        n: c_int,
        value: *mut c_int,
    ) -> c_int;
    fn FcPatternDel(pattern: *mut RawPattern, object: *const c_char) -> c_int;
    fn FcPatternAddString(
        pattern: *mut RawPattern,
        object: *const c_char,
        value: *const u8,
    ) -> c_int;
    fn FcObjectSetCreate() -> *mut RawObjectSet;
    fn FcObjectSetAdd(set: *mut RawObjectSet, object: *const c_char) -> c_int;
    fn FcObjectSetDestroy(set: *mut RawObjectSet);
    fn FcFontList(
        config: *mut RawConfig,
        pattern: *mut RawPattern,
        objects: *mut RawObjectSet,
    ) -> *mut FontSet;
    fn FcFontSetDestroy(set: *mut FontSet);
}
/// Configuration fontconfig privée : polices du système et polices de l'application.
pub(in crate::platform::linux) struct Config(NonNull<RawConfig>);
impl Config {
    /// Configuration chargée comme celle du système (fichiers de configuration et caches).
    pub fn system() -> crate::Result<Self> {
        // SAFETY : constructeur ; configuration possédée.
        Ok(Self(required(
            unsafe { FcInitLoadConfigAndFonts() },
            "FcInitLoadConfigAndFonts",
        )?))
    }
    pub fn raw(&self) -> *mut RawConfig {
        self.0.as_ptr()
    }
    /// Ajoute le fichier `path` aux polices de l'application sous la famille `family`
    /// (qui remplace celle du fichier) ; renvoie `None` si fontconfig refuse le fichier,
    /// sinon si l'une de ses instances est variable.
    pub fn add_font(&self, path: &CStr, family: &CStr) -> Option<bool> {
        // SAFETY : configuration valide ; chemin lu pendant l'appel.
        if unsafe { FcConfigAppFontAddFile(self.raw(), path.as_ptr().cast()) } == 0 {
            return None;
        }
        // SAFETY : ensemble des polices de l'application, possédé par la configuration.
        let set = unsafe { FcConfigGetFonts(self.raw(), SET_APPLICATION) };
        if set.is_null() {
            return None;
        }
        // SAFETY : `nfont` motifs valides dans l'ensemble, modifiables (polices ajoutées
        // par l'application, pas les caches du système).
        let fonts =
            unsafe { std::slice::from_raw_parts((*set).fonts, (*set).nfont.max(0) as usize) };
        let (mut added, mut variable) = (false, false);
        for &pattern in fonts {
            if string(pattern, FILE).as_deref() != Some(path.to_bytes()) {
                continue;
            }
            // SAFETY : motif valide ; chaîne copiée par fontconfig.
            unsafe {
                FcPatternDel(pattern, FAMILY.as_ptr());
                FcPatternAddString(pattern, FAMILY.as_ptr(), family.as_ptr().cast());
            }
            let mut flag = 0;
            // SAFETY : motif valide ; sortie locale.
            let found = unsafe { FcPatternGetBool(pattern, VARIABLE.as_ptr(), 0, &mut flag) };
            variable |= found == RESULT_MATCH && flag != 0;
            added = true;
        }
        added.then_some(variable)
    }
}
impl Drop for Config {
    fn drop(&mut self) {
        // SAFETY : référence possédée, rendue une fois.
        unsafe { FcConfigDestroy(self.raw()) };
    }
}
/// Valeur chaîne `object` d'un motif.
fn string(pattern: *const RawPattern, object: &CStr) -> Option<Vec<u8>> {
    let mut value = std::ptr::null_mut();
    // SAFETY : motif valide ; la chaîne appartient au motif et est copiée aussitôt.
    let found = unsafe { FcPatternGetString(pattern, object.as_ptr(), 0, &mut value) };
    (found == RESULT_MATCH && !value.is_null())
        .then(|| unsafe { CStr::from_ptr(value.cast()) }.to_bytes().to_vec())
}
/// `true` si une police installée (ou de l'application) porte exactement la famille
/// `family` (sans substitution), dans `config` ou la configuration courante.
pub(in crate::platform::linux) fn has_family(config: Option<&Config>, family: &CStr) -> bool {
    // SAFETY : motif et ensemble d'objets possédés, détruits ci-dessous ; la liste rendue
    // par FcFontList est possédée et détruite.
    unsafe {
        let pattern = FcPatternCreate();
        let objects = FcObjectSetCreate();
        if pattern.is_null() || objects.is_null() {
            if !pattern.is_null() {
                FcPatternDestroy(pattern);
            }
            if !objects.is_null() {
                FcObjectSetDestroy(objects);
            }
            return false;
        }
        FcPatternAddString(pattern, FAMILY.as_ptr(), family.as_ptr().cast());
        FcObjectSetAdd(objects, FAMILY.as_ptr());
        let list = FcFontList(
            config.map_or(std::ptr::null_mut(), Config::raw),
            pattern,
            objects,
        );
        FcPatternDestroy(pattern);
        FcObjectSetDestroy(objects);
        if list.is_null() {
            return false;
        }
        let found = (*list).nfont > 0;
        FcFontSetDestroy(list);
        found
    }
}
#[link(name = "freetype")]
unsafe extern "C" {
    fn FT_Init_FreeType(library: *mut *mut c_void) -> c_int;
    fn FT_Done_FreeType(library: *mut c_void) -> c_int;
    fn FT_New_Memory_Face(
        library: *mut c_void,
        base: *const u8,
        size: c_long,
        index: c_long,
        face: *mut *mut c_void,
    ) -> c_int;
    fn FT_Done_Face(face: *mut c_void) -> c_int;
    fn FT_Load_Sfnt_Table(
        face: *mut c_void,
        tag: c_ulong,
        offset: c_long,
        buffer: *mut u8,
        length: *mut c_ulong,
    ) -> c_int;
}
/// Police brute (sfnt) d'un conteneur WOFF ou WOFF2 décodé par FreeType : la table 0
/// désigne tout le fichier, celui que FreeType a reconstruit. `None` si FreeType refuse
/// les données (FreeType compilé sans brotli pour le WOFF2, fichier abîmé).
pub(in crate::platform::linux) fn sfnt(data: &[u8]) -> Option<Vec<u8>> {
    let size = c_long::try_from(data.len()).ok()?;
    let mut library = std::ptr::null_mut();
    // SAFETY : sortie locale ; bibliothèque libérée ci-dessous.
    if unsafe { FT_Init_FreeType(&mut library) } != 0 {
        return None;
    }
    let mut face = std::ptr::null_mut();
    let mut result = None;
    // SAFETY : `data` reste vivant pendant toute la vie de la face (détruite ici).
    if unsafe { FT_New_Memory_Face(library, data.as_ptr(), size, 0, &mut face) } == 0 {
        let mut length: c_ulong = 0;
        // SAFETY : face valide ; une longueur nulle demande la taille.
        if unsafe { FT_Load_Sfnt_Table(face, 0, 0, std::ptr::null_mut(), &mut length) } == 0
            && length > 0
        {
            let mut bytes = vec![0u8; length as usize];
            // SAFETY : tampon de `length` octets valide en écriture.
            if unsafe { FT_Load_Sfnt_Table(face, 0, 0, bytes.as_mut_ptr(), &mut length) } == 0 {
                bytes.truncate(length as usize);
                result = Some(bytes);
            }
        }
        // SAFETY : face possédée, détruite une fois.
        unsafe { FT_Done_Face(face) };
    }
    // SAFETY : bibliothèque possédée, détruite une fois.
    unsafe { FT_Done_FreeType(library) };
    result
}
