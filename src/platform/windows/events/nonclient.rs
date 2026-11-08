//! Zone non cliente et géométrie de la fenêtre : test de position (bords, déplacement,
//! clic traversant), limites de taille, DPI, redimensionnement, activation.
use super::{
    super::{
        ffi::{Hwnd, invalidate, scale, screen_to_client},
        present::runtime,
        window::{
            change,
            dwm::{apply_style, update_frame},
            shadow,
        },
    },
    accessibility, caption, grip,
    pointer::{client_rect, point_from_message},
    state,
};
use crate::ui::{
    scene::PointF,
    title_bar::{self, HitRegion},
};
use std::ptr::null_mut;
use windows_sys::Win32::{
    Foundation::RECT,
    Graphics::Gdi::{GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow},
    UI::WindowsAndMessaging::{
        HTBOTTOM, HTBOTTOMLEFT, HTBOTTOMRIGHT, HTCAPTION, HTCLIENT, HTLEFT, HTNOWHERE, HTRIGHT,
        HTTOP, HTTOPLEFT, HTTOPRIGHT, HTTRANSPARENT, IsZoomed, MINMAXINFO, SWP_NOACTIVATE,
        SWP_NOZORDER, SetWindowPos,
    },
};

/// WM_ENTERSIZEMOVE.
pub(super) fn enter_size_move(window: Hwnd) -> isize {
    state::SIZING.with(|s| s.set(true));
    runtime::set_live_resize(true);
    change::notify(window);
    0
}
/// WM_EXITSIZEMOVE.
pub(super) fn exit_size_move(window: Hwnd) -> isize {
    state::SIZING.with(|s| s.set(false));
    runtime::set_live_resize(false);
    invalidate(window);
    if !state::options().transparent {
        shadow::update(window);
    }
    accessibility::sync(window);
    change::notify(window);
    0
}
/// WM_DPICHANGED.
///
/// # Safety
/// `lparam` est celui de WM_DPICHANGED : il pointe vers un RECT fourni par Windows.
pub(super) unsafe fn dpi_changed(window: Hwnd, lparam: isize) -> isize {
    // Le rectangle suggéré est en pixels physiques.
    // SAFETY : pour WM_DPICHANGED, lparam pointe vers un RECT fourni par Windows.
    let rect = unsafe { &*(lparam as *const RECT) };
    // SAFETY : fenêtre valide pendant son message ; aucun pointeur transmis.
    unsafe {
        SetWindowPos(
            window,
            null_mut(),
            rect.left,
            rect.top,
            rect.right - rect.left,
            rect.bottom - rect.top,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }
    invalidate(window);
    0
}
/// WM_ACTIVATE.
pub(super) fn activate(window: Hwnd, default_proc: impl FnOnce() -> isize) -> isize {
    if !state::options().transparent {
        update_frame(window);
    }
    let result = default_proc();
    if !state::options().transparent {
        shadow::update(window);
    }
    result
}
/// WM_WINDOWPOSCHANGED.
pub(super) fn position_changed(window: Hwnd, default_proc: impl FnOnce() -> isize) -> isize {
    // Laisser Windows produire WM_MOVE/WM_SIZE, puis synchroniser l'ombre.
    let result = default_proc();
    if !state::options().transparent {
        shadow::update(window);
    }
    accessibility::sync(window);
    change::notify(window);
    result
}
/// WM_DWMCOMPOSITIONCHANGED.
pub(super) fn composition_changed(window: Hwnd) -> isize {
    if !state::options().transparent {
        apply_style(window);
    }
    0
}
/// WM_SIZE.
pub(super) fn size(window: Hwnd) -> isize {
    // Recalculer et redessiner les boutons après chaque changement de taille.
    if !state::options().transparent {
        update_frame(window);
    }
    caption::HOVERED_BUTTON.with(|hover| hover.set(None));
    invalidate(window);
    0
}
/// WM_GETMINMAXINFO.
///
/// # Safety
/// `lparam` est celui de WM_GETMINMAXINFO : il pointe vers un MINMAXINFO modifiable.
pub(super) unsafe fn min_max_info(window: Hwnd, lparam: isize) -> isize {
    // Pour ce message, Windows fournit une structure valide à compléter.
    // Les autres limites conservent les valeurs proposées par Windows.
    // SAFETY : pour WM_GETMINMAXINFO, lparam pointe vers un MINMAXINFO modifiable.
    let limits = unsafe { &mut *(lparam as *mut MINMAXINFO) };
    let options = state::options();
    let scale = scale(window);
    limits.ptMinTrackSize.x = (options.min_width as f32 * scale).ceil() as i32;
    limits.ptMinTrackSize.y = (options.min_height as f32 * scale).ceil() as i32;
    if let Some((width, height)) = change::max_track_size(window) {
        limits.ptMaxTrackSize.x = width;
        limits.ptMaxTrackSize.y = height;
    }

    // Agrandir dans la zone de travail de l'écran concerné,
    // en laissant la barre des tâches accessible.
    // SAFETY : simple requête sur la fenêtre en cours de traitement.
    let monitor = unsafe { MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST) };
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    // SAFETY : `info` est une structure locale complète, cbSize renseigné.
    if !monitor.is_null() && unsafe { GetMonitorInfoW(monitor, &mut info) } != 0 {
        limits.ptMaxPosition.x = info.rcWork.left - info.rcMonitor.left;
        limits.ptMaxPosition.y = info.rcWork.top - info.rcMonitor.top;
        limits.ptMaxSize.x = info.rcWork.right - info.rcWork.left;
        limits.ptMaxSize.y = info.rcWork.bottom - info.rcWork.top;
    }
    0
}
/// WM_NCHITTEST : Windows demande quelle zone se trouve sous le pointeur.
/// Priorité aux poignées de l'application, puis aux coins, aux bords, et enfin à la
/// bande ou aux zones de déplacement.
pub(super) fn hit_test(window: Hwnd, lparam: isize, default_proc: impl FnOnce() -> isize) -> isize {
    // Windows stocke deux coordonnées signées de 16 bits dans lparam.
    // Lire la position locale et les dimensions actuelles, à chaque test.
    let Some(mut point) = screen_to_client(window, point_from_message(lparam)) else {
        // En cas d'échec, conserver le traitement Windows par défaut.
        return default_proc();
    };
    let Some(area) = client_rect(window) else {
        return default_proc();
    };
    let scale = scale(window);
    // Composition : DWM ne teste pas l'alpha. Hors du contenu peint (fond
    // transparent, coins arrondis), le clic passe aux fenêtres du même thread
    // placées dessous, comme sur les pixels transparents d'une fenêtre layered.
    let logical = PointF {
        x: point.x as f32 / scale,
        y: point.y as f32 / scale,
    };
    if !runtime::content_at(logical) {
        return HTTRANSPARENT as isize;
    }
    point.x = (point.x as f32 / scale).floor() as i32;
    point.y = (point.y as f32 / scale).floor() as i32;
    // Poignée de l'application : zone cliente, son geste est suivi à la main (`grip.rs`)
    // et redimensionne même une fenêtre fixe. Elle l'emporte sur les bords et sur une
    // zone de déplacement qui la contiendrait.
    if grip::at(window, logical).is_some() {
        return HTCLIENT as isize;
    }

    // Fenêtre agrandie, fixe ou en plein écran : pas de bords à tirer.
    // SAFETY : simple lecture de l'état de la fenêtre.
    let fixed = unsafe { IsZoomed(window) } != 0 || !change::edges_resizable();
    let zone = match title_bar::hit_test(&area, &point, fixed) {
        HitRegion::Outside => HTNOWHERE,
        HitRegion::Caption if state::options().title_bar => HTCAPTION,
        HitRegion::Caption | HitRegion::Client if runtime::drag_region_at(logical) => HTCAPTION,
        HitRegion::Client => HTCLIENT,
        HitRegion::Caption => HTCLIENT,
        HitRegion::Left => HTLEFT,
        HitRegion::Right => HTRIGHT,
        HitRegion::Top => HTTOP,
        HitRegion::Bottom => HTBOTTOM,
        HitRegion::TopLeft => HTTOPLEFT,
        HitRegion::TopRight => HTTOPRIGHT,
        HitRegion::BottomLeft => HTBOTTOMLEFT,
        HitRegion::BottomRight => HTBOTTOMRIGHT,
    };
    zone as isize
}
