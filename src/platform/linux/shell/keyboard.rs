//! Clavier : disposition envoyée par le compositeur (xkbcommon), touches du moteur,
//! raccourcis d'édition (Ctrl+A/C/X/V/Z/Y, Ctrl+←/→, Maj+Inser, Ctrl+Inser, Maj+Suppr),
//! texte saisi avec les touches mortes (`Compose` de la locale) et répétition des touches
//! côté client (`wl_keyboard.repeat_info` sous Wayland). Partagé par les boucles Wayland et
//! X11.
use crate::{
    platform::linux::ffi::xkb::{Composed, Keyboard, Keysym},
    ui::{
        runtime::{Key, UiEvent},
        text_input::EditCommand,
    },
};
use std::{
    os::fd::{AsRawFd, OwnedFd},
    time::{Duration, Instant},
};

/// Différence entre un code evdev (`wl_keyboard.key`) et un code XKB.
const EVDEV_OFFSET: u32 = 8;

/// Touche du moteur d'un symbole XKB.
fn key(sym: Keysym) -> Option<Key> {
    Some(match sym {
        0xff09 | 0xfe20 => Key::Tab,
        0xff0d | 0xff8d => Key::Enter,
        0x20 | 0xff80 => Key::Space,
        0xff51 | 0xff96 => Key::Left,
        0xff52 | 0xff97 => Key::Up,
        0xff53 | 0xff98 => Key::Right,
        0xff54 | 0xff99 => Key::Down,
        0xff50 | 0xff95 => Key::Home,
        0xff57 | 0xff9c => Key::End,
        0xff55 | 0xff9a => Key::PageUp,
        0xff56 | 0xff9b => Key::PageDown,
        0xff1b => Key::Escape,
        0xff08 => Key::Backspace,
        0xffff | 0xff9f => Key::Delete,
        _ => return None,
    })
}
/// Modificateurs utiles au moment d'une touche.
#[derive(Clone, Copy, Default)]
struct Modifiers {
    shift: bool,
    control: bool,
    alt: bool,
    logo: bool,
}
/// Raccourci d'édition de la touche `sym`.
fn edit(sym: Keysym, m: Modifiers) -> Option<EditCommand> {
    let lower = if (0x41..=0x5a).contains(&sym) {
        sym + 0x20
    } else {
        sym
    };
    let extend = m.shift;
    match (m.control, m.shift, lower) {
        (true, _, 0x61) => Some(EditCommand::SelectAll),
        (true, _, 0x63) | (true, false, 0xff63) => Some(EditCommand::Copy),
        (true, _, 0x78) => Some(EditCommand::Cut),
        (true, _, 0x76) => Some(EditCommand::Paste),
        (true, false, 0x7a) => Some(EditCommand::Undo),
        (true, true, 0x7a) | (true, _, 0x79) => Some(EditCommand::Redo),
        (true, _, 0xff51 | 0xff96) => Some(EditCommand::WordLeft { extend }),
        (true, _, 0xff53 | 0xff98) => Some(EditCommand::WordRight { extend }),
        (false, true, 0xff63) => Some(EditCommand::Paste),
        (false, true, 0xffff) => Some(EditCommand::Cut),
        _ => None,
    }
}
/// Touche répétée : code XKB et prochaine répétition.
struct Held {
    key: u32,
    next: Instant,
}
pub(in crate::platform::linux) struct Keys {
    xkb: Option<Keyboard>,
    /// Répétitions par seconde (0 : pas de répétition) et délai avant la première.
    rate: i32,
    delay: Duration,
    held: Option<Held>,
}
impl Keys {
    pub fn new() -> Self {
        Self {
            xkb: None,
            rate: 25,
            delay: Duration::from_millis(600),
            held: None,
        }
    }
    /// Disposition donnée par le système (X11 : compilée d'après les noms du serveur) ;
    /// sans répétition côté client (le serveur X répète lui-même).
    pub fn set_keyboard(&mut self, keyboard: Option<Keyboard>, repeat: bool) {
        self.xkb = keyboard;
        self.held = None;
        if !repeat {
            self.rate = 0;
        }
    }
    /// Symbole XKB de la touche `code` (evdev) dans l'état courant (0 sans disposition).
    pub fn keysym(&self, code: u32) -> u32 {
        self.xkb
            .as_ref()
            .map_or(0, |x| x.keysym(code + EVDEV_OFFSET))
    }
    /// Touche `code` (evdev) répétée par le système.
    pub fn repeated(&mut self, code: u32) -> Vec<UiEvent> {
        self.events(code + EVDEV_OFFSET, true)
    }
    /// `wl_keyboard.keymap` : disposition au format XKB texte dans `fd` (`size` octets).
    pub fn keymap(&mut self, format: u32, fd: Option<OwnedFd>, size: u32) {
        const XKB_V1: u32 = 1;
        let Some(fd) = fd.filter(|_| format == XKB_V1 && size > 0) else {
            return;
        };
        // La taille annoncée est bornée par celle du fichier : lire au-delà d'une
        // projection plus longue que le fichier lèverait SIGBUS.
        // SAFETY : `stat` est une structure C de champs entiers, valide à zéro ;
        // descripteur possédé, structure locale écrite pendant l'appel.
        let mut stat: libc::stat = unsafe { std::mem::zeroed() };
        if unsafe { libc::fstat(fd.as_raw_fd(), &mut stat) } != 0 || stat.st_size <= 0 {
            return;
        }
        let size = (size as usize).min(stat.st_size as usize);
        // SAFETY : projection privée en lecture de `size` octets, libérée ci-dessous.
        let map = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                size,
                libc::PROT_READ,
                libc::MAP_PRIVATE,
                fd.as_raw_fd(),
                0,
            )
        };
        if map == libc::MAP_FAILED {
            return;
        }
        // SAFETY : `size` octets lisibles ; la disposition se termine par un octet nul.
        let bytes = unsafe { std::slice::from_raw_parts(map.cast::<u8>(), size) };
        let text = std::ffi::CStr::from_bytes_until_nul(bytes)
            .ok()
            .map(ToOwned::to_owned);
        // SAFETY : projection possédée, libérée une fois.
        unsafe { libc::munmap(map, size) };
        if let Some(text) = text {
            self.xkb = Keyboard::new(&text, &locale()).ok();
        }
    }
    pub fn modifiers(&mut self, depressed: u32, latched: u32, locked: u32, group: u32) {
        if let Some(xkb) = &self.xkb {
            xkb.update(depressed, latched, locked, group);
        }
    }
    pub fn repeat_info(&mut self, rate: i32, delay: i32) {
        self.rate = rate.max(0);
        self.delay = Duration::from_millis(delay.max(0) as u64);
    }
    /// Focus perdu : répétition arrêtée, composition abandonnée.
    pub fn reset(&mut self) {
        self.held = None;
        if let Some(xkb) = &self.xkb {
            xkb.reset_compose();
        }
    }
    /// Maj est enfoncée (appui qui étend une sélection).
    pub fn shift(&self) -> bool {
        self.modifiers_now().shift
    }
    fn modifiers_now(&self) -> Modifiers {
        let Some(xkb) = &self.xkb else {
            return Modifiers::default();
        };
        Modifiers {
            shift: xkb.active(c"Shift"),
            control: xkb.active(c"Control"),
            alt: xkb.active(c"Mod1"),
            logo: xkb.active(c"Mod4"),
        }
    }
    /// Touche enfoncée (`pressed`) ou relâchée : événements du moteur, dans l'ordre.
    pub fn key(&mut self, code: u32, pressed: bool, now: Instant) -> Vec<UiEvent> {
        let key = code + EVDEV_OFFSET;
        if !pressed {
            if self.held.as_ref().is_some_and(|h| h.key == key) {
                self.held = None;
            }
            let sym = self.xkb.as_ref().map_or(0, |x| x.keysym(key));
            return self::key(sym).map(UiEvent::KeyUp).into_iter().collect();
        }
        let repeats = self.xkb.as_ref().is_some_and(|x| x.repeats(key));
        self.held = (repeats && self.rate > 0).then(|| Held {
            key,
            next: now + self.delay,
        });
        self.events(key, false)
    }
    /// Événements d'une touche enfoncée ou répétée.
    fn events(&mut self, key: u32, repeat: bool) -> Vec<UiEvent> {
        let Some(xkb) = &self.xkb else {
            return Vec::new();
        };
        let sym = xkb.keysym(key);
        let m = self.modifiers_now();
        // Raccourcis d'édition : sous une disposition non latine (cyrillique, grecque…),
        // Ctrl+C vaut par la touche physique, comme sous Windows et macOS.
        let command = edit(sym, m).or_else(|| {
            m.control
                .then(|| xkb.latin_keysym(key))
                .flatten()
                .and_then(|latin| edit(latin, m))
        });
        if let Some(command) = command {
            return vec![UiEvent::Edit(command)];
        }
        if !repeat {
            match xkb.compose(sym) {
                Composed::Absorbed => return Vec::new(),
                Composed::Text(text) if !text.is_empty() => {
                    return vec![UiEvent::Text(text.into())];
                }
                _ => {}
            }
        }
        let mut events = Vec::new();
        if sym == 0xff67 {
            // Touche Menu : menu contextuel du contrôle focalisé.
            events.push(UiEvent::ContextMenu(None));
        }
        if let Some(key) = self::key(sym) {
            events.push(UiEvent::KeyDown {
                key,
                shift: m.shift,
                repeat,
            });
        }
        // Texte : hors raccourcis (Ctrl, Alt, Super) et caractères de contrôle.
        if !(m.control || m.alt || m.logo) {
            let text = xkb.text(key);
            if !text.is_empty() && !text.chars().any(char::is_control) {
                events.push(UiEvent::Text(text.into()));
            }
        }
        events
    }
    /// Prochaine répétition d'une touche tenue.
    pub fn next_repeat(&self) -> Option<Instant> {
        self.held.as_ref().map(|h| h.next)
    }
    /// Répétitions échues à `now`.
    pub fn repeat(&mut self, now: Instant) -> Vec<UiEvent> {
        let Some(held) = &mut self.held else {
            return Vec::new();
        };
        if held.next > now {
            return Vec::new();
        }
        if self.rate <= 0 {
            // `repeat_info` a désactivé la répétition pendant que la touche est tenue.
            self.held = None;
            return Vec::new();
        }
        held.next = now + Duration::from_secs_f64(1. / f64::from(self.rate));
        let key = held.key;
        self.events(key, true)
    }
}
/// Locale des séquences de composition (`LC_ALL`, `LC_CTYPE`, `LANG`, sinon `C`).
pub(in crate::platform::linux) fn locale() -> std::ffi::CString {
    ["LC_ALL", "LC_CTYPE", "LANG"]
        .iter()
        .filter_map(|name| std::env::var(name).ok())
        .find(|value| !value.is_empty())
        .and_then(|value| std::ffi::CString::new(value).ok())
        .unwrap_or_else(|| c"C".to_owned())
}
