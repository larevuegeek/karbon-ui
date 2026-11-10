pub(super) use super::super::*;
pub(super) use std::{
    collections::{HashMap, VecDeque},
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{Condvar, Mutex, MutexGuard, Weak},
    thread::{self, JoinHandle},
};

pub(super) fn lock<T>(value: &Mutex<T>) -> MutexGuard<'_, T> {
    value.lock().unwrap_or_else(|error| error.into_inner())
}
pub(super) struct Completion {
    pub(super) revision: u64,
    pub(super) result: TaskResult,
}
pub(super) struct Inbox {
    pub(super) closed: bool,
    pub(super) results: VecDeque<Completion>,
}
pub(super) struct Mailbox {
    pub(super) inbox: Mutex<Inbox>,
    pub(super) wake: Arc<dyn Fn() + Send + Sync>,
}
pub(super) struct Running {
    pub(super) revision: u64,
    pub(super) cancel: CancellationToken,
}
pub(super) struct Job {
    pub(super) task: Task,
    pub(super) revision: u64,
    pub(super) cancel: CancellationToken,
    pub(super) mailbox: Weak<Mailbox>,
}
impl Job {
    pub(super) fn run(self) {
        if self.cancel.is_cancelled() {
            return;
        }
        let value = catch_unwind(AssertUnwindSafe(|| (self.task.work)(self.cancel.clone())))
            .map_err(|_| TaskError::WorkerPanicked);
        let Some(mailbox) = self.mailbox.upgrade() else {
            return;
        };
        {
            let mut inbox = lock(&mailbox.inbox);
            if inbox.closed || self.cancel.is_cancelled() {
                return;
            }
            inbox.results.push_back(Completion {
                revision: self.revision,
                result: TaskResult {
                    id: self.task.id,
                    value,
                },
            });
        }
        (mailbox.wake)();
    }
}
pub(super) struct Queue {
    pub(super) closed: bool,
    pub(super) jobs: VecDeque<Job>,
}
pub(super) struct Shared {
    pub(super) queue: Mutex<Queue>,
    pub(super) ready: Condvar,
    pub(super) capacity: usize,
}

/// Créé à la première demande ; les threads dorment sur une condition au repos.
pub(crate) struct TaskExecutor {
    pub(super) shared: Arc<Shared>,
    pub(super) _workers: Vec<JoinHandle<()>>,
}
impl TaskExecutor {
    pub(crate) fn for_ui() -> crate::Result<Self> {
        let workers = thread::available_parallelism()
            .map_or(2, |count| count.get().saturating_sub(1).clamp(1, 4));
        Self::new(workers, 64)
    }
    pub(crate) fn new(workers: usize, capacity: usize) -> crate::Result<Self> {
        if workers == 0 || capacity == 0 {
            return Err(crate::Error::invalid_input(
                "task capacity must be positive",
            ));
        }
        let shared = Arc::new(Shared {
            queue: Mutex::new(Queue {
                closed: false,
                jobs: VecDeque::new(),
            }),
            ready: Condvar::new(),
            capacity,
        });
        let mut pool = Self {
            shared,
            _workers: Vec::with_capacity(workers),
        };
        for index in 0..workers {
            let shared = Arc::clone(&pool.shared);
            pool._workers.push(
                thread::Builder::new()
                    .name(format!("karbon-ui-task-{index}"))
                    .spawn(move || {
                        loop {
                            let job = {
                                let mut queue = lock(&shared.queue);
                                while queue.jobs.is_empty() && !queue.closed {
                                    queue = shared
                                        .ready
                                        .wait(queue)
                                        .unwrap_or_else(|error| error.into_inner());
                                }
                                if queue.closed {
                                    return;
                                }
                                queue.jobs.pop_front().expect("nonempty task queue")
                            };
                            job.run();
                        }
                    })?,
            );
        }
        Ok(pool)
    }
    pub(crate) fn discard_cancelled(&self) {
        let discarded = {
            let mut queue = lock(&self.shared.queue);
            if !queue.jobs.iter().any(|job| job.cancel.is_cancelled()) {
                return;
            }
            let mut kept = VecDeque::with_capacity(queue.jobs.len());
            let mut discarded = Vec::new();
            while let Some(job) = queue.jobs.pop_front() {
                if job.cancel.is_cancelled() {
                    discarded.push(job);
                } else {
                    kept.push_back(job);
                }
            }
            queue.jobs = kept;
            discarded
        };
        // Les captures utilisateur sont libérées sans verrou de la file.
        drop(discarded);
    }
    pub(super) fn submit(&self, job: Job) -> Result<(), TaskError> {
        self.discard_cancelled();
        let mut queue = lock(&self.shared.queue);
        if queue.closed {
            return Err(TaskError::ExecutorClosed);
        }
        if queue.jobs.len() >= self.shared.capacity {
            return Err(TaskError::QueueFull);
        }
        queue.jobs.push_back(job);
        drop(queue);
        self.shared.ready.notify_one();
        Ok(())
    }
}
impl Drop for TaskExecutor {
    fn drop(&mut self) {
        let pending = {
            let mut queue = lock(&self.shared.queue);
            queue.closed = true;
            std::mem::take(&mut queue.jobs)
        };
        for job in pending {
            job.cancel.cancel();
        }
        self.shared.ready.notify_all();
        // Ne pas joindre une tâche non coopérative depuis le thread UI.
        // Les jobs déjà en cours sont annulés par la fermeture de leur TaskScope.
    }
}

pub(crate) struct TaskScope {
    pub(super) mailbox: Arc<Mailbox>,
    pub(super) running: HashMap<TaskId, Running>,
    pub(super) revision: u64,
}
impl TaskScope {
    pub(crate) fn active_count(&self) -> usize {
        self.running.len()
    }
    pub(super) const CAPACITY: usize = 64;
    pub(crate) fn new(wake: impl Fn() + Send + Sync + 'static) -> Self {
        Self {
            mailbox: Arc::new(Mailbox {
                inbox: Mutex::new(Inbox {
                    closed: false,
                    results: VecDeque::new(),
                }),
                wake: Arc::new(wake),
            }),
            running: HashMap::new(),
            revision: 0,
        }
    }
    pub(crate) fn cancel(&mut self, id: TaskId) {
        if let Some(running) = self.running.remove(&id) {
            running.cancel.cancel();
        }
        let removed = {
            let mut inbox = lock(&self.mailbox.inbox);
            let mut kept = VecDeque::with_capacity(inbox.results.len());
            let mut removed = Vec::new();
            while let Some(completion) = inbox.results.pop_front() {
                if completion.result.id == id {
                    removed.push(completion);
                } else {
                    kept.push_back(completion);
                }
            }
            inbox.results = kept;
            removed
        };
        drop(removed);
    }
    pub(crate) fn cancel_all(&mut self) {
        for (_, running) in self.running.drain() {
            running.cancel.cancel();
        }
        let removed = std::mem::take(&mut lock(&self.mailbox.inbox).results);
        drop(removed);
    }
    pub(crate) fn spawn(&mut self, task: Task, executor: &TaskExecutor) -> Option<TaskResult> {
        self.cancel(task.id);
        if self.running.len() >= Self::CAPACITY {
            return Some(TaskResult::failure(task.id, TaskError::QueueFull));
        }
        let Some(revision) = self.revision.checked_add(1) else {
            return Some(TaskResult::failure(task.id, TaskError::ExecutorClosed));
        };
        self.revision = revision;
        let id = task.id;
        let cancel = CancellationToken::new();
        self.running.insert(
            id,
            Running {
                revision,
                cancel: cancel.clone(),
            },
        );
        let job = Job {
            task,
            revision,
            cancel,
            mailbox: Arc::downgrade(&self.mailbox),
        };
        if let Err(error) = executor.submit(job) {
            self.running.remove(&id);
            return Some(TaskResult::failure(id, error));
        }
        None
    }
    pub(crate) fn take_next(&mut self) -> Option<TaskResult> {
        loop {
            let completion = lock(&self.mailbox.inbox).results.pop_front()?;
            let id = completion.result.id;
            if self
                .running
                .get(&id)
                .is_some_and(|job| job.revision == completion.revision)
            {
                self.running.remove(&id);
                return Some(completion.result);
            }
        }
    }
    #[cfg(test)]
    pub(super) fn take_ready(&mut self) -> Vec<TaskResult> {
        std::iter::from_fn(|| self.take_next()).collect()
    }
}
impl Drop for TaskScope {
    fn drop(&mut self) {
        self.cancel_all();
        lock(&self.mailbox.inbox).closed = true;
    }
}
