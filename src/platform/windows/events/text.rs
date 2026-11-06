//! Saisie de texte : caractères UTF-16, caret système, composition IMM et messages
//! WM_CHAR, WM_UNICHAR et WM_IME_* du champ focalisé.
use super::{
    super::{
        ffi::{Hwnd, Point, Rect, scale, win_point, win_rect},
        present::runtime,
    },
    send_ui,
};
use crate::ui::runtime::UiEvent;
use std::{cell::Cell, ptr::null_mut, sync::Arc};
use windows_sys::Win32::UI::{
    Input::{
        Ime::{
            CANDIDATEFORM, CFS_EXCLUDE, CFS_POINT, COMPOSITIONFORM, CPS_CANCEL, GCS_COMPSTR,
            GCS_CURSORPOS, GCS_RESULTSTR, HIMC, IACE_DEFAULT, IME_COMPOSITION_STRING,
            ISC_SHOWUICOMPOSITIONWINDOW, ImmAssociateContextEx, ImmGetCompositionStringW,
            ImmGetContext, ImmNotifyIME, ImmReleaseContext, ImmSetCandidateWindow,
            ImmSetCompositionWindow, NI_COMPOSITIONSTR,
        },
        KeyboardAndMouse::GetFocus,
    },
    WindowsAndMessaging::{
        CreateCaret, DefWindowProcW, DestroyCaret, GetCaretBlinkTime, KillTimer, SetCaretPos,
        SetTimer, UNICODE_NOCHAR,
    },
};
const TIMER: usize = 0x5745;
struct Context {
    window: Hwnd,
    handle: HIMC,
}
impl Context {
    fn get(window: Hwnd) -> Option<Self> {
        // SAFETY : contexte IME emprunté, rendu par Drop sur la même fenêtre.
        let handle = unsafe { ImmGetContext(window) };
        (!handle.is_null()).then_some(Self { window, handle })
    }
}
impl Drop for Context {
    fn drop(&mut self) {
        // SAFETY : contexte obtenu par ImmGetContext pour cette fenêtre, rendu une fois.
        unsafe { ImmReleaseContext(self.window, self.handle) };
    }
}
#[derive(Default)]
pub(crate) struct Utf16Decoder {
    high: Option<u16>,
}
impl Utf16Decoder {
    fn feed(&mut self, unit: u16) -> String {
        if (0xD800..=0xDBFF).contains(&unit) {
            let old = self.high.replace(unit);
            return if old.is_some() {
                "�".into()
            } else {
                String::new()
            };
        }
        if let Some(high) = self.high.take() {
            if (0xDC00..=0xDFFF).contains(&unit) {
                return char::from_u32(
                    0x10000 + ((high as u32 - 0xD800) << 10) + (unit as u32 - 0xDC00),
                )
                .unwrap()
                .to_string();
            }
            return format!("�{}", char::from_u32(unit as u32).unwrap_or('�'));
        }
        char::from_u32(unit as u32).unwrap_or('�').to_string()
    }
}
static DECODER: super::state::WindowLocal<std::cell::RefCell<Utf16Decoder>> =
    super::state::WindowLocal::new(|s| &s.decoder);
static ACTIVE: super::state::WindowLocal<Cell<bool>> =
    super::state::WindowLocal::new(|s| &s.input_active);
static CARET_HEIGHT: super::state::WindowLocal<Cell<i32>> =
    super::state::WindowLocal::new(|s| &s.caret_height);
static IME_ENABLED: super::state::WindowLocal<Cell<bool>> =
    super::state::WindowLocal::new(|s| &s.ime_enabled);
static COMPOSING: super::state::WindowLocal<Cell<bool>> =
    super::state::WindowLocal::new(|s| &s.composing);

pub(super) fn shutdown(window: Hwnd) {
    // SAFETY : arrêt d'un timer identifié ; sans effet s'il n'existe pas.
    unsafe {
        KillTimer(window, TIMER);
    }
    if ACTIVE.with(|s| s.replace(false)) {
        // SAFETY : le caret de ce thread a été créé par `sync`.
        unsafe {
            DestroyCaret();
        }
    }
    CARET_HEIGHT.with(|s| s.set(0));
    COMPOSING.with(|s| s.set(false));
    reset();
}
pub(crate) fn character(unit: u16) -> Option<UiEvent> {
    if unit < 32 || unit == 127 {
        return None;
    }
    let value = DECODER.with(|d| d.borrow_mut().feed(unit));
    (!value.is_empty()).then(|| UiEvent::Text(value.into()))
}
fn composition_string(context: &Context, index: IME_COMPOSITION_STRING) -> Option<Arc<str>> {
    // SAFETY : tampon nul et longueur 0 : Windows renvoie seulement la taille requise.
    let size = unsafe { ImmGetCompositionStringW(context.handle, index, null_mut(), 0) };
    if !(0..=1_048_576).contains(&size) || size % 2 != 0 {
        return None;
    }
    let mut value = vec![0u16; size as usize / 2];
    // SAFETY : `value` offre exactement `size` octets en écriture.
    let actual = unsafe {
        ImmGetCompositionStringW(
            context.handle,
            index,
            value.as_mut_ptr().cast(),
            size as u32,
        )
    };
    if actual < 0 {
        return None;
    }
    value.truncate(actual as usize / 2);
    Some(String::from_utf16_lossy(&value).into())
}
fn ime(window: Hwnd, flags: isize, mut send: impl FnMut(UiEvent)) {
    let Some(context) = Context::get(window) else {
        return;
    };
    if flags & GCS_RESULTSTR as isize != 0 {
        COMPOSING.with(|c| c.set(false));
        if let Some(value) = composition_string(&context, GCS_RESULTSTR) {
            send(UiEvent::CompositionCommit(value));
        }
    }
    if flags & (GCS_COMPSTR | GCS_CURSORPOS) as isize != 0 {
        COMPOSING.with(|c| c.set(true));
        if let Some(value) = composition_string(&context, GCS_COMPSTR) {
            // SAFETY : GCS_CURSORPOS sans tampon renvoie directement la position.
            let cursor =
                unsafe { ImmGetCompositionStringW(context.handle, GCS_CURSORPOS, null_mut(), 0) }
                    .max(0) as usize;
            send(UiEvent::CompositionUpdate {
                value,
                cursor_utf16: cursor,
            });
        }
    }
    if flags == 0 {
        COMPOSING.with(|c| c.set(false));
        send(UiEvent::CompositionEnd);
    }
}
fn start() {
    COMPOSING.with(|c| c.set(true));
}
fn end() {
    COMPOSING.with(|c| c.set(false));
}
pub(super) fn reset() {
    DECODER.with(|d| d.borrow_mut().high = None);
}
pub(super) fn is_timer(id: usize) -> bool {
    id == TIMER
}
pub(crate) fn sync(window: Hwnd) {
    if !runtime::initialized() {
        return;
    }
    let info = runtime::focused_input();
    let visible = info
        .as_ref()
        .is_some_and(|i| i.bounds.right > i.bounds.left && i.bounds.bottom > i.bounds.top)
        // SAFETY : simple lecture de la fenêtre focalisée du thread.
        && unsafe { GetFocus() } == window;
    let was = ACTIVE.with(|s| s.replace(visible));
    if visible && !was {
        // SAFETY : simple lecture d'un réglage système.
        let delay = unsafe { GetCaretBlinkTime() };
        if delay != u32::MAX {
            // SAFETY : sans callback, Windows publie WM_TIMER dans la file de la fenêtre.
            unsafe { SetTimer(window, TIMER, if delay == 0 { 500 } else { delay }, None) };
        }
    }
    if !visible && was {
        // SAFETY : arrêt du timer et du caret créés pour cette fenêtre, sur ce thread.
        unsafe {
            KillTimer(window, TIMER);
            DestroyCaret();
        }
        CARET_HEIGHT.with(|h| h.set(0));
        reset();
    }
    let ime = visible && info.as_ref().is_some_and(|i| !i.read_only);
    if COMPOSING.with(Cell::get) && (!ime || info.as_ref().is_some_and(|i| !i.composing)) {
        if let Some(context) = Context::get(window) {
            // SAFETY : contexte IME valide jusqu'à la fin de ce bloc.
            unsafe { ImmNotifyIME(context.handle, NI_COMPOSITIONSTR, CPS_CANCEL, 0) };
        }
        COMPOSING.with(|c| c.set(false));
    }
    if IME_ENABLED.with(|s| s.replace(ime)) != ime {
        // SAFETY : contexte nul + IACE_DEFAULT rétablit le contexte par défaut de la fenêtre ;
        // contexte nul + 0 le dissocie.
        unsafe { ImmAssociateContextEx(window, null_mut(), if ime { IACE_DEFAULT } else { 0 }) };
    }
    if let Some(info) = info.filter(|_| visible) {
        let scale = scale(window);
        let x = (info.caret.left.clamp(info.bounds.left, info.bounds.right) * scale).round() as i32;
        let y = (info.caret.top * scale).round() as i32;
        let height = ((info.caret.bottom - info.caret.top) * scale)
            .round()
            .max(1.) as i32;
        if CARET_HEIGHT.with(|h| h.replace(height)) != height {
            // SAFETY : caret plein (bitmap nul) pour la fenêtre focalisée de ce thread.
            unsafe { CreateCaret(window, null_mut(), 1, height) };
        }
        // SAFETY : déplace le caret de ce thread ; aucun pointeur transmis.
        unsafe { SetCaretPos(x, y) };
        if let Some(context) = Context::get(window) {
            let area = win_rect(Rect {
                left: x,
                top: y,
                right: x + 1,
                bottom: y + height,
            });
            let position = win_point(Point { x, y: y + height });
            // SAFETY : contexte IME valide et structures locales lues pendant les appels.
            unsafe {
                ImmSetCompositionWindow(
                    context.handle,
                    &COMPOSITIONFORM {
                        dwStyle: CFS_POINT,
                        ptCurrentPos: position,
                        rcArea: area,
                    },
                );
                ImmSetCandidateWindow(
                    context.handle,
                    &CANDIDATEFORM {
                        dwIndex: 0,
                        dwStyle: CFS_EXCLUDE,
                        ptCurrentPos: position,
                        rcArea: area,
                    },
                );
            }
        }
    }
}
/// WM_CHAR.
pub(super) fn character_message(
    window: Hwnd,
    wparam: usize,
    default_proc: impl FnOnce() -> isize,
) -> isize {
    if let Some(event) = character(wparam as u16)
        && send_ui(window, event)
    {
        0
    } else {
        default_proc()
    }
}
/// WM_UNICHAR.
pub(super) fn unicode_character(window: Hwnd, wparam: usize) -> isize {
    if wparam == UNICODE_NOCHAR as usize {
        1
    } else if let Some(c) = char::from_u32(wparam as u32) {
        send_ui(window, UiEvent::Text(c.to_string().into()));
        0
    } else {
        0
    }
}
/// WM_IME_SETCONTEXT.
///
/// # Safety
/// Arguments du message courant, reçus de Windows pour cette même fenêtre.
pub(super) unsafe fn set_context(
    window: Hwnd,
    message: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    // Le champ dessine lui-même la composition : masquer la fenêtre IME par défaut.
    let flags = if runtime::focused_input().is_some() {
        lparam & !(ISC_SHOWUICOMPOSITIONWINDOW as isize)
    } else {
        lparam
    };
    // SAFETY : arguments reçus de Windows, seul un bit d'affichage de lparam change.
    unsafe { DefWindowProcW(window, message, wparam, flags) }
}
/// WM_IME_STARTCOMPOSITION ; sans champ focalisé, Windows garde la composition.
pub(super) fn start_composition(window: Hwnd, default_proc: impl FnOnce() -> isize) -> isize {
    if runtime::focused_input().is_none() {
        return default_proc();
    }
    start();
    send_ui(window, UiEvent::CompositionStart);
    0
}
/// WM_IME_COMPOSITION.
pub(super) fn composition(
    window: Hwnd,
    lparam: isize,
    default_proc: impl FnOnce() -> isize,
) -> isize {
    if runtime::focused_input().is_none() {
        return default_proc();
    }
    ime(window, lparam, |event| {
        send_ui(window, event);
    });
    0
}
/// WM_IME_ENDCOMPOSITION.
pub(super) fn end_composition(window: Hwnd, default_proc: impl FnOnce() -> isize) -> isize {
    if runtime::focused_input().is_none() {
        return default_proc();
    }
    end();
    send_ui(window, UiEvent::CompositionEnd);
    0
}
/// WM_IME_CHAR : résultat déjà traité par GCS_RESULTSTR quand un champ est focalisé.
pub(super) fn ime_character(default_proc: impl FnOnce() -> isize) -> isize {
    if runtime::focused_input().is_none() {
        return default_proc();
    }
    0
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn utf16_pairs_and_recovery_preserve_unicode() {
        let mut d = Utf16Decoder::default();
        assert_eq!(d.feed(0xD83D), "");
        assert_eq!(d.feed(0xDE03), "😃");
        assert_eq!(d.feed('é' as u16), "é");
        d.feed(0xD800);
        assert_eq!(d.feed('a' as u16), "�a");
        assert_eq!(d.feed(0xDC00), "�");
    }
}
