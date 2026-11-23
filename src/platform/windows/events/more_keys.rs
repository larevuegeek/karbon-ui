//! Les touches ajoutées à `KeyCode` (KUI-012) : ponctuation, pavé numérique, F13 à F24,
//! touches système. Consultées quand `raw_input::key_code` ne connaît pas le code
//! virtuel : les touches qu'il nommait gardent leur nom.
use crate::ui::runtime::KeyCode;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{MAPVK_VK_TO_CHAR, MapVirtualKeyW};

/// La touche de code virtuel `virtual_key`, parmi celles ajoutées. Une touche de
/// ponctuation est nommée d'après son caractère dans la disposition active.
pub(super) fn added_key_code(virtual_key: u16) -> Option<KeyCode> {
    key_from(virtual_key, character_of)
}

/// Le caractère que donne la touche `virtual_key` sans modificateur dans la disposition
/// active (une touche morte aussi), s'il y en a un.
fn character_of(virtual_key: u16) -> Option<char> {
    // SAFETY : traduction par la disposition active, sans pointeur.
    let value = unsafe { MapVirtualKeyW(u32::from(virtual_key), MAPVK_VK_TO_CHAR) };
    // Le bit de poids fort marque une touche morte : son caractère reste le même.
    char::from_u32(value & 0x7FFF_FFFF).filter(|&c| c != '\0')
}

/// La touche ajoutée de code virtuel `virtual_key` ; `character` dit le caractère d'une
/// touche de ponctuation (séparé pour les essais).
pub(super) fn key_from(
    virtual_key: u16,
    character: impl Fn(u16) -> Option<char>,
) -> Option<KeyCode> {
    use KeyCode::*;
    const PAD: [KeyCode; 10] = [
        Numpad0, Numpad1, Numpad2, Numpad3, Numpad4, Numpad5, Numpad6, Numpad7, Numpad8, Numpad9,
    ];
    const FUNCTIONS: [KeyCode; 12] = [F13, F14, F15, F16, F17, F18, F19, F20, F21, F22, F23, F24];
    Some(match virtual_key {
        0x60..=0x69 => PAD[usize::from(virtual_key - 0x60)],
        0x6A => NumpadMultiply,
        0x6B => NumpadAdd,
        0x6D => NumpadSubtract,
        0x6E => NumpadDecimal,
        0x6F => NumpadDivide,
        0x7C..=0x87 => FUNCTIONS[usize::from(virtual_key - 0x7C)],
        0x5B | 0x5C => Meta,
        0x5D => ContextMenu,
        0x14 => CapsLock,
        0x90 => NumLock,
        0x91 => ScrollLock,
        0x2C => PrintScreen,
        0x13 => Pause,
        // VK_OEM_1 à VK_OEM_3, VK_OEM_4 à VK_OEM_8, VK_OEM_102 : selon leur caractère.
        0xBA..=0xC0 | 0xDB..=0xDF | 0xE2 => punctuation(character(virtual_key)),
        _ => return None,
    })
}

/// La touche de ponctuation qui donne `character`, ou `Unidentified` (« ^ », « ù »).
fn punctuation(character: Option<char>) -> KeyCode {
    use KeyCode::*;
    match character {
        Some('-') => Minus,
        Some('=') => Equal,
        Some('[') => BracketLeft,
        Some(']') => BracketRight,
        Some('\\') => Backslash,
        Some(';') => Semicolon,
        Some('\'') => Quote,
        Some('`') => Backquote,
        Some(',') => Comma,
        Some('.') => Period,
        Some('/') => Slash,
        _ => Unidentified,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Les caractères des touches de ponctuation d'un clavier américain.
    fn us(virtual_key: u16) -> Option<char> {
        Some(match virtual_key {
            0xBA => ';',
            0xBB => '=',
            0xBC => ',',
            0xBD => '-',
            0xBE => '.',
            0xBF => '/',
            0xC0 => '`',
            0xDB => '[',
            0xDC => '\\',
            0xDD => ']',
            0xDE => '\'',
            0xE2 => '\\',
            _ => return None,
        })
    }

    /// … et d'un clavier français : « ^ » est une touche morte, « $ », « ù », « < »
    /// n'ont pas de nom.
    fn french(virtual_key: u16) -> Option<char> {
        Some(match virtual_key {
            0xBA => '$',
            0xBB => '=',
            0xBC => ',',
            0xBE => ';',
            0xBF => ':',
            0xC0 => 'ù',
            0xDB => ')',
            0xDC => '*',
            0xDD => '^',
            0xDE => '²',
            0xDF => '!',
            0xE2 => '<',
            _ => return None,
        })
    }

    #[test]
    fn each_added_virtual_key_has_its_name() {
        let table = [
            (0x60, KeyCode::Numpad0),
            (0x69, KeyCode::Numpad9),
            (0x6A, KeyCode::NumpadMultiply),
            (0x6B, KeyCode::NumpadAdd),
            (0x6D, KeyCode::NumpadSubtract),
            (0x6E, KeyCode::NumpadDecimal),
            (0x6F, KeyCode::NumpadDivide),
            (0x7C, KeyCode::F13),
            (0x87, KeyCode::F24),
            (0x5B, KeyCode::Meta),
            (0x5C, KeyCode::Meta),
            (0x5D, KeyCode::ContextMenu),
            (0x14, KeyCode::CapsLock),
            (0x90, KeyCode::NumLock),
            (0x91, KeyCode::ScrollLock),
            (0x2C, KeyCode::PrintScreen),
            (0x13, KeyCode::Pause),
        ];
        for (vk, code) in table {
            assert_eq!(key_from(vk, us), Some(code), "{vk:#x}");
        }
        // Le pavé numérique ne se confond pas avec les chiffres de la rangée du haut.
        assert_ne!(key_from(0x60, us), Some(KeyCode::Digit0));
        // Une touche inconnue n'est pas donnée.
        assert_eq!(key_from(0xFF, us), None);
    }

    #[test]
    fn punctuation_is_named_by_the_character_of_the_layout() {
        assert_eq!(key_from(0xDB, us), Some(KeyCode::BracketLeft));
        assert_eq!(key_from(0xDD, us), Some(KeyCode::BracketRight));
        assert_eq!(key_from(0xBD, us), Some(KeyCode::Minus));
        assert_eq!(key_from(0xBA, us), Some(KeyCode::Semicolon));
        // Sur un clavier français, la touche à la place de « ; » américain donne « $ » :
        // sans nom, sa place (`physical`) la désigne.
        assert_eq!(key_from(0xBA, french), Some(KeyCode::Unidentified));
        assert_eq!(key_from(0xBE, french), Some(KeyCode::Semicolon));
        assert_eq!(key_from(0xBC, french), Some(KeyCode::Comma));
        assert_eq!(key_from(0xDD, french), Some(KeyCode::Unidentified));
    }

    /// Les touches nommées avant gardent leur nom : `key_code` passe d'abord.
    #[test]
    fn the_keys_named_before_are_not_taken_here() {
        for vk in [0x41u16, 0x30, 0x70, 0x0D, 0x1B, 0x25] {
            assert_eq!(key_from(vk, us), None, "{vk:#x}");
        }
    }
}
