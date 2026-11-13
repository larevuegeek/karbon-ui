//! Dialogues natifs de fichiers : `IFileOpenDialog` et `IFileSaveDialog` du Shell, modaux
//! à la fenêtre. Montrés depuis la boucle de messages, hors de tout emprunt de
//! l'application : la boucle modale du dialogue continue de livrer les messages de la
//! fenêtre (dessin, minuteries).
use super::super::ffi::Hwnd;
use crate::window::{FileDialog, FileDialogKind};
use std::{ffi::OsString, os::windows::ffi::OsStringExt, path::PathBuf};
use windows::{
    Win32::{
        Foundation::{ERROR_CANCELLED, HWND},
        System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance, CoTaskMemFree},
        UI::Shell::{
            Common::COMDLG_FILTERSPEC, FOS_ALLOWMULTISELECT, FOS_FILEMUSTEXIST,
            FOS_FORCEFILESYSTEM, FOS_OVERWRITEPROMPT, FOS_PATHMUSTEXIST, FOS_PICKFOLDERS,
            FileOpenDialog, FileSaveDialog, IFileDialog, IFileOpenDialog, IFileSaveDialog,
            IShellItem, SHCreateItemFromParsingName, SIGDN_FILESYSPATH,
        },
    },
    core::{HSTRING, Interface, PCWSTR},
};

fn failure(operation: &'static str, error: windows::core::Error) -> crate::Error {
    crate::Error::Platform {
        operation,
        code: error.code().0,
    }
}

/// Montre `dialog`, modal à `window` ; les chemins choisis, vides si l'utilisateur annule.
pub(super) fn show(window: Hwnd, dialog: &FileDialog) -> crate::Result<Vec<PathBuf>> {
    let save = dialog.kind == FileDialogKind::SaveFile;
    // SAFETY: COM est initialisé en appartement monothread sur le thread de l'interface ;
    // les CLSID du Shell sont ceux des interfaces demandées.
    let file: IFileDialog = unsafe {
        if save {
            CoCreateInstance::<_, IFileSaveDialog>(&FileSaveDialog, None, CLSCTX_INPROC_SERVER)
                .and_then(|d| d.cast())
        } else {
            CoCreateInstance::<_, IFileOpenDialog>(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)
                .and_then(|d| d.cast())
        }
    }
    .map_err(|e| failure("CoCreateInstance(FileDialog)", e))?;
    // Les chaînes doivent vivre jusqu'à la fin de `Show`.
    let names: Vec<HSTRING> = dialog
        .filters
        .iter()
        .map(|f| HSTRING::from(f.name.as_str()))
        .collect();
    let specs: Vec<HSTRING> = dialog
        .filters
        .iter()
        .map(|f| {
            HSTRING::from(
                f.extensions
                    .iter()
                    .map(|e| format!("*.{e}"))
                    .collect::<Vec<_>>()
                    .join(";"),
            )
        })
        .collect();
    let filters: Vec<COMDLG_FILTERSPEC> = names
        .iter()
        .zip(&specs)
        .map(|(name, spec)| COMDLG_FILTERSPEC {
            pszName: PCWSTR(name.as_ptr()),
            pszSpec: PCWSTR(spec.as_ptr()),
        })
        .collect();
    // SAFETY: dialogue créé ci-dessus ; les pointeurs des filtres restent valides (chaînes
    // gardées par `names` et `specs`) pendant tous les appels.
    unsafe {
        let mut options = file
            .GetOptions()
            .map_err(|e| failure("IFileDialog::GetOptions", e))?
            | FOS_FORCEFILESYSTEM
            | FOS_PATHMUSTEXIST;
        options |= match dialog.kind {
            FileDialogKind::OpenFile => FOS_FILEMUSTEXIST,
            FileDialogKind::OpenFiles => FOS_FILEMUSTEXIST | FOS_ALLOWMULTISELECT,
            FileDialogKind::PickFolder => FOS_PICKFOLDERS,
            FileDialogKind::SaveFile => FOS_OVERWRITEPROMPT,
        };
        file.SetOptions(options)
            .map_err(|e| failure("IFileDialog::SetOptions", e))?;
        if let Some(title) = &dialog.title {
            file.SetTitle(&HSTRING::from(title.as_str()))
                .map_err(|e| failure("IFileDialog::SetTitle", e))?;
        }
        if !filters.is_empty() && dialog.kind != FileDialogKind::PickFolder {
            file.SetFileTypes(&filters)
                .map_err(|e| failure("IFileDialog::SetFileTypes", e))?;
            if save && let Some(extension) = dialog.filters[0].extensions.first() {
                file.SetDefaultExtension(&HSTRING::from(extension.as_str()))
                    .map_err(|e| failure("IFileDialog::SetDefaultExtension", e))?;
            }
        }
        if let Some(name) = &dialog.file_name {
            file.SetFileName(&HSTRING::from(name.as_str()))
                .map_err(|e| failure("IFileDialog::SetFileName", e))?;
        }
        // Dossier de départ introuvable : celui que choisit le système.
        if let Some(directory) = &dialog.directory
            && let Ok(folder) = SHCreateItemFromParsingName::<_, _, IShellItem>(
                &HSTRING::from(directory.as_os_str()),
                None,
            )
        {
            let _ = file.SetFolder(&folder);
        }
        if let Err(error) = file.Show(Some(HWND(window))) {
            if error.code() == ERROR_CANCELLED.to_hresult() {
                return Ok(Vec::new());
            }
            return Err(failure("IFileDialog::Show", error));
        }
        let mut items = Vec::new();
        if dialog.kind == FileDialogKind::OpenFiles {
            let open: IFileOpenDialog = file.cast().map_err(|e| failure("IFileOpenDialog", e))?;
            let results = open
                .GetResults()
                .map_err(|e| failure("IFileOpenDialog::GetResults", e))?;
            let count = results
                .GetCount()
                .map_err(|e| failure("IShellItemArray::GetCount", e))?;
            for i in 0..count {
                items.push(
                    results
                        .GetItemAt(i)
                        .map_err(|e| failure("IShellItemArray::GetItemAt", e))?,
                );
            }
        } else {
            items.push(
                file.GetResult()
                    .map_err(|e| failure("IFileDialog::GetResult", e))?,
            );
        }
        items
            .iter()
            .map(|item| {
                let name = item
                    .GetDisplayName(SIGDN_FILESYSPATH)
                    .map_err(|e| failure("IShellItem::GetDisplayName", e))?;
                // Copie exacte des unités UTF-16 : un nom NTFS peut contenir un demi-codet
                // isolé, que `OsString` garde là où une `String` le refuserait.
                let path = PathBuf::from(OsString::from_wide(name.as_wide()));
                CoTaskMemFree(Some(name.0 as *const _));
                Ok(path)
            })
            .collect()
    }
}
