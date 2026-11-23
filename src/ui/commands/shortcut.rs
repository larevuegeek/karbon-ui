//! Un raccourci du clavier : une touche, prise par ce qui est écrit dessus ou par sa
//! place, et les touches tenues avec elle ; lu et écrit en clair, en anglais ou en
//! français (« Ctrl+Shift+D », « Ctrl+Maj+D »).
use crate::ui::runtime::{KeyCode, KeyPress, PhysicalKey};
use std::fmt;

/// The key of a [`Shortcut`]: named after what the layout prints on it (Ctrl+Z is undo on
/// every layout), or taken by its place on the keyboard (the camera keys WASD on a US
/// keyboard, ZQSD on a French one).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShortcutKey {
    /// The key marked with this name in the active layout.
    Key(KeyCode),
    /// The key at this place, whatever is printed on it.
    Place(PhysicalKey),
}

/// A key and the modifier keys held with it: « Ctrl+S », « Ctrl+Shift+D », « F2 ».
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Shortcut {
    /// The key.
    pub key: ShortcutKey,
    /// Ctrl is held.
    pub control: bool,
    /// Shift is held.
    pub shift: bool,
    /// Alt is held.
    pub alt: bool,
    /// The Windows key (Cmd) is held.
    pub meta: bool,
}

/// The language of the key names of a [`Shortcut`] in clear (both are always read).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ShortcutNames {
    /// « Ctrl+Shift+Delete ».
    #[default]
    English,
    /// « Ctrl+Maj+Suppr ».
    French,
}

/// Why a shortcut written in clear cannot be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShortcutError(pub String);

impl fmt::Display for ShortcutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ShortcutError {}

/// Noms des touches nommées : le code, le nom anglais, le nom français.
const NAMES: &[(KeyCode, &str, &str)] = &[
    (KeyCode::Escape, "Escape", "Échap"),
    (KeyCode::Enter, "Enter", "Entrée"),
    (KeyCode::Tab, "Tab", "Tab"),
    (KeyCode::Space, "Space", "Espace"),
    (KeyCode::Backspace, "Backspace", "Retour arrière"),
    (KeyCode::Delete, "Delete", "Suppr"),
    (KeyCode::Insert, "Insert", "Inser"),
    (KeyCode::Home, "Home", "Origine"),
    (KeyCode::End, "End", "Fin"),
    (KeyCode::PageUp, "PageUp", "Pg préc"),
    (KeyCode::PageDown, "PageDown", "Pg suiv"),
    (KeyCode::Left, "Left", "Gauche"),
    (KeyCode::Right, "Right", "Droite"),
    (KeyCode::Up, "Up", "Haut"),
    (KeyCode::Down, "Down", "Bas"),
    (KeyCode::Minus, "-", "-"),
    (KeyCode::Equal, "=", "="),
    (KeyCode::BracketLeft, "[", "["),
    (KeyCode::BracketRight, "]", "]"),
    (KeyCode::Backslash, "\\", "\\"),
    (KeyCode::Semicolon, ";", ";"),
    (KeyCode::Quote, "'", "'"),
    (KeyCode::Backquote, "`", "`"),
    (KeyCode::Comma, ",", ","),
    (KeyCode::Period, ".", "."),
    (KeyCode::Slash, "/", "/"),
    (KeyCode::NumpadAdd, "Num+", "+ du pavé"),
    (KeyCode::NumpadSubtract, "Num-", "- du pavé"),
    (KeyCode::NumpadMultiply, "Num*", "* du pavé"),
    (KeyCode::NumpadDivide, "Num/", "/ du pavé"),
    (KeyCode::NumpadDecimal, "Num.", ". du pavé"),
    (KeyCode::ContextMenu, "Menu", "Menu"),
    (KeyCode::PrintScreen, "PrintScreen", "Impr. écran"),
    (KeyCode::Pause, "Pause", "Pause"),
];

/// Les touches dont le nom se calcule : lettres, chiffres, pavé, F1 à F24.
fn computed_key(key: KeyCode) -> Option<String> {
    use KeyCode::*;
    const LETTERS: [KeyCode; 26] = [
        A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    ];
    const DIGITS: [KeyCode; 10] = [
        Digit0, Digit1, Digit2, Digit3, Digit4, Digit5, Digit6, Digit7, Digit8, Digit9,
    ];
    const PAD: [KeyCode; 10] = [
        Numpad0, Numpad1, Numpad2, Numpad3, Numpad4, Numpad5, Numpad6, Numpad7, Numpad8, Numpad9,
    ];
    const FUNCTIONS: [KeyCode; 24] = [
        F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12, F13, F14, F15, F16, F17, F18, F19, F20,
        F21, F22, F23, F24,
    ];
    if let Some(i) = LETTERS.iter().position(|&k| k == key) {
        return Some(char::from(b'A' + i as u8).to_string());
    }
    if let Some(i) = DIGITS.iter().position(|&k| k == key) {
        return Some(i.to_string());
    }
    if let Some(i) = PAD.iter().position(|&k| k == key) {
        return Some(format!("Num{i}"));
    }
    FUNCTIONS
        .iter()
        .position(|&k| k == key)
        .map(|i| format!("F{}", i + 1))
}

/// Le nom de la touche `key`.
fn key_name(key: KeyCode, names: ShortcutNames) -> String {
    if let Some((_, english, french)) = NAMES.iter().find(|(k, _, _)| *k == key) {
        return match names {
            ShortcutNames::English => english,
            ShortcutNames::French => french,
        }
        .to_string();
    }
    let name = computed_key(key).unwrap_or_else(|| format!("{key:?}"));
    match (names, name.strip_prefix("Num")) {
        (ShortcutNames::French, Some(digit)) => format!("{digit} du pavé"),
        _ => name,
    }
}

/// La touche nommée `name` (anglais ou français, sans tenir compte des capitales).
fn parse_key(name: &str) -> Option<KeyCode> {
    let wanted = name.trim();
    if let Some((key, _, _)) = NAMES.iter().find(|(_, e, f)| {
        e.eq_ignore_ascii_case(wanted) || f.to_lowercase() == wanted.to_lowercase()
    }) {
        return Some(*key);
    }
    let pad = wanted
        .strip_suffix(" du pavé")
        .map(|digit| format!("Num{digit}"));
    let wanted = pad.as_deref().unwrap_or(wanted);
    ALL_COMPUTED
        .iter()
        .copied()
        .find(|&key| computed_key(key).is_some_and(|n| n.eq_ignore_ascii_case(wanted)))
}

/// Toutes les touches dont le nom se calcule.
const ALL_COMPUTED: [KeyCode; 70] = {
    use KeyCode::*;
    [
        A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z, Digit0,
        Digit1, Digit2, Digit3, Digit4, Digit5, Digit6, Digit7, Digit8, Digit9, Numpad0, Numpad1,
        Numpad2, Numpad3, Numpad4, Numpad5, Numpad6, Numpad7, Numpad8, Numpad9, F1, F2, F3, F4, F5,
        F6, F7, F8, F9, F10, F11, F12, F13, F14, F15, F16, F17, F18, F19, F20, F21, F22, F23, F24,
    ]
};

impl Shortcut {
    /// `key` alone, named after what is printed on it.
    pub const fn key(key: KeyCode) -> Self {
        Self {
            key: ShortcutKey::Key(key),
            control: false,
            shift: false,
            alt: false,
            meta: false,
        }
    }

    /// The key at `place`, alone.
    pub const fn place(place: PhysicalKey) -> Self {
        Self {
            key: ShortcutKey::Place(place),
            ..Self::key(KeyCode::Escape)
        }
    }

    /// The same shortcut with Ctrl held.
    pub const fn ctrl(mut self) -> Self {
        self.control = true;
        self
    }

    /// The same shortcut with Shift held.
    pub const fn shift(mut self) -> Self {
        self.shift = true;
        self
    }

    /// The same shortcut with Alt held.
    pub const fn alt(mut self) -> Self {
        self.alt = true;
        self
    }

    /// The same shortcut with the Windows key (Cmd) held.
    pub const fn meta(mut self) -> Self {
        self.meta = true;
        self
    }

    /// Whether `press` is this shortcut going down (or repeating): the same key, by its
    /// name or by its place, with exactly these modifiers.
    pub fn matches(&self, press: &KeyPress) -> bool {
        let key = match self.key {
            ShortcutKey::Key(key) => press.key == key,
            ShortcutKey::Place(place) => press.physical == Some(place),
        };
        press.pressed
            && key
            && press.control == self.control
            && press.shift == self.shift
            && press.alt == self.alt
            && press.meta == self.meta
    }

    /// The shortcut in clear, as menus show it: « Ctrl+Shift+D » or « Ctrl+Maj+D »; a
    /// key taken by its place is written in brackets, after its web name (« `[KeyW]` »).
    pub fn text(&self, names: ShortcutNames) -> String {
        let (shift, meta) = match names {
            ShortcutNames::English => ("Shift+", "Win+"),
            ShortcutNames::French => ("Maj+", "Win+"),
        };
        let mut text = String::new();
        for (held, name) in [
            (self.control, "Ctrl+"),
            (self.shift, shift),
            (self.alt, "Alt+"),
            (self.meta, meta),
        ] {
            if held {
                text.push_str(name);
            }
        }
        match self.key {
            ShortcutKey::Key(key) => text.push_str(&key_name(key, names)),
            ShortcutKey::Place(place) => text.push_str(&format!("[{place:?}]")),
        }
        text
    }

    /// The shortcut written `text`, in English or in French, capitals ignored:
    /// « Ctrl+S », « ctrl+maj+d », « Alt+F4 », « 7 du pavé », « `[KeyW]` » for a place.
    pub fn parse(text: &str) -> Result<Self, ShortcutError> {
        let parts: Vec<&str> = split_plus(text);
        let (name, held) = parts
            .split_last()
            .ok_or_else(|| ShortcutError("empty shortcut".into()))?;
        let mut shortcut = Self::key(KeyCode::Escape);
        for modifier in held {
            match modifier.trim().to_lowercase().as_str() {
                "ctrl" | "control" => shortcut.control = true,
                "shift" | "maj" => shortcut.shift = true,
                "alt" => shortcut.alt = true,
                "win" | "meta" | "cmd" => shortcut.meta = true,
                other => return Err(ShortcutError(format!("unknown modifier « {other} »"))),
            }
        }
        shortcut.key = match name
            .trim()
            .strip_prefix('[')
            .and_then(|n| n.strip_suffix(']'))
        {
            Some(place) => ShortcutKey::Place(
                place_named(place)
                    .ok_or_else(|| ShortcutError(format!("unknown place « {place} »")))?,
            ),
            None => ShortcutKey::Key(
                parse_key(name).ok_or_else(|| ShortcutError(format!("unknown key « {name} »")))?,
            ),
        };
        Ok(shortcut)
    }
}

/// Coupe `text` aux « + », en gardant un « + » qui est la touche elle-même
/// (« Ctrl++ », « Ctrl+Num+ »).
fn split_plus(text: &str) -> Vec<&str> {
    let text = text.trim();
    if text.is_empty() {
        return Vec::new();
    }
    let mut parts = Vec::new();
    let mut start = 0;
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'+' && i > start {
            parts.push(&text[start..i]);
            start = i + 1;
        }
        i += 1;
    }
    parts.push(&text[start..]);
    parts
}

/// La place nommée `name` (nom du web : « KeyW », « BracketLeft »).
fn place_named(name: &str) -> Option<PhysicalKey> {
    super::places::ALL
        .iter()
        .copied()
        .find(|place| format!("{place:?}").eq_ignore_ascii_case(name.trim()))
}

impl fmt::Display for Shortcut {
    /// The shortcut in English (« Ctrl+Shift+D »).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text(ShortcutNames::English))
    }
}
