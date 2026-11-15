//! Le réveil qu'une fenêtre remet à une boîte aux lettres : un faux résultat de tâche
//! déposé dans sa boîte de résultats, livré par le chemin des tâches de chaque backend.
use super::super::*;
use super::pool::{Completion, Mailbox, lock};
use std::sync::Weak;

/// Accès d'une boîte aux lettres à la fenêtre qui l'écoute.
#[derive(Clone)]
pub(crate) struct Port {
    pub(super) id: TaskId,
    pub(super) revision: u64,
    pub(super) cancel: CancellationToken,
    pub(super) mailbox: Weak<Mailbox>,
}
impl Port {
    /// La fenêtre écoute encore (ni fermée, ni abonnement annulé).
    pub(crate) fn is_live(&self) -> bool {
        !self.cancel.is_cancelled()
            && self
                .mailbox
                .upgrade()
                .is_some_and(|mailbox| !lock(&mailbox.inbox).closed)
    }
    /// Dépose un réveil et réveille la fenêtre ; `false` si elle n'écoute plus.
    pub(crate) fn signal(&self) -> bool {
        let Some(mailbox) = self.mailbox.upgrade() else {
            return false;
        };
        {
            let mut inbox = lock(&mailbox.inbox);
            if inbox.closed || self.cancel.is_cancelled() {
                return false;
            }
            inbox.results.push_back(Completion {
                revision: self.revision,
                result: TaskResult::success(self.id, ()),
            });
        }
        (mailbox.wake)();
        true
    }
}
