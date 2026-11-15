//! Boîte aux lettres : dépôts avant et après l'abonnement, réveil unique, dépôt depuis un
//! autre thread, abonnement perdu puis repris, fin de l'application.
use super::{INBOX_TASK, Inbox};
use crate::task::{Task, TaskExecutor, TaskScope};
use std::{sync::mpsc, time::Duration};

#[derive(Default)]
struct App {
    log: Vec<u32>,
}

/// Portée de tâches dont les réveils arrivent sur un canal, et son exécuteur.
fn window() -> (TaskScope, TaskExecutor, mpsc::Receiver<()>) {
    let (wake, woken) = mpsc::channel();
    let scope = TaskScope::new(move || {
        let _ = wake.send(());
    });
    (scope, TaskExecutor::new(1, 4).unwrap(), woken)
}

fn listen(inbox: &Inbox<App>, scope: &mut TaskScope, executor: &TaskExecutor) {
    let port = inbox.port();
    assert!(port.wants_listener(false));
    let task = Task::listen(INBOX_TASK, port.listener());
    assert!(scope.spawn(task, executor).is_none());
}

#[test]
fn posts_are_applied_in_order_after_one_wake_up() {
    let inbox = Inbox::new();
    let sender = inbox.sender();
    // Avant l'abonnement : gardés, sans réveil.
    sender.post(|app: &mut App| {
        app.log.push(1);
        false
    });
    let (mut scope, executor, woken) = window();
    listen(&inbox, &mut scope, &executor);
    assert!(!inbox.port().wants_listener(false), "already listening");
    for n in 2..5 {
        sender.post(move |app| {
            app.log.push(n);
            n == 3
        });
    }
    // Un seul réveil pour les quatre dépôts.
    woken.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(woken.try_recv().is_err());
    let result = scope.take_next().unwrap();
    assert_eq!(result.id, INBOX_TASK);
    assert!(scope.take_next().is_none());
    let mut app = App::default();
    assert!(inbox.port().deliver_any(&mut app));
    assert_eq!(app.log, [1, 2, 3, 4]);
    assert_eq!(
        scope.active_count(),
        0,
        "a listener is not a task in flight"
    );
    // Livré : le dépôt suivant réveille de nouveau.
    sender.post(|app| {
        app.log.push(5);
        false
    });
    woken.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(scope.take_next().map(|r| r.id), Some(INBOX_TASK));
    assert!(!inbox.deliver(&mut app));
    assert_eq!(app.log, [1, 2, 3, 4, 5]);
}

#[test]
fn another_thread_wakes_the_window() {
    let inbox = Inbox::new();
    let (mut scope, executor, woken) = window();
    listen(&inbox, &mut scope, &executor);
    let sender = inbox.sender();
    std::thread::spawn(move || {
        sender.post(|app: &mut App| {
            app.log.push(7);
            true
        })
    })
    .join()
    .unwrap();
    woken.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(scope.take_next().is_some());
    let mut app = App::default();
    assert!(inbox.deliver(&mut app));
    assert_eq!(app.log, [7]);
}

#[test]
fn a_lost_subscription_is_taken_again() {
    let inbox = Inbox::new();
    let (mut scope, executor, woken) = window();
    listen(&inbox, &mut scope, &executor);
    scope.cancel_all();
    let sender = inbox.sender();
    sender.post(|app: &mut App| {
        app.log.push(1);
        true
    });
    assert!(woken.try_recv().is_err(), "nobody listens");
    // Nouvelle fenêtre (ou portée remplacée) : l'abonnement reprend et réveille aussitôt.
    let (mut scope, executor, woken) = window();
    listen(&inbox, &mut scope, &executor);
    woken.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(scope.take_next().is_some());
    let mut app = App::default();
    assert!(inbox.deliver(&mut app));
    assert_eq!(app.log, [1]);
    // Un abonnement demandé n'est pas redemandé tant que la fenêtre ne l'a pas remis.
    drop(scope);
    assert!(inbox.port().wants_listener(false));
    assert!(!inbox.port().wants_listener(false));
    assert!(
        inbox.port().wants_listener(true),
        "forced after a CancelAll"
    );
}

#[test]
fn senders_stop_when_the_application_ends() {
    let inbox: Inbox<App> = Inbox::new();
    let copy = inbox.clone();
    let sender = inbox.sender();
    drop(inbox);
    assert!(sender.post(|_| false));
    drop(copy);
    assert!(sender.is_closed());
    assert!(!sender.post(|_| false));
}

#[test]
fn wait_returns_when_something_is_posted() {
    let inbox: Inbox<App> = Inbox::new();
    assert!(!inbox.wait(Duration::from_millis(10)));
    let sender = inbox.sender();
    let thread = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(20));
        sender.post(|_| false)
    });
    assert!(inbox.wait(Duration::from_secs(5)));
    assert_eq!(inbox.len(), 1);
    assert!(thread.join().unwrap());
}
