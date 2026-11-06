//! Presse-papiers Windows : texte Unicode lu et écrit pour les requêtes du runtime.
use super::super::ffi::{Hwnd, last_error};
use crate::ui::{runtime::UiEvent, text_input::ClipboardRequest};
use windows_sys::Win32::{
    Foundation::{GlobalFree, HGLOBAL},
    System::{
        DataExchange::{
            CloseClipboard, EmptyClipboard, GetClipboardData, OpenClipboard, SetClipboardData,
        },
        Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock},
        Ole::CF_UNICODETEXT,
    },
};

struct Clipboard;
impl Clipboard {
    fn open(window: Hwnd) -> crate::Result<Self> {
        // SAFETY : ouverture refermée par Drop ; aucun pointeur transmis.
        if unsafe { OpenClipboard(window) } == 0 {
            Err(last_error("OpenClipboard"))
        } else {
            Ok(Self)
        }
    }
}
impl Drop for Clipboard {
    fn drop(&mut self) {
        // SAFETY : le presse-papiers a été ouvert par ce thread dans `open`.
        unsafe { CloseClipboard() };
    }
}
fn write_clipboard(window: Hwnd, value: &str) -> crate::Result<()> {
    let encoded: Vec<u16> = value.encode_utf16().chain(Some(0)).collect();
    // SAFETY : allocation globale libérée ci-dessous, sauf si le presse-papiers l'adopte.
    let memory: HGLOBAL = unsafe { GlobalAlloc(GMEM_MOVEABLE, encoded.len() * 2) };
    if memory.is_null() {
        return Err(last_error("GlobalAlloc"));
    }
    let result = (|| {
        // SAFETY : `memory` est un bloc déplaçable valide, verrouillé puis déverrouillé.
        let data = unsafe { GlobalLock(memory) };
        if data.is_null() {
            return Err(last_error("GlobalLock"));
        }
        // SAFETY : le bloc verrouillé contient exactement `encoded.len()` unités UTF-16.
        unsafe {
            std::ptr::copy_nonoverlapping(encoded.as_ptr(), data.cast(), encoded.len());
            GlobalUnlock(memory);
        }
        let _clipboard = Clipboard::open(window)?;
        // SAFETY : presse-papiers ouvert par ce thread ; en cas de succès, Windows devient
        // propriétaire de `memory`.
        if unsafe { EmptyClipboard() } == 0 {
            return Err(last_error("EmptyClipboard"));
        }
        // SAFETY : voir ci-dessus.
        if unsafe { SetClipboardData(u32::from(CF_UNICODETEXT), memory) }.is_null() {
            return Err(last_error("SetClipboardData"));
        }
        Ok(())
    })();
    if result.is_err() {
        // SAFETY : le bloc n'a pas été adopté par le presse-papiers ; libéré une seule fois.
        unsafe { GlobalFree(memory) };
    }
    result
}
fn read_clipboard(window: Hwnd) -> crate::Result<Option<String>> {
    let _clipboard = Clipboard::open(window)?;
    // SAFETY : presse-papiers ouvert ; le bloc reste à Windows et n'est que lu.
    let memory = unsafe { GetClipboardData(u32::from(CF_UNICODETEXT)) };
    if memory.is_null() {
        return Ok(None);
    }
    // SAFETY : `memory` est un bloc global valide tant que le presse-papiers est ouvert.
    let size = unsafe { GlobalSize(memory) };
    if size > 16 * 1024 * 1024 {
        return Err(crate::Error::invalid_input(
            "clipboard text larger than 16 MiB",
        ));
    }
    // SAFETY : bloc global valide pendant l'ouverture du presse-papiers.
    let data = unsafe { GlobalLock(memory) };
    if data.is_null() {
        return Err(last_error("GlobalLock"));
    }
    // SAFETY : le bloc verrouillé fait `size` octets ; on n'en lit que des u16 complets
    // et la tranche n'est plus utilisée après GlobalUnlock.
    let value = unsafe { std::slice::from_raw_parts(data.cast::<u16>(), size / 2) };
    let len = value
        .iter()
        .position(|&unit| unit == 0)
        .unwrap_or(value.len());
    let result = String::from_utf16_lossy(&value[..len]);
    // SAFETY : déverrouille le bloc verrouillé ci-dessus.
    unsafe { GlobalUnlock(memory) };
    Ok(Some(result))
}
pub(super) fn clipboard(window: Hwnd, request: ClipboardRequest) -> crate::Result<Option<UiEvent>> {
    match request {
        ClipboardRequest::Write { text, cut } => {
            write_clipboard(window, &text)?;
            Ok(cut.then_some(UiEvent::Edit(
                crate::ui::text_input::EditCommand::DeleteSelection,
            )))
        }
        ClipboardRequest::Read => Ok(read_clipboard(window)?.map(|s| UiEvent::Text(s.into()))),
    }
}
