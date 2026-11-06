//! Un réveil ponctuel, désarmé à l'expiration ; aucun timer UI permanent au repos.
use super::super::{ffi::Hwnd, present::runtime};
use std::{cell::Cell, time::Instant};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    IsIconic, IsWindowVisible, KillTimer, PostMessageW, SetTimer, WM_APP,
};

const UI_TIMER: usize = 0x5755;
/// Image d'animation suivante : postée juste après une présentation, quand le réveil tombe
/// dans l'image qui vient. `SetTimer` (granularité ~15,6 ms, minimum 10 ms) ajoutée au
/// temps de calcul donnait une image toutes les ~30 ms ; `Present(1)` cale ici la cadence
/// sur le rafraîchissement de l'écran, comme `requestAnimationFrame`.
pub(crate) const FRAME_WAKE: u32 = WM_APP + 3;
const FRAME: std::time::Duration = std::time::Duration::from_millis(17);
static DEADLINE: super::state::WindowLocal<Cell<Option<Instant>>> =
    super::state::WindowLocal::new(|state| &state.deadline);
static FRAME_POSTED: super::state::WindowLocal<Cell<bool>> =
    super::state::WindowLocal::new(|state| &state.frame_posted);
static FRAME_READY: super::state::WindowLocal<Cell<bool>> =
    super::state::WindowLocal::new(|state| &state.frame_ready);
/// Appelé après chaque présentation : l'image d'animation suivante peut être demandée.
pub(crate) fn presented(window: Hwnd) {
    FRAME_READY.with(|r| r.set(true));
    sync(window);
}
pub(crate) fn sync(window: Hwnd) {
    // SAFETY : simples lectures d'état ; Windows valide le handle.
    let next = if unsafe { IsIconic(window) != 0 || IsWindowVisible(window) == 0 } {
        None
    } else {
        runtime::next_wake()
    };
    // Une image d'animation est déjà en route : elle resynchronisera le réveil.
    if FRAME_POSTED.with(Cell::get) {
        return;
    }
    DEADLINE.with(|slot| {
        if slot.get() == next && !FRAME_READY.with(Cell::get) {
            return;
        }
        if slot.take().is_some() {
            // SAFETY : arrêt d'un timer identifié de cette fenêtre.
            unsafe { KillTimer(window, UI_TIMER) };
        }
        if let Some(deadline) = next {
            let remaining = deadline.saturating_duration_since(Instant::now());
            // Seulement juste après une présentation : poster l'image suivante avant que
            // WM_PAINT ait pu passer affamerait l'affichage (les messages postés passent
            // avant WM_PAINT). Sinon, minuteur classique.
            if remaining <= FRAME && FRAME_READY.with(Cell::get) {
                // SAFETY : message privé posté à une fenêtre de ce thread.
                if unsafe { PostMessageW(window, FRAME_WAKE, 0, 0) } != 0 {
                    FRAME_READY.with(|r| r.set(false));
                    FRAME_POSTED.with(|p| p.set(true));
                    slot.set(Some(deadline));
                    return;
                }
            }
            let millis = remaining
                .as_millis()
                .saturating_add(u128::from(remaining.subsec_nanos() % 1_000_000 != 0))
                .clamp(10, 0x7fff_ffff) as u32;
            // SAFETY : sans callback, Windows publie WM_TIMER dans la file de la fenêtre.
            if unsafe { SetTimer(window, UI_TIMER, millis, None) } != 0 {
                slot.set(Some(deadline));
            }
        }
    });
}
pub(crate) fn fired(window: Hwnd, id: usize) -> bool {
    if id != UI_TIMER {
        return false;
    }
    shutdown(window);
    true
}
/// Réception de [`FRAME_WAKE`] : le réveil est consommé.
pub(crate) fn frame_fired() {
    FRAME_POSTED.with(|p| p.set(false));
    DEADLINE.with(|slot| slot.set(None));
}
pub(crate) fn shutdown(window: Hwnd) {
    // SAFETY : arrêt d'un timer identifié ; sans effet s'il n'existe pas.
    unsafe { KillTimer(window, UI_TIMER) };
    DEADLINE.with(|slot| slot.set(None));
}
