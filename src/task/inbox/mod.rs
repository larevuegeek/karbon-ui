//! [`Inbox`]: the application's mailbox, posted to from any thread.
// Organisation : `channel` (boîte, expéditeur, abonnement de la fenêtre).
mod channel;
#[cfg(test)]
mod tests;
pub(crate) use channel::{INBOX_TASK, InboxPort, Listener};
pub use channel::{Inbox, InboxSender};
