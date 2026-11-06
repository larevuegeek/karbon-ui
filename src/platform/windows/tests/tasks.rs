//! Tâches de fond : réveil de la fenêtre, résultat livré sur le thread UI, résultat
//! obsolète d'une fenêtre fermée puis rouverte sous le même identifiant.
use super::*;
use crate::window::ExitPolicy;
use windows_sys::Win32::UI::WindowsAndMessaging::DestroyWindow;

#[test]
fn background_result_wakes_the_window_and_runs_the_callback_on_the_ui_thread() {
    use crate::task::{Task, TaskCommand, TaskId};
    use std::{sync::mpsc, time::Duration};
    let (_apartment, session) = session();
    let mut application = app();
    application.exit_on_completion = true;
    let completed = Rc::clone(&application.completed);
    let expected_thread = std::thread::current().id();
    let (started, running) = mpsc::channel();
    let (release, wait) = mpsc::channel();
    application
        .task_commands
        .borrow_mut()
        .push(TaskCommand::Spawn(Task::new(TaskId(7), move |_| {
            started.send(std::thread::current().id()).unwrap();
            wait.recv_timeout(Duration::from_secs(2)).unwrap();
            42_u32
        })));
    session.open(descriptor(1, application), false).unwrap();
    // SAFETY : simple lecture de l'identifiant du thread courant.
    let ui_thread = unsafe { GetCurrentThreadId() };
    let (finished, done) = mpsc::channel();
    let driver = std::thread::spawn(move || {
        let worker = running.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_ne!(worker, expected_thread);
        release.send(()).unwrap();
        if done.recv_timeout(Duration::from_secs(2)).is_err() {
            // SAFETY : message sans pointeur vers la file du thread UI.
            unsafe {
                PostThreadMessageW(ui_thread, WM_QUIT, 0, 0);
            }
            return false;
        }
        true
    });
    session
        .run_message_loop(ExitPolicy::LastWindowClosed)
        .unwrap();
    finished.send(()).unwrap();
    assert!(
        driver.join().unwrap(),
        "task completion did not wake the UI"
    );
    assert_eq!(&*completed.borrow(), &[(TaskId(7), 42, expected_thread)]);
}

#[test]
fn closing_and_reusing_window_ids_rejects_an_old_background_result() {
    use crate::task::{Task, TaskCommand, TaskId};
    use std::{sync::mpsc, time::Duration};
    let (_apartment, session) = session();
    let old = app();
    let obsolete = Rc::clone(&old.completed);
    let (started, running) = mpsc::channel();
    let (release, wait) = mpsc::channel();
    old.task_commands
        .borrow_mut()
        .push(TaskCommand::Spawn(Task::new(TaskId(1), move |cancel| {
            started.send(cancel).unwrap();
            wait.recv_timeout(Duration::from_secs(2)).unwrap();
            1_u32
        })));
    let window = session.open(descriptor(1, old), false).unwrap();
    session.tasks.process();
    let cancel = running.recv_timeout(Duration::from_secs(2)).unwrap();
    // SAFETY : fenêtre de test créée sur ce thread ; Windows valide le handle.
    unsafe {
        DestroyWindow(window);
    }
    assert!(cancel.is_cancelled());
    let mut replacement = app();
    replacement.exit_on_completion = true;
    let completed = Rc::clone(&replacement.completed);
    replacement
        .task_commands
        .borrow_mut()
        .push(TaskCommand::Spawn(Task::new(TaskId(1), |_| 99_u32)));
    session.open(descriptor(1, replacement), false).unwrap();
    release.send(()).unwrap();
    // SAFETY : simple lecture de l'identifiant du thread courant.
    let ui_thread = unsafe { GetCurrentThreadId() };
    let (finished, done) = mpsc::channel();
    let watchdog = std::thread::spawn(move || {
        if done.recv_timeout(Duration::from_secs(2)).is_err() {
            // SAFETY : message sans pointeur vers la file du thread UI.
            unsafe {
                PostThreadMessageW(ui_thread, WM_QUIT, 0, 0);
            }
            return false;
        }
        true
    });
    session
        .run_message_loop(ExitPolicy::LastWindowClosed)
        .unwrap();
    finished.send(()).unwrap();
    assert!(watchdog.join().unwrap());
    assert!(obsolete.borrow().is_empty());
    assert_eq!(completed.borrow().len(), 1);
    assert_eq!(completed.borrow()[0].1, 99);
}
