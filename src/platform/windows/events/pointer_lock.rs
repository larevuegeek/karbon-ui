//! Pointeur verrouillé sous Windows (`Element::pointer_lock`) : pendant le geste d'un
//! élément qui le demande, le pointeur est caché, tenu dans la fenêtre (`ClipCursor`) et
//! ramené à sa place après chaque mouvement (`SetCursorPos`) ; le runtime reçoit l'écart,
//! le mouvement propre de la souris, qui ne bute donc jamais au bord de l'écran. À la fin
//! du geste, ou si la fenêtre perd le clavier ou la souris, tout est rendu : pointeur
//! visible, libre, à la place de l'appui.
use super::super::{
    ffi::{Hwnd, Point},
    present::runtime,
};
use std::cell::Cell;
use windows_sys::Win32::{
    Foundation::{POINT, RECT},
    Graphics::Gdi::ClientToScreen,
    UI::WindowsAndMessaging::{ClipCursor, GetClientRect, SetCursorPos, ShowCursor},
};

/// Le verrou en cours : la fenêtre, la place de l'appui dans sa zone client et à
/// l'écran (pixels physiques).
#[derive(Clone, Copy)]
struct Lock {
    window: Hwnd,
    client: Point,
    screen: POINT,
}

thread_local! {
    static LOCK: Cell<Option<Lock>> = const { Cell::new(None) };
}

fn current(window: Hwnd) -> Option<Lock> {
    LOCK.with(Cell::get).filter(|lock| lock.window == window)
}

/// Ce qu'un mouvement à `client` (pixels physiques) devient quand le pointeur est
/// verrouillé à `anchor` : la place de l'appui et l'écart, ou rien quand il n'y a pas
/// d'écart (le mouvement que fait `SetCursorPos` en ramenant le pointeur).
pub(super) fn locked_move(anchor: Point, client: Point) -> Option<(Point, (i32, i32))> {
    let motion = (client.x - anchor.x, client.y - anchor.y);
    (motion != (0, 0)).then_some((anchor, motion))
}

/// Un mouvement à `client` dans `window` : `None` pour l'ignorer, sinon la position à
/// donner au runtime et, verrouillé, le mouvement propre de la souris en pixels
/// physiques.
pub(super) fn filter_move(window: Hwnd, client: Point) -> Option<(Point, Option<(i32, i32)>)> {
    match current(window) {
        Some(lock) => locked_move(lock.client, client).map(|(at, motion)| (at, Some(motion))),
        None => Some((client, None)),
    }
}

/// Après une entrée : prend le verrou que demande le geste en cours, ou le rend s'il ne
/// le demande plus ; verrouillé, ramène le pointeur à sa place.
pub(super) fn sync(window: Hwnd, client: Point) {
    let wanted = runtime::pointer_lock_wanted();
    match (current(window), wanted) {
        (None, true) => start(window, client),
        (Some(lock), false) => stop(lock),
        (Some(lock), true) => {
            // SAFETY : coordonnées de l'écran, simple déplacement du pointeur de l'appui.
            unsafe { SetCursorPos(lock.screen.x, lock.screen.y) };
        }
        (None, false) => {}
    }
}

fn start(window: Hwnd, client: Point) {
    let mut screen = POINT {
        x: client.x,
        y: client.y,
    };
    let mut area = RECT::default();
    // SAFETY : fenêtre valide sur son fil ; structures locales passées par adresse.
    unsafe {
        ClientToScreen(window, &mut screen);
        GetClientRect(window, &mut area);
        let mut corner = POINT {
            x: area.left,
            y: area.top,
        };
        ClientToScreen(window, &mut corner);
        let clip = RECT {
            left: corner.x,
            top: corner.y,
            right: corner.x + (area.right - area.left),
            bottom: corner.y + (area.bottom - area.top),
        };
        ClipCursor(&clip);
        ShowCursor(0);
    }
    LOCK.with(|lock| {
        lock.set(Some(Lock {
            window,
            client,
            screen,
        }))
    });
}

fn stop(lock: Lock) {
    LOCK.with(|slot| slot.set(None));
    // SAFETY : rend ce que `start` a pris : pointeur libre, à la place de l'appui, visible
    // (un seul `ShowCursor(1)` pour le seul `ShowCursor(0)`).
    unsafe {
        ClipCursor(std::ptr::null());
        SetCursorPos(lock.screen.x, lock.screen.y);
        ShowCursor(1);
    }
}

/// La fenêtre a perdu le clavier ou la souris : un geste verrouillé finit (l'élément
/// reçoit le relâchement des boutons tenus) et le pointeur est rendu.
pub(super) fn lost(window: Hwnd) {
    let Some(lock) = current(window) else {
        return;
    };
    let result = runtime::raw_input(window, false, |ui| ui.pointer_lock_lost());
    if result.repaint {
        super::super::ffi::invalidate(window);
    }
    stop(lock);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_locked_move_reports_the_motion_and_stays_at_the_press() {
        let anchor = Point { x: 300, y: 200 };
        let (at, motion) = locked_move(anchor, Point { x: 310, y: 195 }).unwrap();
        assert_eq!((at.x, at.y, motion), (300, 200, (10, -5)));
        // Le mouvement fait en ramenant le pointeur n'en est pas un.
        assert!(locked_move(anchor, anchor).is_none());
    }

    #[test]
    fn without_a_lock_a_move_goes_on_unchanged() {
        let client = Point { x: 12, y: 34 };
        let (at, motion) = filter_move(std::ptr::null_mut(), client).unwrap();
        assert_eq!((at.x, at.y, motion), (12, 34, None));
    }
}
