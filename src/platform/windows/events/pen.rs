//! Le stylet (Windows Ink) et la place de la fenêtre sur l'écran, pour `PointerEvent` :
//! `pressure`, `tilt` et `screen`.
//!
//! Windows envoie d'abord un message `WM_POINTER*` pour le stylet, puis, si la fenêtre le
//! laisse à `DefWindowProcW`, le message de souris qui en découle. On lit la pression et
//! l'inclinaison au premier, on laisse Windows faire le second comme avant, et le message
//! de souris porte alors ce qu'on a lu, s'il vient bien du stylet (signature de
//! `GetMessageExtraInfo`). Une souris garde une pression de 1.
use super::super::ffi::{Hwnd, scale};
use crate::ui::runtime::raw_input::{PenSample, pen_pressure};
use std::cell::Cell;
use windows_sys::Win32::{
    Foundation::POINT,
    Graphics::Gdi::ClientToScreen,
    UI::{
        HiDpi::GetDpiForSystem,
        Input::Pointer::{GetPointerPenInfo, GetPointerType, POINTER_PEN_INFO},
        WindowsAndMessaging::{
            GetMessageExtraInfo, PEN_MASK_PRESSURE, PEN_MASK_TILT_X, PEN_MASK_TILT_Y, PT_PEN,
        },
    },
};

/// Masque et signature des messages de souris faits par Windows d'après un stylet ou un
/// doigt (documentation de `GetMessageExtraInfo`).
const SIGNATURE_MASK: isize = 0xFFFF_FF00;
const FROM_PEN: isize = 0xFF51_5700;

thread_local! {
    /// La dernière lecture du stylet : pression de 0 à 1, inclinaison en degrés.
    static LAST_PEN: Cell<Option<PenSample>> = const { Cell::new(None) };
}

/// WM_POINTERDOWN, WM_POINTERUPDATE, WM_POINTERUP : lit le stylet, puis laisse Windows en
/// faire les messages de souris, comme avant.
pub(super) fn pointer_message(wparam: usize, default_proc: impl FnOnce() -> isize) -> isize {
    let id = (wparam & 0xFFFF) as u32;
    let mut kind = 0;
    // SAFETY : `kind` est une variable locale valide en écriture pendant l'appel.
    let pen = unsafe { GetPointerType(id, &mut kind) } != 0 && kind == PT_PEN;
    if pen {
        let mut info = POINTER_PEN_INFO::default();
        // SAFETY : `info` est une structure locale valide en écriture pendant l'appel.
        if unsafe { GetPointerPenInfo(id, &mut info) } != 0 {
            LAST_PEN.with(|last| last.set(Some(reading(&info))));
        }
    }
    default_proc()
}

/// La pression et l'inclinaison lues dans `info`, celles d'une souris pour ce que le
/// stylet ne dit pas.
fn reading(info: &POINTER_PEN_INFO) -> PenSample {
    let pressure = match info.penMask & PEN_MASK_PRESSURE {
        0 => 1.,
        _ => pen_pressure(info.pressure),
    };
    let tilt_x = match info.penMask & PEN_MASK_TILT_X {
        0 => 0.,
        _ => info.tiltX as f32,
    };
    let tilt_y = match info.penMask & PEN_MASK_TILT_Y {
        0 => 0.,
        _ => info.tiltY as f32,
    };
    (pressure, (tilt_x, tilt_y))
}

/// Le message de souris en cours vient-il d'un stylet ?
fn from_pen() -> bool {
    // SAFETY : simple lecture de l'information jointe au message courant du thread.
    let extra = unsafe { GetMessageExtraInfo() };
    extra & SIGNATURE_MASK == FROM_PEN
}

/// Ce que le runtime doit savoir avant une entrée du pointeur dans `window` : le stylet,
/// s'il fait le message en cours, le coin de la zone client sur l'écran (unités de
/// `WindowChange::Position`) et le rapport de ces unités à celles de la fenêtre.
pub(super) fn extras(window: Hwnd) -> (Option<PenSample>, (f32, f32), f32) {
    let pen = if from_pen() {
        LAST_PEN.with(Cell::get)
    } else {
        None
    };
    let mut corner = POINT { x: 0, y: 0 };
    // SAFETY : `corner` est une structure locale valide en lecture et écriture.
    unsafe { ClientToScreen(window, &mut corner) };
    // SAFETY : simple lecture du DPI système.
    let system = unsafe { GetDpiForSystem() }.max(96) as f32 / 96.;
    let origin = (corner.x as f32 / system, corner.y as f32 / system);
    (pen, origin, scale(window) / system)
}
