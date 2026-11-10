//! Background work: `Send` data, cooperative cancellation, and results delivered back
//! to the UI thread.
//!
//! An application queues [`TaskCommand`]s from
//! [`Application::take_task_commands`](crate::application::Application::take_task_commands);
//! each finished task comes back as a [`TaskResult`] through
//! [`Application::task_completed`](crate::application::Application::task_completed), on the
//! UI thread. Tasks run on a small shared worker pool; a window can have at most 64 tasks
//! in flight.
mod executor;
pub(crate) use executor::{TaskExecutor, TaskScope};
mod types;
pub use types::*;
