//! Background work: `Send` data, cooperative cancellation, and results delivered back
//! to the UI thread.
//!
//! An application queues [`TaskCommand`]s from
//! [`Application::take_task_commands`](crate::application::Application::take_task_commands);
//! each finished task comes back as a [`TaskResult`] through
//! [`Application::task_completed`](crate::application::Application::task_completed), on the
//! UI thread. Tasks run on a small shared worker pool; a window can have at most 64 tasks
//! in flight.
use std::{
    any::Any,
    fmt,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

mod executor;
pub(crate) use executor::{TaskExecutor, TaskScope};

/// Identifier chosen by the application for a task, unique within its window.
///
/// Spawning a task with an ID that is already running cancels the previous one; only the
/// latest result for an ID is delivered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(pub u64);

/// Cooperative cancellation flag handed to a task's work closure.
///
/// Cancellation never interrupts a running closure: long-running work should poll
/// [`is_cancelled`](Self::is_cancelled) and return early. Results of cancelled tasks are
/// discarded.
#[derive(Clone)]
pub struct CancellationToken(Arc<AtomicBool>);
impl CancellationToken {
    /// Return `true` once the task has been cancelled, replaced or its window closed.
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
    pub(crate) fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }
    pub(crate) fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
}

type Value = Box<dyn Any + Send>;
/// A unit of background work: a closure run once on a worker thread.
pub struct Task {
    /// Identifier reported back in the matching [`TaskResult`].
    pub id: TaskId,
    work: Box<dyn FnOnce(CancellationToken) -> Value + Send>,
}
impl Task {
    /// Create a task running `work` off the UI thread.
    ///
    /// The value returned by `work` is delivered as a [`TaskResult`]; read it with
    /// [`TaskResult::into_value::<T>`](TaskResult::into_value). A panic inside `work` is
    /// caught and reported as [`TaskError::WorkerPanicked`].
    pub fn new<T: Send + 'static>(
        id: TaskId,
        work: impl FnOnce(CancellationToken) -> T + Send + 'static,
    ) -> Self {
        Self {
            id,
            work: Box::new(move |cancel| Box::new(work(cancel))),
        }
    }
}

/// Request returned by [`Application::take_task_commands`](crate::application::Application::take_task_commands).
pub enum TaskCommand {
    /// Start a task. Replaces any previous request with the same ID in this window.
    Spawn(Task),
    /// Cancel the task with this ID and drop its pending result, if any.
    Cancel(TaskId),
    /// Cancel every task of this window and drop their pending results.
    CancelAll,
}

/// Reason a task produced no value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskError {
    /// The window already has the maximum number of tasks (64) in flight.
    QueueFull,
    /// The executor is shutting down and no longer accepts work.
    ExecutorClosed,
    /// The task's closure panicked.
    WorkerPanicked,
    /// The worker pool could not be started; carries the OS error message.
    StartFailed(String),
    /// [`TaskResult::into_value`] was called with a type other than the one returned
    /// by the task.
    TypeMismatch,
}
impl fmt::Display for TaskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::QueueFull => f.write_str("task capacity reached"),
            Self::ExecutorClosed => f.write_str("task executor closed"),
            Self::WorkerPanicked => f.write_str("task worker panicked"),
            Self::StartFailed(error) => write!(f, "task executor startup failed: {error}"),
            Self::TypeMismatch => f.write_str("unexpected task result type"),
        }
    }
}
impl std::error::Error for TaskError {}

/// Outcome of a task, delivered on the UI thread to
/// [`Application::task_completed`](crate::application::Application::task_completed).
pub struct TaskResult {
    /// Identifier of the task that produced this result.
    pub id: TaskId,
    value: Result<Value, TaskError>,
}
impl TaskResult {
    /// Build a successful result carrying `value` (mainly useful for tests).
    pub fn success<T: Send + 'static>(id: TaskId, value: T) -> Self {
        Self {
            id,
            value: Ok(Box::new(value)),
        }
    }
    /// Take the task's value as `T`, the type returned by its closure.
    ///
    /// Returns the task's [`TaskError`] if it failed, or [`TaskError::TypeMismatch`] if
    /// `T` is not the returned type. Domain errors can be returned with
    /// `T = Result<MyValue, MyError>`.
    pub fn into_value<T: Send + 'static>(self) -> Result<T, TaskError> {
        self.value?
            .downcast::<T>()
            .map(|value| *value)
            .map_err(|_| TaskError::TypeMismatch)
    }
    /// Build a failed result carrying `error` (mainly useful for tests).
    pub fn failure(id: TaskId, error: TaskError) -> Self {
        Self {
            id,
            value: Err(error),
        }
    }
}
