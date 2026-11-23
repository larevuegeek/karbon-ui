//! Presse-papiers de l'application (`crate::clipboard`) : du texte Unicode et des formats
//! nommés par l'application (`RegisterClipboardFormatW`), écrits ensemble. Le
//! presse-papiers est ouvert au nom d'une fenêtre « message seulement » faite pour
//! l'occasion : sans propriétaire, `SetClipboardData` peut échouer après `EmptyClipboard`.
use super::ffi::{Hwnd, last_error};
use crate::clipboard::ClipboardContent;
use std::time::Duration;
use windows_sys::Win32::{
    Foundation::{GlobalFree, HGLOBAL},
    System::{
        DataExchange::{
            CloseClipboard, EmptyClipboard, GetClipboardData, IsClipboardFormatAvailable,
            OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
        },
        Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock},
        Ole::CF_UNICODETEXT,
    },
    UI::WindowsAndMessaging::{CreateWindowExW, DestroyWindow, HWND_MESSAGE},
};

/// Plafond d'un bloc relu (texte ou format).
const MAX_READ: usize = 256 * 1024 * 1024;

/// `text` en UTF-16 terminé par un nul.
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

/// Une fenêtre « message seulement », propriétaire du presse-papiers le temps d'un appel.
struct Owner(Hwnd);
impl Owner {
    fn new() -> crate::Result<Self> {
        let class = wide("STATIC");
        // SAFETY : classe prédéfinie, fenêtre sans parent visible (HWND_MESSAGE), aucun
        // pointeur retenu par le système au-delà de l'appel.
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                class.as_ptr(),
                std::ptr::null(),
                0,
                0,
                0,
                0,
                0,
                HWND_MESSAGE,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null(),
            )
        };
        if hwnd.is_null() {
            return Err(last_error("CreateWindowExW"));
        }
        Ok(Self(hwnd))
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        // SAFETY : fenêtre créée par `new` sur ce fil ; les données écrites restent au
        // presse-papiers (aucun rendu différé).
        unsafe { DestroyWindow(self.0) };
    }
}

/// Le presse-papiers ouvert, refermé à la fin. Un autre programme peut le tenir un
/// instant : quelques essais espacés de 10 ms.
struct Open;
impl Open {
    fn new(owner: &Owner) -> crate::Result<Self> {
        for attempt in 0..10 {
            // SAFETY : fenêtre valide ; refermé par Drop.
            if unsafe { OpenClipboard(owner.0) } != 0 {
                return Ok(Self);
            }
            if attempt < 9 {
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        Err(last_error("OpenClipboard"))
    }
}
impl Drop for Open {
    fn drop(&mut self) {
        // SAFETY : ouvert par ce fil dans `new`.
        unsafe { CloseClipboard() };
    }
}

/// Le numéro du format nommé `name`, enregistré au besoin.
fn format_of(name: &str) -> crate::Result<u32> {
    let name = wide(name);
    // SAFETY : nom terminé par un nul, lu pendant l'appel seulement.
    let format = unsafe { RegisterClipboardFormatW(name.as_ptr()) };
    if format == 0 {
        return Err(last_error("RegisterClipboardFormatW"));
    }
    Ok(format)
}

/// Un bloc global mobile qui contient `bytes`, à donner au presse-papiers.
fn global_block(bytes: &[u8]) -> crate::Result<HGLOBAL> {
    // SAFETY : taille d'au moins un octet ; le bloc verrouillé reçoit exactement
    // `bytes.len()` octets puis est déverrouillé, ou libéré s'il ne se verrouille pas.
    unsafe {
        let memory = GlobalAlloc(GMEM_MOVEABLE, bytes.len().max(1));
        if memory.is_null() {
            return Err(last_error("GlobalAlloc"));
        }
        let data = GlobalLock(memory);
        if data.is_null() {
            GlobalFree(memory);
            return Err(last_error("GlobalLock"));
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), data.cast::<u8>(), bytes.len());
        GlobalUnlock(memory);
        Ok(memory)
    }
}

/// Donne `bytes` au presse-papiers ouvert, dans `format`.
fn put(format: u32, bytes: &[u8]) -> crate::Result<()> {
    let memory = global_block(bytes)?;
    // SAFETY : presse-papiers ouvert par ce fil et vidé ; en cas de succès Windows
    // devient propriétaire du bloc, sinon il est libéré une seule fois.
    unsafe {
        if SetClipboardData(format, memory).is_null() {
            GlobalFree(memory);
            return Err(last_error("SetClipboardData"));
        }
    }
    Ok(())
}

/// Un bloc à écrire : la longueur exacte sur huit octets (`GlobalSize` arrondit), puis
/// les octets.
fn frame(bytes: &[u8]) -> Vec<u8> {
    let mut block = (bytes.len() as u64).to_le_bytes().to_vec();
    block.extend_from_slice(bytes);
    block
}

/// Le contenu d'un bloc écrit par `frame` ; `None` si le bloc ne vient pas de nous (trop
/// court, longueur impossible).
fn unframe(block: &[u8]) -> Option<Vec<u8>> {
    let head: [u8; 8] = block.get(..8)?.try_into().ok()?;
    let length = usize::try_from(u64::from_le_bytes(head)).ok()?;
    block
        .get(8..8usize.checked_add(length)?)
        .map(<[u8]>::to_vec)
}

pub(crate) fn write(content: &ClipboardContent) -> crate::Result<()> {
    let formats: Vec<(u32, &[u8])> = content
        .formats
        .iter()
        .map(|(name, bytes)| Ok((format_of(name)?, bytes.as_slice())))
        .collect::<crate::Result<_>>()?;
    let owner = Owner::new()?;
    let _open = Open::new(&owner)?;
    // SAFETY : presse-papiers ouvert par ce fil, au nom de `owner`.
    if unsafe { EmptyClipboard() } == 0 {
        return Err(last_error("EmptyClipboard"));
    }
    if let Some(text) = &content.text {
        let bytes: Vec<u8> = wide(text).iter().flat_map(|u| u.to_le_bytes()).collect();
        put(u32::from(CF_UNICODETEXT), &bytes)?;
    }
    for (format, bytes) in formats {
        put(format, &frame(bytes))?;
    }
    Ok(())
}

/// Les octets du presse-papiers dans `format`, s'il y en a.
fn take(format: u32) -> crate::Result<Option<Vec<u8>>> {
    // SAFETY : simple question au système.
    if unsafe { IsClipboardFormatAvailable(format) } == 0 {
        return Ok(None);
    }
    let owner = Owner::new()?;
    let _open = Open::new(&owner)?;
    // SAFETY : presse-papiers ouvert ; le bloc reste à Windows et n'est que lu, verrouillé
    // le temps de la copie de ses `size` octets.
    unsafe {
        let memory = GetClipboardData(format);
        if memory.is_null() {
            return Ok(None);
        }
        let size = GlobalSize(memory);
        if size > MAX_READ {
            return Err(crate::Error::invalid_input(
                "clipboard block larger than 256 MiB",
            ));
        }
        let data = GlobalLock(memory);
        if data.is_null() {
            return Err(last_error("GlobalLock"));
        }
        let bytes = std::slice::from_raw_parts(data.cast::<u8>(), size).to_vec();
        GlobalUnlock(memory);
        Ok(Some(bytes))
    }
}

pub(crate) fn read_text() -> crate::Result<Option<String>> {
    let Some(bytes) = take(u32::from(CF_UNICODETEXT))? else {
        return Ok(None);
    };
    let units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u16::from_le_bytes(*pair))
        .collect();
    let end = units.iter().position(|&u| u == 0).unwrap_or(units.len());
    Ok(Some(String::from_utf16_lossy(&units[..end])))
}

pub(crate) fn read_format(name: &str) -> crate::Result<Option<Vec<u8>>> {
    let Some(block) = take(format_of(name)?)? else {
        return Ok(None);
    };
    Ok(unframe(&block))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_block_keeps_its_exact_length_through_its_header() {
        let mut block = frame(&[1, 2, 3, 4, 5]);
        block.extend_from_slice(&[0, 0, 0]); // l'arrondi du système
        assert_eq!(unframe(&block), Some(vec![1, 2, 3, 4, 5]));
        // Témoins : un bloc trop court, une longueur qui dépasse le bloc.
        assert_eq!(unframe(&[1, 2]), None);
        let mut long = 50u64.to_le_bytes().to_vec();
        long.push(1);
        assert_eq!(unframe(&long), None);
        assert_eq!(unframe(&u64::MAX.to_le_bytes()), None);
    }

    /// Écrit dans le vrai presse-papiers de la session (puis y remet le texte d'avant) :
    /// à la demande.
    #[test]
    #[ignore = "écrit le presse-papiers de la session"]
    fn text_and_an_own_format_go_through_the_system_clipboard() {
        let before = read_text().unwrap();
        let content =
            ClipboardContent::text("Caisse, Tonneau").with("karbon-ui/essai", vec![7, 0, 255]);
        write(&content).unwrap();
        assert_eq!(read_text().unwrap().as_deref(), Some("Caisse, Tonneau"));
        assert_eq!(
            read_format("karbon-ui/essai").unwrap(),
            Some(vec![7, 0, 255])
        );
        // Un texte seul efface le format : il ne reste pas d'anciennes données.
        write(&ClipboardContent::text("seul")).unwrap();
        assert_eq!(read_format("karbon-ui/essai").unwrap(), None);
        if let Some(text) = before {
            write(&ClipboardContent::text(text)).unwrap();
        }
    }
}
