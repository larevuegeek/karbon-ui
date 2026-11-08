//! Souris : positions logiques (DPI appliqué), boutons, molette, menu contextuel,
//! sortie de la fenêtre et curseur demandé par le runtime.
use super::{
    super::{
        ffi::{Hwnd, Point, Rect, client_area, scale, screen_to_client},
        present::{paint, runtime},
    },
    caption, grip, keyboard, send_ui,
};
use crate::ui::{
    runtime::{Cursor, UiEvent},
    scene::PointF,
    title_bar::TITLE_BAR_HEIGHT,
};
use std::cell::Cell;
use std::ptr::null_mut;
use windows_sys::Win32::{
    System::SystemServices::MK_SHIFT,
    UI::{
        Input::KeyboardAndMouse::{TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent, VK_SHIFT},
        WindowsAndMessaging::{
            HTCLIENT, IDC_APPSTARTING, IDC_ARROW, IDC_CROSS, IDC_HAND, IDC_HELP, IDC_IBEAM, IDC_NO,
            IDC_SIZEALL, IDC_SIZENESW, IDC_SIZENS, IDC_SIZENWSE, IDC_SIZEWE, IDC_WAIT, LoadCursorW,
            SPI_GETWHEELSCROLLCHARS, SPI_GETWHEELSCROLLLINES, SetCursor, SystemParametersInfoW,
            WM_MOUSEHWHEEL,
        },
    },
};

static TRACKING_MOUSE: super::state::WindowLocal<Cell<bool>> =
    super::state::WindowLocal::new(|state| &state.tracking);

pub(super) fn point_from_message(lparam: isize) -> Point {
    Point {
        x: lparam as i16 as i32,
        y: (lparam >> 16) as i16 as i32,
    }
}
/// Zone client en pixels logiques.
pub(super) fn client_rect(window: Hwnd) -> Option<Rect> {
    let mut area = client_area(window)?;
    let scale = scale(window);
    area.right = (area.right as f32 / scale).ceil() as i32;
    area.bottom = (area.bottom as f32 / scale).ceil() as i32;
    Some(area)
}
pub(super) fn logical_point(window: Hwnd, lparam: isize) -> Point {
    let p = point_from_message(lparam);
    let scale = scale(window);
    Point {
        x: (p.x as f32 / scale).floor() as i32,
        y: (p.y as f32 / scale).floor() as i32,
    }
}
fn pointer(window: Hwnd, lparam: isize) -> PointF {
    let p = point_from_message(lparam);
    let scale = scale(window);
    PointF {
        x: p.x as f32 / scale,
        y: p.y as f32 / scale,
    }
}
fn update_cursor() {
    let id = match runtime::cursor() {
        Cursor::Arrow => IDC_ARROW,
        Cursor::Hand => IDC_HAND,
        Cursor::ResizeHorizontal => IDC_SIZEWE,
        Cursor::ResizeVertical => IDC_SIZENS,
        Cursor::Text => IDC_IBEAM,
        Cursor::NotAllowed => IDC_NO,
        Cursor::Wait => IDC_WAIT,
        Cursor::Progress => IDC_APPSTARTING,
        Cursor::Help => IDC_HELP,
        Cursor::Move => IDC_SIZEALL,
        Cursor::Crosshair => IDC_CROSS,
        Cursor::ResizeDiagonal => IDC_SIZENWSE,
        Cursor::ResizeAntiDiagonal => IDC_SIZENESW,
    };
    // SAFETY : curseur système partagé, désigné par un identifiant MAKEINTRESOURCE.
    let cursor = unsafe { LoadCursorW(null_mut(), id) };
    if !cursor.is_null() {
        // SAFETY : `cursor` vient d'être chargé et reste valide (ressource système).
        unsafe { SetCursor(cursor) };
    }
}
fn context_menu_screen_point(lparam: isize) -> Option<Point> {
    let p = point_from_message(lparam);
    (!(p.x == -1 && p.y == -1)).then_some(p)
}

/// WM_SETCURSOR : curseur du runtime sur la zone client, celui de Windows ailleurs.
pub(super) fn set_cursor(lparam: isize, default_proc: impl FnOnce() -> isize) -> isize {
    if lparam & 0xFFFF == HTCLIENT as isize {
        update_cursor();
        1
    } else {
        default_proc()
    }
}
/// WM_CONTEXTMENU : clic droit, touche Menu ou Shift+F10.
pub(super) fn context_menu(
    window: Hwnd,
    lparam: isize,
    default_proc: impl FnOnce() -> isize,
) -> isize {
    let point = if let Some(p) = context_menu_screen_point(lparam) {
        let Some(p) = screen_to_client(window, p) else {
            return default_proc();
        };
        let scale = scale(window);
        Some(PointF {
            x: p.x as f32 / scale,
            y: p.y as f32 / scale,
        })
    } else {
        None
    };
    if send_ui(window, UiEvent::ContextMenu(point)) {
        0
    } else {
        default_proc()
    }
}
/// WM_MOUSEWHEEL et WM_MOUSEHWHEEL.
pub(super) fn wheel(
    window: Hwnd,
    message: u32,
    wparam: usize,
    lparam: isize,
    default_proc: impl FnOnce() -> isize,
) -> isize {
    // La molette fournit des coordonnées ÉCRAN signées, contrairement à WM_MOUSEMOVE.
    let Some(p) = screen_to_client(window, point_from_message(lparam)) else {
        return default_proc();
    };
    let horizontal = message == WM_MOUSEHWHEEL || wparam & MK_SHIFT as usize != 0; // Shift+molette
    let mut units = 3u32;
    // SAFETY : Windows écrit un u32 dans `units`, valide pendant l'appel.
    unsafe {
        SystemParametersInfoW(
            if horizontal {
                SPI_GETWHEELSCROLLCHARS
            } else {
                SPI_GETWHEELSCROLLLINES
            },
            0,
            (&mut units as *mut u32).cast(),
            0,
        )
    };
    let pixels = if units == u32::MAX {
        client_rect(window).map_or(400., |r| {
            if horizontal {
                r.right as f32
            } else {
                (r.bottom - TITLE_BAR_HEIGHT).max(0) as f32
            }
        }) * 0.9
    } else {
        // Comme Chromium : une ligne fait 100/3 px, trois lignes par cran font 100 px.
        units as f32 * 100. / 3.
    };
    let raw = (wparam >> 16) as i16;
    // Crans entiers (souris) : glissement doux ; deltas fins (pavé tactile précis) : immédiats.
    let notched = raw != 0 && raw % 120 == 0;
    let delta = (raw as f32) / 120. * pixels;
    let delta = if message == WM_MOUSEHWHEEL {
        delta
    } else {
        -delta
    };
    let scale = scale(window);
    let position = PointF {
        x: p.x as f32 / scale,
        y: p.y as f32 / scale,
    };
    let (delta_x, delta_y) = if horizontal { (delta, 0.) } else { (0., delta) };
    let event = if notched {
        UiEvent::Wheel {
            position,
            delta_x,
            delta_y,
        }
    } else {
        UiEvent::Scroll {
            position,
            delta_x,
            delta_y,
        }
    };
    if send_ui(window, event) {
        0
    } else {
        default_proc()
    }
}
pub(super) fn mouse_move(window: Hwnd, lparam: isize) -> isize {
    // Geste d'une poignée : l'UI ne reçoit ni mouvement ni relâchement.
    if grip::drag(window) {
        return 0;
    }
    caption::update_hover(window, caption::button_from_message(window, lparam));
    send_ui(window, UiEvent::PointerMove(pointer(window, lparam)));
    update_cursor();
    paint::flush_if_due(window);
    if !TRACKING_MOUSE.with(Cell::get) {
        let mut tracking = TRACKMOUSEEVENT {
            cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
            dwFlags: TME_LEAVE,
            hwndTrack: window,
            dwHoverTime: 0,
        };
        // SAFETY : structure locale complète, cbSize renseigné.
        if unsafe { TrackMouseEvent(&mut tracking) } != 0 {
            TRACKING_MOUSE.with(|active| active.set(true));
        }
    }
    0
}
pub(super) fn mouse_leave(window: Hwnd) -> isize {
    TRACKING_MOUSE.with(|active| active.set(false));
    caption::update_hover(window, None);
    send_ui(window, UiEvent::PointerLeave);
    0
}
pub(super) fn button_down(window: Hwnd, lparam: isize) -> isize {
    if !caption::press(window, lparam) && !grip::press(window, lparam) {
        send_ui(
            window,
            if keyboard::is_pressed(VK_SHIFT) {
                UiEvent::PointerDownExtended(pointer(window, lparam))
            } else {
                UiEvent::PointerDown(pointer(window, lparam))
            },
        );
    }
    0
}
pub(super) fn button_up(window: Hwnd, lparam: isize) -> isize {
    if !grip::finish(window) && !caption::release(window, lparam) {
        send_ui(window, UiEvent::PointerUp(pointer(window, lparam)));
    }
    0
}
pub(super) fn capture_changed(window: Hwnd) -> isize {
    // Capture reprise pendant le geste d'une poignée (fenêtre système, Échap…) : il
    // s'arrête à la taille atteinte.
    grip::finish(window);
    if runtime::has_capture() {
        send_ui(window, UiEvent::Cancel);
    }
    // Une autre interaction a récupéré la souris : annuler le clic.
    caption::cancel(window);
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn context_menu_decodes_keyboard_sentinel_and_signed_screen_coordinates() {
        assert!(context_menu_screen_point(-1).is_none());
        assert!(context_menu_screen_point(u32::MAX as usize as isize).is_none());
        let packed = (-350_i16 as u16 as u32) | ((-120_i16 as u16 as u32) << 16);
        let p = context_menu_screen_point(packed as isize).unwrap();
        assert_eq!((p.x, p.y), (-350, -120));
        let p = context_menu_screen_point(0).unwrap();
        assert_eq!((p.x, p.y), (0, 0));
    }
}
