//! Boucle d'événements AppKit et fenêtres natives.
//!
//! La boucle est écrite à la main (`nextEventMatchingMask:untilDate:…`) : l'attente se
//! termine au prochain réveil demandé par le runtime (animations, infobulles), ce qui
//! tient lieu de minuterie, ou à un événement « application » posté par les tâches de
//! fond (réveil thread-safe). Entre deux événements, la boucle livre les tâches, exécute
//! les commandes de fenêtre et peint les fenêtres invalidées.
//!
//! Les méthodes Objective-C (`view.rs`) retrouvent leur session par identité ; aucun
//! emprunt des sessions n'est tenu pendant un appel AppKit qui peut ré-entrer (fermeture,
//! zoom, déplacement de la fenêtre).
pub(super) mod accessible;
mod change;
mod dialog;
mod events;
mod menu;
mod session;
mod text;
mod view;

use crate::{
    task::TaskExecutor,
    ui::{runtime::UiEvent, scene::PointF},
    window::{ExitPolicy, WindowAction, WindowCommand, WindowDescriptor, WindowId},
};
use objc2::{MainThreadMarker, msg_send, rc::autoreleasepool, runtime::AnyObject};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSEvent, NSEventMask, NSEventModifierFlags,
    NSEventType, NSWindow,
};
use objc2_foundation::{NSDate, NSDefaultRunLoopMode, NSPoint};
use session::Session;
use std::{
    cell::{Cell, RefCell},
    panic::{AssertUnwindSafe, catch_unwind},
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

/// Bouton ou geste du pointeur reçu par la vue.
#[derive(Clone, Copy)]
enum Pointer {
    Move,
    Down,
    Up,
    Context,
    Leave,
    Scroll,
}
thread_local! {
    static RUNNING: Cell<bool> = const { Cell::new(false) };
    static SESSIONS: RefCell<Vec<Session>> = const { RefCell::new(Vec::new()) };
    static PENDING: RefCell<Vec<WindowCommand>> = const { RefCell::new(Vec::new()) };
    static CLOSED: RefCell<Vec<WindowId>> = const { RefCell::new(Vec::new()) };
    static ERROR: RefCell<Option<crate::Error>> = const { RefCell::new(None) };
    static EXIT: Cell<bool> = const { Cell::new(false) };
    static EXECUTOR: RefCell<Option<TaskExecutor>> = const { RefCell::new(None) };
}
/// Un réveil est déjà en file : les suivants sont fusionnés.
static WAKE_PENDING: AtomicBool = AtomicBool::new(false);
/// Sous-type des événements « application » de réveil.
const WAKE_SUBTYPE: i16 = 0x4b55;

/// Réveille la boucle depuis n'importe quel thread (résultat de tâche disponible).
pub(super) fn wake() {
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
fn record_error(error: crate::Error) {
    ERROR.with(|slot| {
        slot.borrow_mut().get_or_insert(error);
    });
}
/// Exécute `f` sur la session `id` ; une erreur ou une panique de l'application est
/// conservée et ferme la fenêtre.
fn with<R>(id: WindowId, f: impl FnOnce(&mut Session) -> crate::Result<R>) -> Option<R> {
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
fn window_of(id: WindowId) -> Option<objc2::rc::Retained<NSWindow>> {
    SESSIONS.with(|s| {
        s.try_borrow()
            .ok()?
            .iter()
            .find(|s| s.id == id)
            .map(|s| s.window.clone())
    })
}
fn pointer(id: WindowId, kind: Pointer, point: PointF, event: &NSEvent) {
    // Geste sur une poignée de redimensionnement : l'UI ne reçoit ni mouvement ni
    // relâchement.
    match kind {
        Pointer::Move => {
            if let Some(window) = window_of(id)
                && change::grip_drag(id, &window)
            {
                return;
            }
        }
        Pointer::Up if change::grip_end(id) => {
            with(id, |s| {
                s.report(MainThreadMarker::from(&*s.window));
                Ok(())
            });
            return;
        }
        _ => {}
    }
    let m = events::modifiers(event);
    let ui = match kind {
        Pointer::Move => UiEvent::PointerMove(point),
        Pointer::Up => UiEvent::PointerUp(point),
        Pointer::Leave => UiEvent::PointerLeave,
        Pointer::Context => UiEvent::ContextMenu(Some(point)),
        Pointer::Scroll => {
            let (delta_x, delta_y, notched) = events::scroll(event);
            if notched {
                UiEvent::Wheel {
                    position: point,
                    delta_x,
                    delta_y,
                }
            } else {
                UiEvent::Scroll {
                    position: point,
                    delta_x,
                    delta_y,
                }
            }
        }
        Pointer::Down => {
            // Poignée de redimensionnement de l'application : le geste est suivi à la main.
            let grip = with(id, |s| {
                s.prepare()?;
                Ok(s.resize_region_at(point))
            });
            if let (Some(Some(edge)), Some(window)) = (grip, window_of(id)) {
                change::grip_start(id, &window, edge);
                return;
            }
            // Zone de déplacement de l'application : la fenêtre se déplace (double clic :
            // zoom) et l'UI ne reçoit pas le clic, comme HTCAPTION sous Windows.
            let drag = with(id, |s| Ok(s.drag_region_at(point)));
            if drag == Some(true) {
                if let Some(window) = window_of(id) {
                    let zoomable = window
                        .styleMask()
                        .contains(objc2_app_kit::NSWindowStyleMask::Resizable);
                    if event.clickCount() == 2 && zoomable {
                        window.zoom(None);
                    } else {
                        window.performWindowDragWithEvent(event);
                    }
                }
                return;
            }
            // Position à jour avant l'appui (un menu a pu apparaître sous un pointeur
            // immobile), comme `HeadlessApp::click_at`.
            with(id, |s| s.input(UiEvent::PointerMove(point)));
            if m.shift {
                UiEvent::PointerDownExtended(point)
            } else {
                UiEvent::PointerDown(point)
            }
        }
    };
    with(id, |s| s.input(ui));
}
/// Touche enfoncée ou relâchée ; `true` si l'événement doit passer par
/// `interpretKeyEvents:` (texte, touche morte, IME), voir `text.rs`.
fn key(id: WindowId, event: &NSEvent, down: bool) -> bool {
    use crate::ui::runtime::Key;
    let m = events::modifiers(event);
    let code = event.keyCode();
    if !down {
        if let Some(key) = events::key(code) {
            with(id, |s| s.input(UiEvent::KeyUp(key)));
        }
        return false;
    }
    // Composition en cours : toutes les touches (flèches, Entrée…) vont à l'IME.
    if with(id, |s| Ok(s.marked().is_some())) == Some(true) {
        return true;
    }
    if let Some(edit) = events::edit_command(event, m) {
        with(id, |s| s.input(UiEvent::Edit(edit)));
        return false;
    }
    let key = match events::key(code) {
        // ⌘← / ⌘→ : début et fin de ligne.
        Some(Key::Left) if m.command => Some(Key::Home),
        Some(Key::Right) if m.command => Some(Key::End),
        other => other,
    };
    if let Some(key) = key {
        with(id, |s| {
            s.input(UiEvent::KeyDown {
                key,
                shift: m.shift,
                repeat: event.isARepeat(),
            })
        });
        // L'espace insère aussi un caractère ; les autres touches du moteur, non.
        return key == Key::Space && !m.command && !m.control;
    }
    !m.command && !m.control
}
/// Commande d'édition de la barre de menus.
fn edit(id: WindowId, command: crate::ui::text_input::EditCommand) {
    with(id, |s| s.input(UiEvent::Edit(command)));
}
/// Commande de fenêtre de la barre de menus, exécutée par la boucle.
fn command(command: WindowCommand) {
    PENDING.with(|p| p.borrow_mut().push(command));
}
fn closed(id: WindowId) {
    CLOSED.with(|c| c.borrow_mut().push(id));
}
/// Taille ou échelle changée : peinte tout de suite (la boucle ne tourne pas pendant un
/// redimensionnement à la souris).
fn resized(id: WindowId) {
    with(id, |s| {
        s.dirty = true;
        s.paint()
    });
}
/// Redimensionnement à la souris (début et fin du geste) : pendant le geste, les ombres
/// floutées des éléments qui ne font que changer de taille sont étirées au lieu d'être
/// recalculées à chaque pas ; la fin du geste les repeint exactement.
fn live_resize(id: WindowId, live: bool) {
    with(id, |s| {
        s.runtime.set_live_resize(live);
        if live {
            return Ok(());
        }
        s.dirty = true;
        s.paint()
    });
}
fn focus(id: WindowId, focused: bool) {
    with(id, |s| {
        if focused {
            s.refresh_cursor();
        }
        s.input(UiEvent::FocusChanged(focused)).map(drop)
    });
}
/// Fenêtre réduite ou cachée : ses ressources de rendu sont libérées.
fn hidden(id: WindowId) {
    with(id, |s| {
        s.hidden();
        Ok(())
    });
}
/// Visibilité changée : entièrement cachée (autres fenêtres, autre bureau), la fenêtre
/// libère ses ressources de rendu.
fn occlusion(id: WindowId) {
    let visible = window_of(id).is_none_or(|w| {
        w.occlusionState()
            .contains(objc2_app_kit::NSWindowOcclusionState::Visible)
    });
    if !visible {
        hidden(id);
    }
}
fn paint_now(id: WindowId) {
    with(id, |s| s.paint());
}
fn ids() -> Vec<WindowId> {
    SESSIONS.with(|s| s.borrow().iter().map(|s| s.id).collect())
}
fn open(descriptor: WindowDescriptor, mtm: MainThreadMarker) -> crate::Result<()> {
    if ids().contains(&descriptor.id) {
        return Err(crate::Error::DuplicateWindowId(descriptor.id));
    }
    let session = Session::open(descriptor, mtm)?;
    let id = session.id;
    SESSIONS.with(|s| s.borrow_mut().push(session));
    with(id, |s| s.paint());
    with(id, |s| {
        s.show();
        Ok(())
    });
    Ok(())
}
fn execute(command: WindowCommand, mtm: MainThreadMarker) {
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
            } else {
                with(id, |s| {
                    s.change(change, mtm);
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
fn service(policy: ExitPolicy, mtm: MainThreadMarker) -> bool {
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
    for id in ids() {
        with(id, |s| if s.dirty { s.paint() } else { Ok(()) });
    }
    let failed = ERROR.with(|e| e.borrow().is_some());
    let empty = SESSIONS.with(|s| s.borrow().is_empty());
    let pending = CLOSED.with(|c| !c.borrow().is_empty());
    !(EXIT.with(Cell::get) || failed || (empty && !pending))
}
/// Prochaine échéance d'attente : réveil le plus proche, ou tout de suite si une fenêtre
/// attend d'être peinte.
fn deadline() -> Option<f64> {
    SESSIONS.with(|s| {
        let sessions = s.borrow();
        if sessions.iter().any(|s| s.dirty) {
            return Some(0.);
        }
        let now = Instant::now();
        sessions
            .iter()
            .filter_map(|s| s.next_wake())
            .min()
            .map(|wake| wake.saturating_duration_since(now).as_secs_f64())
    })
}
struct RunGuard;
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
        RUNNING.with(|r| r.set(false));
    }
}
/// Boucle d'événements AppKit ; erreurs documentées par `platform::run_windows`.
pub(super) fn run_windows(windows: Vec<WindowDescriptor>, policy: ExitPolicy) -> crate::Result<()> {
    crate::window::validation::validate(&windows, policy)?;
    let mtm = MainThreadMarker::new().ok_or(crate::Error::Unsupported {
        reason: "the macOS event loop must run on the main thread",
    })?;
    if RUNNING.with(|r| r.replace(true)) {
        return Err(crate::Error::AlreadyRunning);
    }
    let _guard = RunGuard;
    ERROR.with(|e| e.borrow_mut().take());
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
    menu::install(&app, mtm);
    app.finishLaunching();
    for descriptor in windows {
        open(descriptor, mtm)?;
    }
    #[allow(deprecated)]
    app.activateIgnoringOtherApps(true);
    // SAFETY : constante de Foundation.
    let mode = unsafe { NSDefaultRunLoopMode };
    while autoreleasepool(|_| {
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
