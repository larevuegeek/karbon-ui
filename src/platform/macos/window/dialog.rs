//! Dialogues natifs de fichiers : `NSOpenPanel` et `NSSavePanel`, en feuille attachée à
//! la fenêtre (modale à elle seule, comme sous Windows).
//!
//! La feuille rend la main tout de suite : la boucle d'événements continue de tourner
//! (dessin, animations, tâches de fond) et aucun emprunt des sessions n'est tenu pendant
//! que le panneau est ouvert. Le bloc de fin met le choix en file et réveille la boucle ;
//! `service` le livre ensuite à l'application de la fenêtre comme le résultat d'une
//! tâche. Fenêtre fermée entre-temps : rien n'est livré.
use crate::{
    task::{TaskError, TaskId, TaskResult},
    window::{FileDialog, FileDialogKind, FileDialogResult, WindowId},
};
use block2::RcBlock;
use objc2::{MainThreadMarker, Message, rc::Retained};
use objc2_app_kit::{
    NSModalResponse, NSModalResponseAbort, NSModalResponseOK, NSOpenPanel, NSSavePanel, NSWindow,
};
use objc2_foundation::{NSArray, NSString, NSURL};
use objc2_uniform_type_identifiers::UTType;
use std::{cell::RefCell, path::PathBuf};

thread_local! {
    /// Choix faits dans les panneaux fermés, pas encore livrés.
    static FINISHED: RefCell<Vec<(WindowId, TaskResult)>> = const { RefCell::new(Vec::new()) };
}

/// Panneau montré : le panneau d'ouverture (fichiers, dossier) ou d'enregistrement.
enum Panel {
    Open(Retained<NSOpenPanel>),
    Save(Retained<NSSavePanel>),
}

impl Panel {
    fn save_panel(&self) -> &NSSavePanel {
        match self {
            Panel::Open(panel) => panel,
            Panel::Save(panel) => panel,
        }
    }
    /// Chemins choisis après validation ; `None` si une URL n'est pas un chemin.
    fn paths(&self) -> Option<Vec<PathBuf>> {
        match self {
            Panel::Open(panel) => panel.URLs().iter().map(|url| url.to_file_path()).collect(),
            Panel::Save(panel) => panel
                .URL()
                .map(|url| url.to_file_path().map(|path| vec![path]))
                .unwrap_or(Some(Vec::new())),
        }
    }
}

/// Types de fichiers acceptés : les extensions de tous les filtres, dans l'ordre (la
/// première donne l'extension proposée à l'enregistrement). Vide si un filtre accepte
/// tout (`*`) : aucun type n'est alors imposé.
fn content_types(dialog: &FileDialog) -> Vec<Retained<UTType>> {
    let extensions = dialog.filters.iter().flat_map(|f| f.extensions.iter());
    if extensions.clone().any(|e| e == "*") {
        return Vec::new();
    }
    let mut types: Vec<Retained<UTType>> = Vec::new();
    for extension in extensions {
        let extension = extension.trim_start_matches('.');
        // Extension inconnue du système : type dynamique ; invalide : ignorée.
        if let Some(kind) = UTType::typeWithFilenameExtension(&NSString::from_str(extension))
            && !types
                .iter()
                .any(|known| known.identifier() == kind.identifier())
        {
            types.push(kind);
        }
    }
    types
}

/// Montre `dialog` en feuille sur `window` (session `id`) ; le choix sera livré sous
/// `task` par [`take_finished`].
pub(super) fn show(
    id: WindowId,
    window: &NSWindow,
    task: TaskId,
    dialog: &FileDialog,
    mtm: MainThreadMarker,
) {
    let panel = match dialog.kind {
        FileDialogKind::SaveFile => Panel::Save(NSSavePanel::savePanel(mtm)),
        kind => {
            let panel = NSOpenPanel::openPanel(mtm);
            let folder = kind == FileDialogKind::PickFolder;
            panel.setCanChooseFiles(!folder);
            panel.setCanChooseDirectories(folder);
            panel.setAllowsMultipleSelection(kind == FileDialogKind::OpenFiles);
            Panel::Open(panel)
        }
    };
    let save = panel.save_panel();
    if let Some(title) = &dialog.title {
        // Une feuille n'affiche pas de barre de titre : le titre sert aussi de message.
        let title = NSString::from_str(title);
        save.setTitle(Some(&title));
        save.setMessage(Some(&title));
    }
    // Dossier de départ introuvable : celui que choisit le système.
    if let Some(directory) = &dialog.directory
        && let Some(url) = NSURL::from_directory_path(directory)
    {
        save.setDirectoryURL(Some(&url));
    }
    if dialog.kind == FileDialogKind::SaveFile {
        save.setCanCreateDirectories(true);
        if let Some(name) = &dialog.file_name {
            save.setNameFieldStringValue(&NSString::from_str(name));
        }
    }
    if dialog.kind != FileDialogKind::PickFolder {
        let types = content_types(dialog);
        if !types.is_empty() {
            save.setAllowedContentTypes(&NSArray::from_retained_slice(&types));
        }
    }
    // Le bloc garde le panneau jusqu'à la fin ; AppKit le libère après l'appel, ce qui
    // rompt le cycle panneau → bloc → panneau.
    let shown = save.retain();
    let handler = RcBlock::new(move |response: NSModalResponse| {
        let result = if response == NSModalResponseOK {
            match panel.paths() {
                Some(paths) => TaskResult::success(task, FileDialogResult { paths }),
                None => TaskResult::failure(
                    task,
                    TaskError::StartFailed("file dialog returned a non-file URL".into()),
                ),
            }
        } else if response == NSModalResponseAbort {
            TaskResult::failure(
                task,
                TaskError::StartFailed("the file dialog could not be shown".into()),
            )
        } else {
            TaskResult::success(task, FileDialogResult::default())
        };
        FINISHED.with(|f| f.borrow_mut().push((id, result)));
        // Le bloc peut être appelé hors d'un événement (file principale) : la boucle est
        // réveillée pour livrer le choix sans attendre le prochain événement.
        super::wake();
    });
    shown.beginSheetModalForWindow_completionHandler(window, &handler);
}

/// Choix faits depuis le dernier appel, à livrer aux sessions.
pub(super) fn take_finished() -> Vec<(WindowId, TaskResult)> {
    FINISHED.with(|f| std::mem::take(&mut *f.borrow_mut()))
}
