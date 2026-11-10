use super::*;
pub(super) use crate::{
    task::{CancellationToken, TaskError, TaskId, TaskResult},
    window::{FileDialog, FileDialogResult},
};
pub(super) use std::sync::{Arc, mpsc};

/// Réveil de la boucle de fenêtres, appelable depuis un autre thread.
pub(in crate::platform::linux) type Waker = Arc<dyn Fn() + Send + Sync>;

/// Dialogues ouverts par une fenêtre et choix à lui remettre. Détruit avec la fenêtre :
/// ses dialogues encore ouverts sont fermés et leurs choix ne sont pas remis.
///
/// Ces dialogues ne sont pas des tâches de fond : `TaskCommand::Cancel` ne les ferme pas
/// (comme sous Windows, où le dialogue est modal et synchrone).
pub(in crate::platform::linux) struct Dialogs {
    pub(super) sender: mpsc::Sender<TaskResult>,
    pub(super) receiver: mpsc::Receiver<TaskResult>,
    /// Annulé à la destruction : les threads ferment leur dialogue.
    pub(super) closed: CancellationToken,
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
pub(super) fn result(task: TaskId, outcome: Result<Vec<std::path::PathBuf>, String>) -> TaskResult {
    match outcome {
        Ok(paths) => TaskResult::success(task, FileDialogResult { paths }),
        Err(message) => TaskResult::failure(task, TaskError::StartFailed(message)),
    }
}
