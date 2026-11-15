//! Background work: `Send` data, cooperative cancellation, and results delivered back
//! to the UI thread.
//!
//! An application queues [`TaskCommand`]s from
//! [`Application::take_task_commands`](crate::application::Application::take_task_commands);
//! each finished task comes back as a [`TaskResult`] through
//! [`Application::task_completed`](crate::application::Application::task_completed), on the
//! UI thread. Tasks run on a small shared worker pool; a window can have at most 64 tasks
//! in flight.
//!
//! Work that lives outside karbon-ui (an async runtime, a thread of the application, a
//! device callback) reaches the application through its [`Inbox`]: any thread posts a
//! closure or a message, the window wakes up and applies it on the UI thread.
// Organisation : `types` (identifiants, tâches, résultats), `executor` (réserve
// d'ouvriers, tâches d'une fenêtre, réveil remis aux boîtes aux lettres), `inbox` (boîte
// aux lettres de l'application).
mod executor;
mod inbox;
mod types;
pub(crate) use executor::{Port, TaskExecutor, TaskScope};
pub(crate) use inbox::{INBOX_TASK, InboxPort};
pub use inbox::{Inbox, InboxSender};
pub use types::*;
