pub(super) use super::session::Session;
use super::*;
pub(super) use crate::{
    task::TaskExecutor,
    ui::{runtime::UiEvent, scene::PointF},
    window::{ExitPolicy, WindowAction, WindowCommand, WindowDescriptor, WindowId},
};
pub(super) use objc2::{MainThreadMarker, msg_send, rc::autoreleasepool, runtime::AnyObject};
pub(super) use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSEvent, NSEventMask, NSEventModifierFlags,
    NSEventType, NSWindow,
};
pub(super) use objc2_foundation::{NSDate, NSDefaultRunLoopMode, NSPoint};
pub(super) use std::{
    cell::{Cell, RefCell},
    panic::{AssertUnwindSafe, catch_unwind},
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

thread_local! {
    pub(super) static RUNNING: Cell<bool> = const { Cell::new(false) };
    pub(super) static SESSIONS: RefCell<Vec<Session>> = const { RefCell::new(Vec::new()) };
    pub(super) static PENDING: RefCell<Vec<WindowCommand>> = const { RefCell::new(Vec::new()) };
    pub(super) static CLOSED: RefCell<Vec<WindowId>> = const { RefCell::new(Vec::new()) };
    pub(super) static ERROR: RefCell<Option<crate::Error>> = const { RefCell::new(None) };
    pub(super) static EXIT: Cell<bool> = const { Cell::new(false) };
    pub(super) static EXECUTOR: RefCell<Option<TaskExecutor>> = const { RefCell::new(None) };
}
/// Un réveil est déjà en file : les suivants sont fusionnés.
pub(super) static WAKE_PENDING: AtomicBool = AtomicBool::new(false);
/// Sous-type des événements « application » de réveil.
pub(super) const WAKE_SUBTYPE: i16 = 0x4b55;

/// Réveille la boucle depuis n'importe quel thread (résultat de tâche disponible).
pub(crate) fn wake() {
    if WAKE_PENDING.swap(true, Ordering::AcqRel) {
        return;
    }
    autoreleasepool(|_| {
        // SAFETY : `postEvent:atStart:` est documenté utilisable depuis un autre thread ;
        // l'événement ne transporte aucun pointeur.
        unsafe {
            let app: *mut AnyObject = msg_send![objc2::class!(NSApplication), sharedApplication];
            let event: *mut AnyObject = msg_send![
                objc2::class!(NSEvent),
                otherEventWithType: NSEventType::ApplicationDefined,
                location: NSPoint::new(0., 0.),
                modifierFlags: NSEventModifierFlags(0),
                timestamp: 0f64,
                windowNumber: 0isize,
                context: std::ptr::null_mut::<AnyObject>(),
                subtype: WAKE_SUBTYPE,
                data1: 0isize,
                data2: 0isize
            ];
            if !app.is_null() && !event.is_null() {
                let _: () = msg_send![app, postEvent: event, atStart: false];
            }
        }
    });
}
pub(super) fn record_error(error: crate::Error) {
    ERROR.with(|slot| {
        slot.borrow_mut().get_or_insert(error);
    });
}
/// Exécute `f` sur la session `id` ; une erreur ou une panique de l'application est
/// conservée et ferme la fenêtre.
pub(super) fn with<R>(id: WindowId, f: impl FnOnce(&mut Session) -> crate::Result<R>) -> Option<R> {
    let outcome = SESSIONS.with(|sessions| {
        let mut sessions = sessions.try_borrow_mut().ok()?;
        let session = sessions.iter_mut().find(|s| s.id == id)?;
        Some(catch_unwind(AssertUnwindSafe(|| f(session))))
    })?;
    let error = match outcome {
        Ok(Ok(value)) => return Some(value),
        Ok(Err(error)) => error,
        Err(payload) => crate::Error::Panicked {
            message: payload
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "non-string panic payload".into()),
        },
    };
    record_error(error);
    PENDING.with(|p| p.borrow_mut().push(WindowCommand::Close(id)));
    None
}
pub(super) fn window_of(id: WindowId) -> Option<objc2::rc::Retained<NSWindow>> {
    SESSIONS.with(|s| {
        s.try_borrow()
            .ok()?
            .iter()
            .find(|s| s.id == id)
            .map(|s| s.window.clone())
    })
}
/// Commande de fenêtre de la barre de menus, exécutée par la boucle.
pub(super) fn command(command: WindowCommand) {
    PENDING.with(|p| p.borrow_mut().push(command));
}
pub(super) fn closed(id: WindowId) {
    // Fenêtre fermée pendant le geste d'une poignée : il se termine avec elle.
    change::grip_end(id);
    CLOSED.with(|c| c.borrow_mut().push(id));
}
/// Taille ou échelle changée : peinte tout de suite (la boucle ne tourne pas pendant un
/// redimensionnement à la souris).
pub(super) fn resized(id: WindowId) {
    with(id, |s| {
        s.dirty = true;
        s.paint()
    });
}
/// Redimensionnement à la souris (début et fin du geste) : pendant le geste, les ombres
/// floutées des éléments qui ne font que changer de taille sont étirées au lieu d'être
/// recalculées à chaque pas ; la fin du geste les repeint exactement.
pub(super) fn live_resize(id: WindowId, live: bool) {
    with(id, |s| {
        s.runtime.set_live_resize(live);
        if live {
            return Ok(());
        }
        s.dirty = true;
        s.paint()
    });
}
pub(super) fn focus(id: WindowId, focused: bool) {
    with(id, |s| {
        if focused {
            s.refresh_cursor();
        }
        s.input(UiEvent::FocusChanged(focused)).map(drop)
    });
}
/// Fenêtre réduite ou cachée : ses ressources de rendu sont libérées.
pub(super) fn hidden(id: WindowId) {
    with(id, |s| {
        s.hidden();
        Ok(())
    });
}
/// Visibilité changée : entièrement cachée (autres fenêtres, autre bureau), la fenêtre
/// libère ses ressources de rendu ; de nouveau visible, elle refait son image.
pub(super) fn occlusion(id: WindowId) {
    let visible = window_of(id).is_none_or(|w| {
        w.occlusionState()
            .contains(objc2_app_kit::NSWindowOcclusionState::Visible)
    });
    if visible {
        resized(id);
    } else {
        hidden(id);
    }
}
/// Pression mémoire du système (`memory.rs`) : chaque fenêtre allège ses ressources.
fn relieve_memory(critical: bool) {
    for id in ids() {
        with(id, |s| {
            s.relieve_memory(critical);
            Ok(())
        });
    }
}
/// Battement de l'écran d'une fenêtre (`frame_link.rs`) : la boucle repart pour l'image.
pub(super) fn display_tick(id: WindowId) {
    with(id, |s| {
        s.frame_tick(Instant::now());
        Ok(())
    });
    wake();
}
pub(super) fn paint_now(id: WindowId) {
    with(id, |s| s.paint());
}
pub(super) fn ids() -> Vec<WindowId> {
    SESSIONS.with(|s| s.borrow().iter().map(|s| s.id).collect())
}
pub(super) fn open(descriptor: WindowDescriptor, mtm: MainThreadMarker) -> crate::Result<()> {
    if ids().contains(&descriptor.id) {
        return Err(crate::Error::DuplicateWindowId(descriptor.id));
    }
    let session = Session::open(descriptor, mtm)?;
    let id = session.id;
    SESSIONS.with(|s| s.borrow_mut().push(session));
    with(id, |s| s.paint());
    // Hors de l'emprunt des sessions : la fenêtre devenue clé le signale aussitôt.
    let activate = with(id, |s| Ok(s.options.activate)).unwrap_or(false);
    if let Some(window) = window_of(id) {
        if activate {
            window.makeKeyAndOrderFront(None);
        } else {
            window.orderFront(None);
        }
    }
    Ok(())
}
pub(super) fn execute(command: WindowCommand, mtm: MainThreadMarker) {
    match command {
        WindowCommand::Open(descriptor) => {
            if let Err(error) = open(descriptor, mtm) {
                record_error(error);
            }
        }
        WindowCommand::Close(id) | WindowCommand::Action(id, WindowAction::Close) => {
            if let Some(window) = window_of(id) {
                window.close();
            }
        }
        WindowCommand::Action(id, WindowAction::Minimize) => {
            if let Some(window) = window_of(id) {
                window.miniaturize(None);
            }
        }
        WindowCommand::Action(id, WindowAction::ToggleMaximize) => {
            if let Some(window) = window_of(id) {
                window.zoom(None);
            }
        }
        WindowCommand::Change(id, change) => {
            if let Some(reason) = crate::window::change::invalid_change(&change) {
                record_error(crate::Error::invalid_window_options(reason));
            } else if let Some(window) = window_of(id) {
                change::apply(&window, change, mtm);
                with(id, |s| {
                    s.changed(change, mtm);
                    Ok(())
                });
            }
        }
        WindowCommand::FileDialog(id, task, file_dialog) => {
            if let Some(window) = window_of(id) {
                dialog::show(id, &window, task, &file_dialog, mtm);
            }
        }
        WindowCommand::Exit => EXIT.with(|e| e.set(true)),
    }
}
/// Travail entre deux événements ; `false` quand la boucle doit s'arrêter.
pub(super) fn service(policy: ExitPolicy, mtm: MainThreadMarker) -> bool {
    for id in CLOSED.with(|c| std::mem::take(&mut *c.borrow_mut())) {
        let session = SESSIONS.with(|s| {
            let mut sessions = s.borrow_mut();
            let index = sessions.iter().position(|s| s.id == id)?;
            Some(sessions.remove(index))
        });
        // L'application est détruite hors de tout emprunt.
        drop(session);
        if policy == ExitPolicy::MainWindowClosed(id) {
            EXIT.with(|e| e.set(true));
        }
    }
    // Choix des dialogues de fichiers, livrés comme des résultats de tâches.
    for (id, result) in dialog::take_finished() {
        with(id, |s| {
            s.dirty |= s.runtime.task_completed(result);
            Ok(())
        });
    }
    let now = Instant::now();
    for id in ids() {
        let commands = with(id, |s| {
            EXECUTOR.with(|e| s.tasks(&mut e.borrow_mut()));
            s.tick(now)?;
            s.report(mtm);
            Ok(s.take_window_commands())
        });
        PENDING.with(|p| p.borrow_mut().extend(commands.unwrap_or_default()));
    }
    loop {
        let commands = PENDING.with(|p| std::mem::take(&mut *p.borrow_mut()));
        if commands.is_empty() {
            break;
        }
        for command in commands {
            execute(command, mtm);
        }
    }
    let now = Instant::now();
    for id in ids() {
        with(id, |s| {
            if s.dirty && s.frame_ready(now) {
                s.paint()
            } else {
                Ok(())
            }
        });
    }
    let failed = ERROR.with(|e| e.borrow().is_some());
    let empty = SESSIONS.with(|s| s.borrow().is_empty());
    let pending = CLOSED.with(|c| !c.borrow().is_empty());
    !(EXIT.with(Cell::get) || failed || (empty && !pending))
}
/// Prochaine échéance d'attente : réveil le plus proche, ou tout de suite si une fenêtre
/// attend d'être peinte.
pub(super) fn deadline() -> Option<f64> {
    SESSIONS.with(|s| {
        let sessions = s.borrow();
        let now = Instant::now();
        if sessions.iter().any(|s| s.dirty && s.frame_ready(now)) {
            return Some(0.);
        }
        sessions
            .iter()
            .filter_map(|s| s.next_wake())
            .min()
            .map(|wake| wake.saturating_duration_since(now).as_secs_f64())
    })
}
pub(super) struct RunGuard;
impl Drop for RunGuard {
    fn drop(&mut self) {
        let sessions = SESSIONS.with(|s| std::mem::take(&mut *s.borrow_mut()));
        for session in &sessions {
            session.window.close();
        }
        drop(sessions);
        PENDING.with(|p| p.borrow_mut().clear());
        CLOSED.with(|c| c.borrow_mut().clear());
        drop(dialog::take_finished());
        EXIT.with(|e| e.set(false));
        EXECUTOR.with(|e| e.borrow_mut().take());
        change::grip_reset();
        RUNNING.with(|r| r.set(false));
    }
}
/// Boucle d'événements AppKit ; erreurs documentées par `platform::run_windows`.
pub(crate) fn run_windows(windows: Vec<WindowDescriptor>, policy: ExitPolicy) -> crate::Result<()> {
    crate::window::validation::validate(&windows, policy)?;
    let mtm = MainThreadMarker::new().ok_or(crate::Error::Unsupported {
        reason: "the macOS event loop must run on the main thread",
    })?;
    if RUNNING.with(|r| r.replace(true)) {
        return Err(crate::Error::AlreadyRunning);
    }
    let guard = RunGuard;
    let app = NSApplication::sharedApplication(mtm);
    let delegate = super::app_delegate::AppDelegate::new(mtm);
    app.setDelegate(Some(objc2::runtime::ProtocolObject::from_ref(&*delegate)));
    let result = run_loop(&app, windows, policy, mtm);
    // Sessions fermées et applications détruites avant qu'AppKit ne termine le processus
    // (fermeture de session, voir `app_delegate.rs`).
    drop(guard);
    app.setDelegate(None);
    delegate.finish(&app);
    result
}
/// Ouvre les fenêtres et fait tourner la boucle jusqu'à la sortie.
fn run_loop(
    app: &NSApplication,
    windows: Vec<WindowDescriptor>,
    policy: ExitPolicy,
    mtm: MainThreadMarker,
) -> crate::Result<()> {
    // Détruite avant le garde : plus de rappel une fois les sessions fermées.
    let _pressure = super::memory::MemoryPressure::watch(relieve_memory);
    ERROR.with(|e| e.borrow_mut().take());
    app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
    menu::install(app, mtm);
    app.finishLaunching();
    // Des fenêtres toutes ouvertes sans activation ne prennent pas le premier plan.
    let activate = windows.iter().any(|window| window.options.activate);
    for descriptor in windows {
        open(descriptor, mtm)?;
    }
    if activate {
        #[allow(deprecated)]
        app.activateIgnoringOtherApps(true);
    }
    // SAFETY : constante de Foundation.
    let mode = unsafe { NSDefaultRunLoopMode };
    while autoreleasepool(|_| {
        // Un réveil consommé par une boucle AppKit imbriquée (menu, dialogue) laisserait
        // l'indicateur levé et ferait ignorer tous les suivants (M9 de la revue) : chaque
        // tour repart de zéro, avant de livrer les tâches.
        WAKE_PENDING.store(false, Ordering::Release);
        if !service(policy, mtm) {
            return false;
        }
        let until = match deadline() {
            Some(seconds) => NSDate::dateWithTimeIntervalSinceNow(seconds),
            None => NSDate::distantFuture(),
        };
        if let Some(event) = app.nextEventMatchingMask_untilDate_inMode_dequeue(
            NSEventMask::Any,
            Some(&until),
            mode,
            true,
        ) {
            if event.r#type() == NSEventType::ApplicationDefined
                && event.subtype().0 == WAKE_SUBTYPE
            {
                WAKE_PENDING.store(false, Ordering::Release);
            } else {
                app.sendEvent(&event);
            }
        }
        true
    }) {}
    match ERROR.with(|e| e.borrow_mut().take()) {
        Some(error) => Err(error),
        None => Ok(()),
    }
}
