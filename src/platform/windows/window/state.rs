//! État natif par fenêtre. Le scope restaure le contexte après un message réentrant.
use super::{
    super::{
        accessibility,
        events::{grip, text},
        ffi::Hwnd,
        present,
    },
    shadow, tasks,
};
use crate::{
    ui::title_bar::CaptionButton,
    window::{WindowId, WindowOptions},
};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
    time::Instant,
};

pub(crate) struct WindowState {
    pub id: WindowId,
    pub options: Cell<WindowOptions>,
    pub drawing: RefCell<Option<present::DrawingState>>,
    pub accessibility: RefCell<Option<accessibility::NativeState>>,
    pub shadow: RefCell<Option<shadow::ShadowWindow>>,
    pub pressed: Cell<Option<CaptionButton>>,
    pub hovered: Cell<Option<CaptionButton>>,
    pub tracking: Cell<bool>,
    pub decoder: RefCell<text::Utf16Decoder>,
    pub input_active: Cell<bool>,
    pub caret_height: Cell<i32>,
    pub ime_enabled: Cell<bool>,
    pub composing: Cell<bool>,
    pub deadline: Cell<Option<Instant>>,
    /// Un FRAME_WAKE attend dans la file (une seule image d'animation demandée à la fois).
    pub frame_posted: Cell<bool>,
    /// Une image vient d'être présentée : la suivante peut être demandée par FRAME_WAKE.
    pub frame_ready: Cell<bool>,
    /// Redimensionnement ou déplacement à la souris en cours (WM_ENTERSIZEMOVE, ou
    /// geste d'une poignée de l'application).
    pub sizing: Cell<bool>,
    /// Geste en cours sur une poignée de l'application (`events/grip.rs`).
    pub grip: Cell<Option<grip::Grip>>,
    /// Dernière synchronisation UI Automation et focus alors annoncé.
    pub accessibility_synced: Cell<accessibility::Synced>,
    /// Une synchronisation UI Automation attend son minuteur.
    pub accessibility_pending: Cell<bool>,
    pub tasks: RefCell<Option<tasks::NativeTasks>>,
    /// Une panique a interrompu un message : l'application n'est plus appelée.
    pub poisoned: Cell<bool>,
    /// Commandes d'une fenêtre ouverte (`window/change.rs`) : redimensionnable par ses
    /// bords, taille maximale et proportions imposées (unités logiques).
    pub resizable: Cell<bool>,
    pub max_size: Cell<Option<(i32, i32)>>,
    pub aspect: Cell<Option<(u32, u32)>>,
    /// Plein écran : rectangle et état agrandi d'avant, rétablis à la sortie.
    pub fullscreen: Cell<Option<super::change::Restore>>,
    /// Dernier état remis à `Application::window_changed`.
    pub reported: Cell<Option<crate::window::WindowState>>,
}
impl WindowState {
    pub fn new(id: WindowId, options: WindowOptions) -> Rc<Self> {
        Rc::new(Self {
            id,
            options: Cell::new(options),
            drawing: RefCell::new(None),
            accessibility: RefCell::new(None),
            shadow: RefCell::new(None),
            pressed: Cell::new(None),
            hovered: Cell::new(None),
            tracking: Cell::new(false),
            decoder: RefCell::new(text::Utf16Decoder::default()),
            input_active: Cell::new(false),
            caret_height: Cell::new(0),
            ime_enabled: Cell::new(true),
            composing: Cell::new(false),
            deadline: Cell::new(None),
            frame_posted: Cell::new(false),
            frame_ready: Cell::new(false),
            sizing: Cell::new(false),
            grip: Cell::new(None),
            accessibility_synced: Cell::new(None),
            accessibility_pending: Cell::new(false),
            tasks: RefCell::new(None),
            poisoned: Cell::new(false),
            resizable: Cell::new(true),
            max_size: Cell::new(None),
            aspect: Cell::new(None),
            fullscreen: Cell::new(None),
            reported: Cell::new(None),
        })
    }
}
thread_local! {
    static CURRENT: RefCell<Rc<WindowState>> = RefCell::new(WindowState::new(WindowId(0), WindowOptions::DEFAULT));
    static WINDOWS: RefCell<HashMap<usize, Rc<WindowState>>> = RefCell::new(HashMap::new());
}

pub(crate) struct Scope(Rc<WindowState>);
impl Scope {
    pub fn new(state: Rc<WindowState>) -> Self {
        Self(CURRENT.with(|slot| slot.replace(state)))
    }
}
impl Drop for Scope {
    fn drop(&mut self) {
        CURRENT.with(|slot| {
            slot.replace(Rc::clone(&self.0));
        });
    }
}
pub(crate) fn enter(window: Hwnd) -> Scope {
    let state = WINDOWS
        .with(|windows| windows.borrow().get(&(window as usize)).cloned())
        .unwrap_or_else(|| CURRENT.with(|slot| Rc::clone(&slot.borrow())));
    Scope::new(state)
}
pub(super) fn register(window: Hwnd, state: Rc<WindowState>) {
    WINDOWS.with(|windows| {
        windows.borrow_mut().insert(window as usize, state);
    });
}
pub(crate) fn remove(window: Hwnd) {
    // Ne pas conserver le borrow du registre pendant la libération des ressources.
    let removed = WINDOWS.with(|windows| windows.borrow_mut().remove(&(window as usize)));
    drop(removed);
}
pub(crate) fn windows() -> Vec<(Hwnd, WindowId)> {
    WINDOWS.with(|windows| {
        windows
            .borrow()
            .iter()
            .map(|(&handle, state)| (handle as Hwnd, state.id))
            .collect()
    })
}

/// Accès à un champ du contexte courant, sans garder le registre emprunté.
pub(crate) struct WindowLocal<T> {
    field: fn(&WindowState) -> &T,
}
impl<T> WindowLocal<T> {
    pub const fn new(field: fn(&WindowState) -> &T) -> Self {
        Self { field }
    }
    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        let state = CURRENT.with(|slot| Rc::clone(&slot.borrow()));
        f((self.field)(&state))
    }
}

/// Options de la fenêtre courante (F3 et F5 les modifient).
pub(crate) static OPTIONS: WindowLocal<Cell<WindowOptions>> =
    WindowLocal::new(|state| &state.options);
pub(crate) fn options() -> WindowOptions {
    OPTIONS.with(Cell::get)
}
/// Redimensionnement ou déplacement à la souris en cours (WM_ENTERSIZEMOVE).
pub(crate) static SIZING: WindowLocal<Cell<bool>> = WindowLocal::new(|state| &state.sizing);
/// Redimensionnement ou déplacement à la souris en cours.
pub(crate) fn sizing() -> bool {
    SIZING.with(Cell::get)
}
