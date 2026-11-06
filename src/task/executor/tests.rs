use super::*;
use std::{sync::mpsc, time::Duration};
fn scope() -> (TaskScope, mpsc::Receiver<()>) {
    let (send, receive) = mpsc::channel();
    (
        TaskScope::new(move || {
            let _ = send.send(());
        }),
        receive,
    )
}
fn ready(scope: &mut TaskScope, wake: &mpsc::Receiver<()>) -> Vec<TaskResult> {
    loop {
        wake.recv_timeout(Duration::from_secs(2))
            .expect("worker did not complete");
        let values = scope.take_ready();
        if !values.is_empty() {
            return values;
        }
    }
}
#[test]
fn results_cross_threads_and_preserve_values_and_business_errors() {
    let executor = TaskExecutor::new(1, 4).unwrap();
    let (mut scope, wake) = scope();
    let ui_thread = thread::current().id();
    scope.spawn(
        Task::new(TaskId(1), move |_| {
            assert_ne!(thread::current().id(), ui_thread);
            Ok::<_, String>(vec![1_u32, 2, 3])
        }),
        &executor,
    );
    let result = ready(&mut scope, &wake).pop().unwrap();
    assert_eq!(result.id, TaskId(1));
    assert_eq!(
        result
            .into_value::<Result<Vec<u32>, String>>()
            .unwrap()
            .unwrap(),
        vec![1, 2, 3]
    );
    assert!(scope.running.is_empty());
    assert!(scope.take_ready().is_empty());
}
#[test]
fn replacement_drops_late_results_even_when_work_ignores_cancellation() {
    let executor = TaskExecutor::new(1, 4).unwrap();
    let (mut scope, wake) = scope();
    let (started, running) = mpsc::channel();
    let (release, wait) = mpsc::channel();
    scope.spawn(
        Task::new(TaskId(1), move |cancel| {
            started.send(cancel).unwrap();
            wait.recv_timeout(Duration::from_secs(2)).unwrap();
            1_u32
        }),
        &executor,
    );
    let cancellation = running.recv_timeout(Duration::from_secs(2)).unwrap();
    scope.spawn(Task::new(TaskId(1), |_| 2_u32), &executor);
    assert!(cancellation.is_cancelled());
    release.send(()).unwrap();
    let result = ready(&mut scope, &wake);
    assert_eq!(result.len(), 1);
    assert_eq!(
        result
            .into_iter()
            .next()
            .unwrap()
            .into_value::<u32>()
            .unwrap(),
        2
    );
}
#[test]
fn cancellation_removes_a_completed_result_before_it_is_delivered() {
    let executor = TaskExecutor::new(1, 4).unwrap();
    let (mut scope, wake) = scope();
    scope.spawn(Task::new(TaskId(1), |_| 9_u32), &executor);
    wake.recv_timeout(Duration::from_secs(2)).unwrap();
    scope.cancel(TaskId(1));
    assert!(scope.take_ready().is_empty());
    scope.spawn(Task::new(TaskId(1), |_| 10_u32), &executor);
    assert_eq!(
        ready(&mut scope, &wake)
            .pop()
            .unwrap()
            .into_value::<u32>()
            .unwrap(),
        10
    );
}
#[test]
fn cancelled_queued_captures_are_freed_without_running_them() {
    let executor = TaskExecutor::new(1, 2).unwrap();
    let (mut scope, wake) = scope();
    let (started, running) = mpsc::channel();
    let (release, wait) = mpsc::channel();
    scope.spawn(
        Task::new(TaskId(1), move |_| {
            started.send(()).unwrap();
            wait.recv_timeout(Duration::from_secs(2)).unwrap();
        }),
        &executor,
    );
    running.recv_timeout(Duration::from_secs(2)).unwrap();
    let captured = Arc::new(42_u32);
    let weak = Arc::downgrade(&captured);
    scope.spawn(
        Task::new(TaskId(2), move |_| {
            drop(captured);
            panic!("cancelled task ran")
        }),
        &executor,
    );
    scope.cancel(TaskId(2));
    executor.discard_cancelled();
    assert!(weak.upgrade().is_none());
    release.send(()).unwrap();
    assert_eq!(ready(&mut scope, &wake).len(), 1);
}
#[test]
fn bounded_queue_rejects_excess_work_and_reclaims_cancelled_slots() {
    let executor = TaskExecutor::new(1, 1).unwrap();
    let (mut scope, wake) = scope();
    let (started, running) = mpsc::channel();
    let (release, wait) = mpsc::channel();
    scope.spawn(
        Task::new(TaskId(1), move |_| {
            started.send(()).unwrap();
            wait.recv_timeout(Duration::from_secs(2)).unwrap();
        }),
        &executor,
    );
    running.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(
        scope
            .spawn(Task::new(TaskId(2), |_| 2_u32), &executor)
            .is_none()
    );
    let error = scope
        .spawn(Task::new(TaskId(3), |_| 3_u32), &executor)
        .unwrap();
    assert_eq!(error.into_value::<u32>(), Err(TaskError::QueueFull));
    scope.cancel(TaskId(2));
    assert!(
        scope
            .spawn(Task::new(TaskId(4), |_| 4_u32), &executor)
            .is_none()
    );
    release.send(()).unwrap();
    let mut ids = Vec::new();
    while ids.len() < 2 {
        ids.extend(ready(&mut scope, &wake).into_iter().map(|result| result.id));
    }
    assert!(ids.contains(&TaskId(1)) && ids.contains(&TaskId(4)));
}
#[test]
fn closing_a_scope_cancels_running_work_and_prevents_wakes_to_a_new_scope() {
    let executor = TaskExecutor::new(1, 2).unwrap();
    let (mut old, wake) = scope();
    let (started, running) = mpsc::channel();
    let (release, wait) = mpsc::channel();
    old.spawn(
        Task::new(TaskId(1), move |cancel| {
            started.send(cancel).unwrap();
            wait.recv_timeout(Duration::from_secs(2)).unwrap();
            7_u32
        }),
        &executor,
    );
    let cancel = running.recv_timeout(Duration::from_secs(2)).unwrap();
    let old_mailbox = Arc::downgrade(&old.mailbox);
    drop(old);
    assert!(cancel.is_cancelled());
    assert!(old_mailbox.upgrade().is_none());
    let (mut new, new_wake) = scope();
    new.spawn(Task::new(TaskId(1), |_| 8_u32), &executor);
    release.send(()).unwrap();
    assert_eq!(
        ready(&mut new, &new_wake)
            .pop()
            .unwrap()
            .into_value::<u32>()
            .unwrap(),
        8
    );
    assert!(wake.try_recv().is_err());
}
#[test]
fn worker_panics_are_reported_and_the_pool_remains_usable() {
    let executor = TaskExecutor::new(1, 4).unwrap();
    let (mut scope, wake) = scope();
    scope.spawn(
        Task::new(TaskId(1), |_| -> u32 { panic!("expected worker panic") }),
        &executor,
    );
    assert_eq!(
        ready(&mut scope, &wake).pop().unwrap().into_value::<u32>(),
        Err(TaskError::WorkerPanicked)
    );
    scope.spawn(Task::new(TaskId(2), |_| "still working"), &executor);
    assert_eq!(
        ready(&mut scope, &wake).pop().unwrap().into_value::<u32>(),
        Err(TaskError::TypeMismatch)
    );
    scope.spawn(Task::new(TaskId(3), |_| 3_u32), &executor);
    assert_eq!(
        ready(&mut scope, &wake)
            .pop()
            .unwrap()
            .into_value::<u32>()
            .unwrap(),
        3
    );
}
#[test]
fn cancel_all_discards_each_window_task_and_invalid_pool_sizes_are_rejected() {
    assert!(TaskExecutor::new(0, 1).is_err());
    assert!(TaskExecutor::new(1, 0).is_err());
    let executor = TaskExecutor::new(1, 4).unwrap();
    let (mut scope, _) = scope();
    scope.spawn(Task::new(TaskId(1), |_| 1_u32), &executor);
    scope.spawn(Task::new(TaskId(2), |_| 2_u32), &executor);
    scope.cancel_all();
    assert!(scope.running.is_empty());
    assert!(scope.take_ready().is_empty());
}

#[test]
fn each_scope_limits_its_active_tasks_even_if_the_executor_has_more_room() {
    let executor = TaskExecutor::new(1, 128).unwrap();
    let (mut scope, _) = scope();
    let (started, running) = mpsc::channel();
    let (release, wait) = mpsc::channel();
    scope.spawn(
        Task::new(TaskId(0), move |_| {
            started.send(()).unwrap();
            wait.recv_timeout(Duration::from_secs(2)).unwrap();
        }),
        &executor,
    );
    running.recv_timeout(Duration::from_secs(2)).unwrap();
    for id in 1..64 {
        assert!(
            scope
                .spawn(Task::new(TaskId(id), |_| 0_u32), &executor)
                .is_none()
        );
    }
    assert_eq!(
        scope
            .spawn(Task::new(TaskId(64), |_| 0_u32), &executor)
            .unwrap()
            .into_value::<u32>(),
        Err(TaskError::QueueFull)
    );
    scope.cancel_all();
    executor.discard_cancelled();
    release.send(()).unwrap();
}
