//! Portails XDG du bureau (`org.freedesktop.portal.Desktop`) : dialogues de fichiers
//! (`file_chooser.rs`) et thème (`appearance.rs`), communs à Wayland et X11 et indépendants
//! du toolkit du bureau (GNOME, KDE, wlroots…).
//!
//! Un dialogue reste ouvert le temps que l'utilisateur choisisse : il est demandé et
//! attendu sur un thread à lui, avec sa propre connexion au bus de session, pour que la
//! boucle de fenêtres continue de peindre. Le choix revient par un canal propre à la
//! fenêtre (`Dialogs`), que la boucle vide avec les résultats des tâches de fond après
//! avoir été réveillée (`eventfd` de `shell::Wake`).
mod appearance;
mod file_chooser;
#[cfg(test)]
mod tests;

use crate::{
    task::{CancellationToken, TaskError, TaskId, TaskResult},
    window::{FileDialog, FileDialogResult},
};
use std::sync::{Arc, mpsc};

pub(in crate::platform::linux) use appearance::prefers_dark;

/// Réveil de la boucle de fenêtres, appelable depuis un autre thread.
pub(in crate::platform::linux) type Waker = Arc<dyn Fn() + Send + Sync>;

/// Dialogues ouverts par une fenêtre et choix à lui remettre. Détruit avec la fenêtre :
/// ses dialogues encore ouverts sont fermés et leurs choix ne sont pas remis.
///
/// Ces dialogues ne sont pas des tâches de fond : `TaskCommand::Cancel` ne les ferme pas
/// (comme sous Windows, où le dialogue est modal et synchrone).
pub(in crate::platform::linux) struct Dialogs {
    sender: mpsc::Sender<TaskResult>,
    receiver: mpsc::Receiver<TaskResult>,
    /// Annulé à la destruction : les threads ferment leur dialogue.
    closed: CancellationToken,
}
impl Dialogs {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            sender,
            receiver,
            closed: CancellationToken::new(),
        }
    }
    /// Montre `dialog` (identifiant de la fenêtre parente pour le portail : `x11:<XID>`,
    /// ou vide) ; le choix, ou l'échec, revient sous `task` après un appel à `wake`.
    pub fn open(&self, task: TaskId, dialog: FileDialog, parent: String, wake: Waker) {
        let sender = self.sender.clone();
        let deliver = move |result: TaskResult| {
            // Fenêtre fermée entre-temps : le canal n'a plus de lecteur, rien n'est remis.
            if sender.send(result).is_ok() {
                wake();
            }
        };
        let closed = self.closed.clone();
        let worker = deliver.clone();
        let spawned = std::thread::Builder::new()
            .name("karbon-ui-file-dialog".into())
            .spawn(move || {
                let outcome = file_chooser::show(&dialog, &parent, &closed);
                if !closed.is_cancelled() {
                    worker(result(task, outcome));
                }
            });
        if let Err(error) = spawned {
            deliver(TaskResult::failure(
                task,
                TaskError::StartFailed(error.to_string()),
            ));
        }
    }
    /// Prochain choix à remettre à l'application.
    pub fn take_next(&self) -> Option<TaskResult> {
        self.receiver.try_recv().ok()
    }
}
impl Drop for Dialogs {
    fn drop(&mut self) {
        self.closed.cancel();
    }
}
/// Résultat remis à l'application : les chemins choisis (aucun si l'utilisateur annule),
/// ou l'échec du portail.
fn result(task: TaskId, outcome: Result<Vec<std::path::PathBuf>, String>) -> TaskResult {
    match outcome {
        Ok(paths) => TaskResult::success(task, FileDialogResult { paths }),
        Err(message) => TaskResult::failure(task, TaskError::StartFailed(message)),
    }
}
