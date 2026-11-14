//! Menus Win32 au thème du système : sombres quand Windows est en mode sombre (« Choisir
//! le mode de l'application »), comme les menus de l'Explorateur et des autres icônes de la
//! zone de notification.
//!
//! Windows n'a pas d'API documentée pour cela. L'Explorateur, le Bloc-notes et Windows
//! Terminal appellent trois fonctions d'`uxtheme.dll` exportées par ordinal seulement,
//! stables depuis Windows 10 1809 :
//!
//! - 135 `SetPreferredAppMode(AllowDark)` (1903 et plus ; `AllowDarkModeForApp(TRUE)` en
//!   1809, même argument `1`) : les menus du processus peuvent suivre le mode sombre ;
//! - 133 `AllowDarkModeForWindow(fenêtre, TRUE)` : la fenêtre propriétaire du menu
//!   l'accepte ;
//! - 136 `FlushMenuThemes()` : recharge le thème des menus.
//!
//! Absentes (Windows plus ancien, ou retirées un jour) : le menu reste clair, rien d'autre
//! ne change.
use super::super::ffi::Hwnd;
use std::sync::OnceLock;
use windows_sys::Win32::System::LibraryLoader::{
    GetModuleHandleW, GetProcAddress, LOAD_LIBRARY_SEARCH_SYSTEM32, LoadLibraryExW,
};

/// Fonction exportée telle que la rend `GetProcAddress`.
type Exported = unsafe extern "system" fn() -> isize;
type SetPreferredAppMode = unsafe extern "system" fn(mode: i32) -> i32;
type AllowDarkModeForWindow = unsafe extern "system" fn(window: Hwnd, allow: i32) -> i32;
type FlushMenuThemes = unsafe extern "system" fn();

/// `PreferredAppMode::AllowDark` : sombre quand le système l'est.
const ALLOW_DARK: i32 = 1;

struct Uxtheme {
    allow_for_window: AllowDarkModeForWindow,
    flush: FlushMenuThemes,
}

/// Fonctions chargées une fois ; le mode préféré du processus est posé à ce moment.
fn uxtheme() -> Option<&'static Uxtheme> {
    static UXTHEME: OnceLock<Option<Uxtheme>> = OnceLock::new();
    UXTHEME
        .get_or_init(|| {
            let name: Vec<u16> = "uxtheme.dll\0".encode_utf16().collect();
            // SAFETY : nom terminé par zéro ; bibliothèque système chargée depuis System32 et
            // jamais déchargée (les pointeurs gardés restent valides).
            let module = unsafe {
                let loaded = GetModuleHandleW(name.as_ptr());
                if loaded.is_null() {
                    LoadLibraryExW(
                        name.as_ptr(),
                        std::ptr::null_mut(),
                        LOAD_LIBRARY_SEARCH_SYSTEM32,
                    )
                } else {
                    loaded
                }
            };
            if module.is_null() {
                return None;
            }
            // SAFETY : un ordinal se passe à la place du nom (MAKEINTRESOURCEA) ; les
            // signatures sont celles qu'utilisent l'Explorateur et Windows Terminal.
            unsafe {
                let set_mode = GetProcAddress(module, 135 as *const u8)?;
                let allow_for_window = GetProcAddress(module, 133 as *const u8)?;
                let flush = GetProcAddress(module, 136 as *const u8)?;
                let set_mode = std::mem::transmute::<Exported, SetPreferredAppMode>(set_mode);
                let flush = std::mem::transmute::<Exported, FlushMenuThemes>(flush);
                set_mode(ALLOW_DARK);
                flush();
                Some(Uxtheme {
                    allow_for_window: std::mem::transmute::<Exported, AllowDarkModeForWindow>(
                        allow_for_window,
                    ),
                    flush,
                })
            }
        })
        .as_ref()
}

/// À appeler avant d'ouvrir un menu possédé par `window` : il suivra le mode du système
/// (relu à chaque ouverture, un changement de mode est donc pris en compte).
pub(crate) fn follow_system(window: Hwnd) {
    if let Some(uxtheme) = uxtheme() {
        // SAFETY : fenêtre valide pendant son message ; fonctions résolues ci-dessus.
        unsafe {
            (uxtheme.allow_for_window)(window, 1);
            (uxtheme.flush)();
        }
    }
}
