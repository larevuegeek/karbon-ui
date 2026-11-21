//! Place physique des touches sous Windows (`KeyPress::physical`, `KeyPress::scancode`) :
//! le code de balayage que porte lparam des messages du clavier (bits 16 à 23, et bit 24
//! pour les touches étendues), traduit par une table fixe, indépendante de la disposition.
use crate::ui::runtime::PhysicalKey;
use std::cell::Cell;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{MAPVK_VK_TO_VSC_EX, MapVirtualKeyW};

/// Bit 24 de lparam : touche étendue (flèches et pavé de navigation, Ctrl et Alt de droite,
/// Entrée et / du pavé numérique, touches Windows…).
const EXTENDED: isize = 1 << 24;
/// Préfixe des codes étendus dans [`scancode`], comme `MapVirtualKeyW` le donne.
const EXTENDED_PREFIX: u32 = 0xE000;

thread_local! {
    /// lparam du WM_KEYUP en cours de traitement : `keyboard::key_up` ne le transmet pas
    /// à `raw_input::key`, la table de routage le dépose ici juste avant.
    static RELEASE: Cell<isize> = const { Cell::new(0) };
}

/// Note lparam du WM_KEYUP qui va être traité.
pub(super) fn remember_release(lparam: isize) {
    RELEASE.set(lparam);
}

/// lparam du WM_KEYUP en cours, déposé par [`remember_release`] ; 0 s'il n'y en a pas.
pub(super) fn take_release() -> isize {
    RELEASE.take()
}

/// Code de balayage d'un message du clavier : celui de lparam, avec `0xE000` pour une
/// touche étendue. Quand lparam n'en porte pas (touche envoyée par un programme avec son
/// seul code virtuel), celui que Windows associe au code virtuel dans la disposition
/// active ; 0 si aucun.
pub(super) fn scancode(virtual_key: u16, lparam: isize) -> u32 {
    let code = ((lparam >> 16) & 0xFF) as u32;
    if code != 0 {
        let extended = lparam & EXTENDED != 0;
        return code | if extended { EXTENDED_PREFIX } else { 0 };
    }
    // SAFETY : simple traduction par la disposition du thread, sans pointeur.
    unsafe { MapVirtualKeyW(u32::from(virtual_key), MAPVK_VK_TO_VSC_EX) }
}

/// La touche à la place de code de balayage `scancode` (jeu 1, préfixe `0xE000` pour les
/// touches étendues), comme le web la nomme (table de Chromium pour Windows).
pub(super) fn physical_key(scancode: u32) -> Option<PhysicalKey> {
    use PhysicalKey::*;
    Some(match scancode {
        0x01 => Escape,
        0x02 => Digit1,
        0x03 => Digit2,
        0x04 => Digit3,
        0x05 => Digit4,
        0x06 => Digit5,
        0x07 => Digit6,
        0x08 => Digit7,
        0x09 => Digit8,
        0x0A => Digit9,
        0x0B => Digit0,
        0x0C => Minus,
        0x0D => Equal,
        0x0E => Backspace,
        0x0F => Tab,
        0x10 => KeyQ,
        0x11 => KeyW,
        0x12 => KeyE,
        0x13 => KeyR,
        0x14 => KeyT,
        0x15 => KeyY,
        0x16 => KeyU,
        0x17 => KeyI,
        0x18 => KeyO,
        0x19 => KeyP,
        0x1A => BracketLeft,
        0x1B => BracketRight,
        0x1C => Enter,
        0x1D => ControlLeft,
        0x1E => KeyA,
        0x1F => KeyS,
        0x20 => KeyD,
        0x21 => KeyF,
        0x22 => KeyG,
        0x23 => KeyH,
        0x24 => KeyJ,
        0x25 => KeyK,
        0x26 => KeyL,
        0x27 => Semicolon,
        0x28 => Quote,
        0x29 => Backquote,
        0x2A => ShiftLeft,
        0x2B => Backslash,
        0x2C => KeyZ,
        0x2D => KeyX,
        0x2E => KeyC,
        0x2F => KeyV,
        0x30 => KeyB,
        0x31 => KeyN,
        0x32 => KeyM,
        0x33 => Comma,
        0x34 => Period,
        0x35 => Slash,
        0x36 => ShiftRight,
        0x37 => NumpadMultiply,
        0x38 => AltLeft,
        0x39 => Space,
        0x3A => CapsLock,
        0x3B => F1,
        0x3C => F2,
        0x3D => F3,
        0x3E => F4,
        0x3F => F5,
        0x40 => F6,
        0x41 => F7,
        0x42 => F8,
        0x43 => F9,
        0x44 => F10,
        // Pause arrive sans le bit étendu, Verr. Num. avec (l'inverse de leur histoire).
        0x45 => Pause,
        0x46 => ScrollLock,
        0x47 => Numpad7,
        0x48 => Numpad8,
        0x49 => Numpad9,
        0x4A => NumpadSubtract,
        0x4B => Numpad4,
        0x4C => Numpad5,
        0x4D => Numpad6,
        0x4E => NumpadAdd,
        0x4F => Numpad1,
        0x50 => Numpad2,
        0x51 => Numpad3,
        0x52 => Numpad0,
        0x53 => NumpadDecimal,
        0x56 => IntlBackslash,
        0x57 => F11,
        0x58 => F12,
        0x59 => NumpadEqual,
        0x64 => F13,
        0x65 => F14,
        0x66 => F15,
        0x67 => F16,
        0x68 => F17,
        0x69 => F18,
        0x6A => F19,
        0x6B => F20,
        0x6C => F21,
        0x6D => F22,
        0x6E => F23,
        0x73 => IntlRo,
        0x76 => F24,
        0x7D => IntlYen,
        0xE01C => NumpadEnter,
        0xE01D => ControlRight,
        0xE035 => NumpadDivide,
        0xE037 => PrintScreen,
        0xE038 => AltRight,
        0xE045 => NumLock,
        0xE047 => Home,
        0xE048 => ArrowUp,
        0xE049 => PageUp,
        0xE04B => ArrowLeft,
        0xE04D => ArrowRight,
        0xE04F => End,
        0xE050 => ArrowDown,
        0xE051 => PageDown,
        0xE052 => Insert,
        0xE053 => Delete,
        0xE05B => MetaLeft,
        0xE05C => MetaRight,
        0xE05D => ContextMenu,
        // Pause selon `MapVirtualKeyW` (séquence E1 1D 45 du clavier).
        0xE11D => Pause,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// lparam d'un message du clavier portant `code` (et le bit étendu au besoin).
    fn lparam(code: isize, extended: bool) -> isize {
        (code << 16) | if extended { EXTENDED } else { 0 } | 1
    }

    #[test]
    fn scan_codes_name_the_place_of_the_key() {
        use PhysicalKey::*;
        let table = [
            (0x10, Some(KeyQ)),
            (0x11, Some(KeyW)),
            (0x1E, Some(KeyA)),
            (0x2C, Some(KeyZ)),
            (0x32, Some(KeyM)),
            (0x02, Some(Digit1)),
            (0x0B, Some(Digit0)),
            (0x27, Some(Semicolon)),
            (0x28, Some(Quote)),
            (0x29, Some(Backquote)),
            (0x56, Some(IntlBackslash)),
            (0x3B, Some(F1)),
            (0x44, Some(F10)),
            (0x57, Some(F11)),
            (0x58, Some(F12)),
            (0x64, Some(F13)),
            (0x76, Some(F24)),
            (0x2A, Some(ShiftLeft)),
            (0x36, Some(ShiftRight)),
            (0x1D, Some(ControlLeft)),
            (0xE01D, Some(ControlRight)),
            (0x38, Some(AltLeft)),
            (0xE038, Some(AltRight)),
            (0xE05B, Some(MetaLeft)),
            (0x1C, Some(Enter)),
            (0xE01C, Some(NumpadEnter)),
            (0x45, Some(Pause)),
            (0xE11D, Some(Pause)),
            (0xE045, Some(NumLock)),
            (0x37, Some(NumpadMultiply)),
            (0xE037, Some(PrintScreen)),
            (0x00, None),
            (0x54, None),
            (0xE010, None),
        ];
        for (code, expected) in table {
            assert_eq!(physical_key(code), expected, "code {code:#06X}");
        }
    }

    /// Les touches du pavé de navigation et celles du pavé numérique partagent un code ; le
    /// bit étendu de lparam les sépare.
    #[test]
    fn the_extended_bit_separates_the_arrows_from_the_numeric_keypad() {
        use PhysicalKey::*;
        let pairs = [
            (0x48, ArrowUp, Numpad8),
            (0x50, ArrowDown, Numpad2),
            (0x4B, ArrowLeft, Numpad4),
            (0x4D, ArrowRight, Numpad6),
            (0x47, Home, Numpad7),
            (0x4F, End, Numpad1),
            (0x49, PageUp, Numpad9),
            (0x51, PageDown, Numpad3),
            (0x52, Insert, Numpad0),
            (0x53, Delete, NumpadDecimal),
            (0x35, NumpadDivide, Slash),
        ];
        for (code, extended, keypad) in pairs {
            let up = scancode(0, lparam(code, true));
            assert_eq!(up, 0xE000 | code as u32);
            assert_eq!(physical_key(up), Some(extended));
            assert_eq!(physical_key(scancode(0, lparam(code, false))), Some(keypad));
        }
    }

    /// Chaque touche de la table a une seule place : deux codes ne donnent la même touche
    /// que pour Pause (son code dans lparam, et celui de `MapVirtualKeyW`).
    #[test]
    fn no_two_scan_codes_give_the_same_key_except_pause() {
        let mut seen = std::collections::HashMap::new();
        for code in (0..0x80).chain(0xE000..0xE080).chain(0xE100..0xE180) {
            if let Some(key) = physical_key(code)
                && let Some(first) = seen.insert(key, code)
            {
                assert_eq!(key, PhysicalKey::Pause, "{first:#06X} et {code:#06X}");
            }
        }
        // Les 120 touches de `PhysicalKey` ont toutes un code.
        assert_eq!(seen.len(), 120);
    }

    #[test]
    fn a_release_keeps_its_lparam_until_it_is_taken() {
        remember_release(lparam(0x11, false));
        assert_eq!(scancode(0x5A, take_release()), 0x11);
        assert_eq!(take_release(), 0);
    }
}
