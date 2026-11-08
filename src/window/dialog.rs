//! Dialogues natifs de fichiers : ouvrir un ou plusieurs fichiers, choisir un dossier,
//! enregistrer sous. Demandés par [`WindowCommand::FileDialog`](super::WindowCommand),
//! le choix revient à l'application comme le résultat d'une tâche.
use std::path::PathBuf;

/// What a [`FileDialog`] asks the user to choose.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FileDialogKind {
    /// One existing file.
    #[default]
    OpenFile,
    /// One or more existing files.
    OpenFiles,
    /// One existing folder.
    PickFolder,
    /// A file name to save to (the file may not exist yet).
    SaveFile,
}

/// File type offered by a [`FileDialog`]: a label and its extensions, without the dot
/// (`"FLAC"`, `["flac"]`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FileFilter {
    /// Label shown in the type list.
    pub name: String,
    /// Extensions without the leading dot.
    pub extensions: Vec<String>,
}

/// Native file chooser, shown modal to its window by
/// [`WindowCommand::FileDialog`](super::WindowCommand::FileDialog).
///
/// ```
/// use karbon_ui::window::FileDialog;
///
/// let dialog = FileDialog::open_files()
///     .title("Add music")
///     .filter("Audio", &["mp3", "flac"]);
/// assert_eq!(dialog.filters[0].extensions, ["mp3", "flac"]);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FileDialog {
    /// What the user chooses.
    pub kind: FileDialogKind,
    /// Dialog title; the system default when `None`.
    pub title: Option<String>,
    /// File types offered, in order (the first one is selected). Ignored when picking a
    /// folder.
    pub filters: Vec<FileFilter>,
    /// File name proposed by a save dialog.
    pub file_name: Option<String>,
    /// Folder shown first; the system's choice when `None`.
    pub directory: Option<PathBuf>,
}

impl FileDialog {
    /// Choose one existing file.
    pub fn open_file() -> Self {
        Self::default()
    }
    /// Choose one or more existing files.
    pub fn open_files() -> Self {
        Self {
            kind: FileDialogKind::OpenFiles,
            ..Self::default()
        }
    }
    /// Choose one existing folder.
    pub fn pick_folder() -> Self {
        Self {
            kind: FileDialogKind::PickFolder,
            ..Self::default()
        }
    }
    /// Choose where to save a file.
    pub fn save_file() -> Self {
        Self {
            kind: FileDialogKind::SaveFile,
            ..Self::default()
        }
    }
    /// Set the dialog title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }
    /// Offer a file type: a label and its extensions, without the dot.
    pub fn filter(mut self, name: impl Into<String>, extensions: &[&str]) -> Self {
        self.filters.push(FileFilter {
            name: name.into(),
            extensions: extensions.iter().map(|e| (*e).to_owned()).collect(),
        });
        self
    }
    /// Propose a file name (save dialog).
    pub fn file_name(mut self, name: impl Into<String>) -> Self {
        self.file_name = Some(name.into());
        self
    }
    /// Show this folder first.
    pub fn directory(mut self, directory: impl Into<PathBuf>) -> Self {
        self.directory = Some(directory.into());
        self
    }
}

/// Choice made in a [`FileDialog`], delivered to
/// [`Application::task_completed`](crate::application::Application::task_completed) under
/// the [`TaskId`](crate::task::TaskId) given with the command (read it with
/// [`TaskResult::into_value`](crate::task::TaskResult::into_value)).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FileDialogResult {
    /// Chosen paths: empty when the user cancelled, one entry except for
    /// [`FileDialogKind::OpenFiles`].
    pub paths: Vec<PathBuf>,
}
