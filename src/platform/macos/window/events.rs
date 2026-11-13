//! Traduction des événements AppKit en `UiEvent` : touches, raccourcis d'édition,
//! texte saisi, défilement ; curseurs et presse-papiers.
use crate::ui::{
    runtime::{Cursor, Key},
    text_input::EditCommand,
};
use objc2::rc::Retained;
use objc2_app_kit::{
    NSCursor, NSEvent, NSEventModifierFlags, NSPasteboard, NSPasteboardTypeString,
};
use objc2_foundation::NSString;

/// Touche du moteur d'un code de touche virtuel macOS (`kVK_*`, indépendant de la
/// disposition du clavier).
pub(super) fn key(code: u16) -> Option<Key> {
    Some(match code {
        0x30 => Key::Tab,
        0x24 | 0x4C => Key::Enter,
        0x31 => Key::Space,
        0x7B => Key::Left,
        0x7C => Key::Right,
        0x7D => Key::Down,
        0x7E => Key::Up,
        0x73 => Key::Home,
        0x77 => Key::End,
        0x74 => Key::PageUp,
        0x79 => Key::PageDown,
        0x35 => Key::Escape,
        0x33 => Key::Backspace,
        0x75 => Key::Delete,
        _ => return None,
    })
}
/// Modificateurs utiles d'un événement.
#[derive(Clone, Copy)]
pub(super) struct Modifiers {
    pub shift: bool,
    pub command: bool,
    pub option: bool,
    pub control: bool,
}
pub(super) fn modifiers(event: &NSEvent) -> Modifiers {
    let flags = event.modifierFlags();
    Modifiers {
        shift: flags.contains(NSEventModifierFlags::Shift),
        command: flags.contains(NSEventModifierFlags::Command),
        option: flags.contains(NSEventModifierFlags::Option),
        control: flags.contains(NSEventModifierFlags::Control),
    }
}
/// Caractère de la touche sans modificateur, en minuscule.
pub(super) fn base_character(event: &NSEvent) -> Option<char> {
    event
        .charactersIgnoringModifiers()
        .and_then(|s| s.to_string().chars().next())
        .map(|c| c.to_ascii_lowercase())
}
/// Raccourcis d'édition macOS : ⌘A/C/X/V/Z, ⇧⌘Z, ⌥← / ⌥→ (mot précédent, suivant).
pub(super) fn edit_command(event: &NSEvent, m: Modifiers) -> Option<EditCommand> {
    if m.command && !m.control && !m.option {
        return match base_character(event)? {
            'a' => Some(EditCommand::SelectAll),
            'c' => Some(EditCommand::Copy),
            'x' => Some(EditCommand::Cut),
            'v' => Some(EditCommand::Paste),
            'z' if m.shift => Some(EditCommand::Redo),
            'z' => Some(EditCommand::Undo),
            _ => None,
        };
    }
    if m.option && !m.command && !m.control {
        return match key(event.keyCode())? {
            Key::Left => Some(EditCommand::WordLeft { extend: m.shift }),
            Key::Right => Some(EditCommand::WordRight { extend: m.shift }),
            _ => None,
        };
    }
    None
}
/// Deltas d'un défilement en unités logiques (positifs vers le bas et la droite, comme le
/// moteur) et `true` pour une molette à crans (glissement doux).
pub(super) fn scroll(event: &NSEvent) -> (f32, f32, bool) {
    let (dx, dy) = (
        event.scrollingDeltaX() as f32,
        event.scrollingDeltaY() as f32,
    );
    // Molette classique : AppKit compte des lignes ; Windows défile 3 × 16 px par cran.
    let precise = event.hasPreciseScrollingDeltas();
    let unit = if precise { 1. } else { 16. };
    (-dx * unit, -dy * unit, !precise)
}
/// Curseur AppKit d'une forme du moteur (les formes absentes d'AppKit retombent sur la
/// flèche). Les curseurs de redimensionnement historiques restent utilisés : leurs
/// remplaçants n'existent qu'à partir de macOS 15.
#[allow(deprecated)]
pub(super) fn cursor(cursor: Cursor) -> Retained<NSCursor> {
    match cursor {
        Cursor::Hand => NSCursor::pointingHandCursor(),
        Cursor::ResizeHorizontal => NSCursor::resizeLeftRightCursor(),
        Cursor::ResizeVertical => NSCursor::resizeUpDownCursor(),
        Cursor::Text => NSCursor::IBeamCursor(),
        Cursor::NotAllowed => NSCursor::operationNotAllowedCursor(),
        Cursor::Move => NSCursor::openHandCursor(),
        Cursor::Crosshair => NSCursor::crosshairCursor(),
        Cursor::Arrow
        | Cursor::Wait
        | Cursor::Progress
        | Cursor::Help
        | Cursor::ResizeDiagonal
        | Cursor::ResizeAntiDiagonal => NSCursor::arrowCursor(),
    }
}
pub(super) fn write_clipboard(text: &str) {
    let board = NSPasteboard::generalPasteboard();
    board.clearContents();
    // SAFETY : constante d'AppKit.
    board.setString_forType(&NSString::from_str(text), unsafe { NSPasteboardTypeString });
}
pub(super) fn read_clipboard() -> Option<String> {
    let board = NSPasteboard::generalPasteboard();
    // SAFETY : constante d'AppKit.
    board
        .stringForType(unsafe { NSPasteboardTypeString })
        .map(|s| s.to_string())
}
