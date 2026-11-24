//! Le glisser qui sort de la fenêtre (KUI-047) : quand le runtime dit qu'un glisser de
//! données exportées (`Payload::exported_as`) a quitté la fenêtre, ses fichiers passent
//! au glisser d'OLE (`DoDragDrop`), que l'Explorateur ou un autre programme reçoit.
//! L'objet de données est celui du Shell (tableau d'éléments des fichiers), qui
//! donne `CF_HDROP` et les formats que l'Explorateur attend. La boucle de `DoDragDrop`
//! garde la main jusqu'au lâcher ; la source apprend ensuite la fin du glisser.
use super::super::{ffi::Hwnd, present::runtime};
use crate::ui::runtime::EventResult;
use std::{os::windows::ffi::OsStrExt, path::PathBuf};
use windows::{
    Win32::{
        Foundation::{DRAGDROP_S_CANCEL, DRAGDROP_S_DROP, DRAGDROP_S_USEDEFAULTCURSORS, S_OK},
        System::{
            Com::IDataObject,
            Ole::{
                DROPEFFECT, DROPEFFECT_COPY, DoDragDrop, IDropSource, IDropSource_Impl,
                OleInitialize,
            },
            SystemServices::{MK_LBUTTON, MODIFIERKEYS_FLAGS},
        },
        UI::Shell::{
            BHID_DataObject, Common::ITEMIDLIST, ILCreateFromPathW, ILFree,
            SHCreateShellItemArrayFromIDLists,
        },
    },
    core::{BOOL, HRESULT, PCWSTR, implement},
};

/// Après une entrée du pointeur : si un glisser vient de quitter la fenêtre avec des
/// fichiers, mène le glisser du système, puis dit au runtime s'ils ont été pris.
pub(crate) fn after_pointer(window: Hwnd) {
    let mut files = None;
    runtime::raw_input(window, false, |ui| {
        files = ui.take_outgoing_files();
        EventResult::default()
    });
    let Some(files) = files else {
        return;
    };
    // SAFETY : rend la capture que tenait le geste ; OLE suit la souris lui-même.
    unsafe {
        windows_sys::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture();
    }
    let dropped = drag_files(&files).unwrap_or(false);
    let result = runtime::raw_input(window, false, |ui| ui.outgoing_drag_done(dropped));
    if result.repaint {
        super::super::ffi::invalidate(window);
    }
}

/// Le glisser d'OLE de `files`, en copie ; `true` s'il a été lâché sur une cible.
fn drag_files(files: &[PathBuf]) -> windows::core::Result<bool> {
    let data = data_object(files)?;
    let source: IDropSource = DropSource.into();
    let mut effect = DROPEFFECT(0);
    // SAFETY : OLE initialisé sur le fil de la fenêtre (appartement unique) ; objets
    // COM valides pendant tout l'appel ; `effect` est une variable locale.
    let result = unsafe {
        let _ = OleInitialize(None);
        DoDragDrop(&data, &source, DROPEFFECT_COPY, &mut effect)
    };
    Ok(result == DRAGDROP_S_DROP && effect.0 & DROPEFFECT_COPY.0 != 0)
}

/// L'objet de données du Shell pour `files` (chemins absolus).
pub(super) fn data_object(files: &[PathBuf]) -> windows::core::Result<IDataObject> {
    let wide: Vec<Vec<u16>> = files
        .iter()
        .map(|path| path.as_os_str().encode_wide().chain([0]).collect())
        .collect();
    // SAFETY : chaînes terminées par un zéro, vivantes pendant l'appel ; chaque liste
    // d'identifiants rendue est libérée ci-dessous.
    let items: Vec<*mut ITEMIDLIST> = wide
        .iter()
        .map(|w| unsafe { ILCreateFromPathW(PCWSTR(w.as_ptr())) })
        .collect();
    let all = items.iter().all(|item| !item.is_null());
    let list: Vec<*const ITEMIDLIST> = items.iter().map(|&item| item.cast_const()).collect();
    // SAFETY : identifiants absolus valides ; le Shell les copie dans son tableau, dont
    // l'objet de données donne `CF_HDROP` et les formats du Shell.
    let data = if all {
        unsafe {
            SHCreateShellItemArrayFromIDLists(&list)
                .and_then(|array| array.BindToHandler(None, &BHID_DataObject))
        }
    } else {
        Err(windows::core::Error::from(
            windows::Win32::Foundation::E_INVALIDARG,
        ))
    };
    for item in items.into_iter().filter(|item| !item.is_null()) {
        // SAFETY : identifiant créé par `ILCreateFromPathW`, libéré une fois.
        unsafe { ILFree(Some(item.cast_const())) };
    }
    data
}

/// La source du glisser : il continue tant que le bouton gauche est tenu, Échap
/// l'annule, les curseurs sont ceux du système.
#[implement(IDropSource)]
struct DropSource;

impl IDropSource_Impl for DropSource_Impl {
    fn QueryContinueDrag(&self, escape: BOOL, keys: MODIFIERKEYS_FLAGS) -> HRESULT {
        if escape.as_bool() {
            DRAGDROP_S_CANCEL
        } else if keys.0 & MK_LBUTTON.0 == 0 {
            DRAGDROP_S_DROP
        } else {
            S_OK
        }
    }

    fn GiveFeedback(&self, _: DROPEFFECT) -> HRESULT {
        DRAGDROP_S_USEDEFAULTCURSORS
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::{
        System::{
            Com::{DVASPECT_CONTENT, FORMATETC, TYMED_HGLOBAL},
            Ole::{CF_HDROP, ReleaseStgMedium},
        },
        UI::Shell::{DragQueryFileW, HDROP},
    };

    /// Les chemins que donne le format `CF_HDROP` de `data`.
    fn paths_of(data: &IDataObject) -> Vec<PathBuf> {
        let format = FORMATETC {
            cfFormat: CF_HDROP.0,
            ptd: std::ptr::null_mut(),
            dwAspect: DVASPECT_CONTENT.0,
            lindex: -1,
            tymed: TYMED_HGLOBAL.0 as u32,
        };
        // SAFETY : objet valide ; le support rendu est relâché ; tampons à la taille dite.
        unsafe {
            let mut medium = data.GetData(&format).unwrap();
            let drop = HDROP(medium.u.hGlobal.0);
            let count = DragQueryFileW(drop, u32::MAX, None);
            let out = (0..count)
                .map(|i| {
                    let mut buffer = vec![0u16; 1024];
                    let len = DragQueryFileW(drop, i, Some(&mut buffer)) as usize;
                    PathBuf::from(String::from_utf16_lossy(&buffer[..len]))
                })
                .collect();
            ReleaseStgMedium(&mut medium);
            out
        }
    }

    #[test]
    fn the_data_object_of_two_files_gives_their_paths_to_the_explorer() {
        // SAFETY : initialisation d'OLE sur le fil de l'essai.
        let _ = unsafe { OleInitialize(None) };
        let folder = std::env::temp_dir().join(format!("karbon-drag-out-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let files = [folder.join("brique.png"), folder.join("chaise.glb")];
        for file in &files {
            std::fs::write(file, b"x").unwrap();
        }
        let data = data_object(&files).unwrap();
        assert_eq!(paths_of(&data), files);
        // Témoin : un chemin qui n'existe pas ne fait pas d'objet de données.
        assert!(data_object(&[folder.join("absent.png")]).is_err());
        std::fs::remove_dir_all(&folder).unwrap();
    }
}
