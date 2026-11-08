//! Commandes d'une fenêtre ouverte (`WindowChange`) et état remis à l'application
//! (`Application::window_changed`).
//!
//! Unités : tailles en pixels logiques de l'écran de la fenêtre (DPI par écran),
//! positions en pixels logiques au DPI système, comme `WindowOptions::x` à l'ouverture
//! (`create.rs`). La fenêtre n'a pas de cadre (`WM_NCCALCSIZE` rend 0) : zone client et
//! rectangle de la fenêtre ont la même taille.
use super::{
    super::{
        ffi::{Hwnd, client_area, last_error, scale},
        present::runtime,
    },
    dwm, shadow, state,
};
use crate::window::{
    ResizeEdge, WindowChange, WindowState,
    change::{
        Edges, ScreenRect, anchored, bound_size, centered, constrain_aspect, grip_size,
        invalid_change, keep_on_screen,
    },
};
use std::cell::Cell;
use windows_sys::Win32::{
    Foundation::RECT,
    Graphics::Gdi::{GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow},
    UI::{
        HiDpi::GetDpiForSystem,
        WindowsAndMessaging::{
            GWL_STYLE, GetWindowLongPtrW, GetWindowRect, HWND_NOTOPMOST, HWND_TOP, HWND_TOPMOST,
            IsIconic, IsZoomed, SW_HIDE, SW_MAXIMIZE, SW_RESTORE, SW_SHOW, SW_SHOWNOACTIVATE,
            SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
            SetForegroundWindow, SetWindowLongPtrW, SetWindowPos, ShowWindow, WMSZ_BOTTOM,
            WMSZ_BOTTOMLEFT, WMSZ_BOTTOMRIGHT, WMSZ_LEFT, WMSZ_RIGHT, WMSZ_TOP, WMSZ_TOPLEFT,
            WMSZ_TOPRIGHT, WS_MAXIMIZEBOX, WS_THICKFRAME,
        },
    },
};

/// Rectangle et état agrandi d'avant le plein écran.
#[derive(Clone, Copy)]
pub(crate) struct Restore {
    rect: RECT,
    maximized: bool,
}

static RESIZABLE: state::WindowLocal<Cell<bool>> = state::WindowLocal::new(|s| &s.resizable);
static MAX_SIZE: state::WindowLocal<Cell<Option<(i32, i32)>>> =
    state::WindowLocal::new(|s| &s.max_size);
static ASPECT: state::WindowLocal<Cell<Option<(u32, u32)>>> =
    state::WindowLocal::new(|s| &s.aspect);
static FULLSCREEN: state::WindowLocal<Cell<Option<Restore>>> =
    state::WindowLocal::new(|s| &s.fullscreen);
static REPORTED: state::WindowLocal<Cell<Option<WindowState>>> =
    state::WindowLocal::new(|s| &s.reported);

/// La fenêtre courante couvre tout son écran.
pub(crate) fn fullscreen() -> bool {
    FULLSCREEN.with(|f| f.get().is_some())
}
/// L'utilisateur peut tirer les bords de la fenêtre courante (ni fixe, ni plein écran).
pub(crate) fn edges_resizable() -> bool {
    RESIZABLE.with(Cell::get) && !fullscreen()
}

fn system_scale() -> f32 {
    // SAFETY : simple lecture du DPI système.
    unsafe { GetDpiForSystem() }.max(96) as f32 / 96.
}
pub(crate) fn window_rect(window: Hwnd) -> Option<RECT> {
    let mut rect = RECT::default();
    // SAFETY : `rect` est une structure locale valide en écriture pendant l'appel.
    (unsafe { GetWindowRect(window, &mut rect) } != 0).then_some(rect)
}
/// Zone de travail (`work`) ou écran entier de l'écran le plus proche, en pixels physiques.
fn monitor_area(window: Hwnd, work: bool) -> Option<RECT> {
    // SAFETY : simple requête sur une fenêtre de ce thread.
    let monitor = unsafe { MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST) };
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    // SAFETY : `info` est une structure locale complète, cbSize renseigné.
    if monitor.is_null() || unsafe { GetMonitorInfoW(monitor, &mut info) } == 0 {
        return None;
    }
    Some(if work { info.rcWork } else { info.rcMonitor })
}
fn screen_rect(rect: RECT) -> ScreenRect {
    ScreenRect {
        x: rect.left,
        y: rect.top,
        width: rect.right - rect.left,
        height: rect.bottom - rect.top,
    }
}
fn set_window_pos(window: Hwnd, after: Hwnd, rect: (i32, i32, i32, i32), flags: u32) {
    // SAFETY : fenêtre de ce thread ; aucun pointeur transmis.
    if unsafe { SetWindowPos(window, after, rect.0, rect.1, rect.2, rect.3, flags) } == 0 {
        eprintln!(
            "karbon-ui: window change failed: {}",
            last_error("SetWindowPos")
        );
    }
}
fn move_to(window: Hwnd, (x, y): (i32, i32)) {
    set_window_pos(
        window,
        HWND_TOP,
        (x, y, 0, 0),
        SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
    );
}
/// Taille logique bornée par la taille minimale et maximale de la fenêtre courante.
fn bounded(width: i32, height: i32) -> (i32, i32) {
    let options = state::options();
    let (max_width, max_height) = MAX_SIZE.with(Cell::get).unwrap_or((i32::MAX, i32::MAX));
    (
        width.min(max_width).max(options.min_width),
        height.min(max_height).max(options.min_height),
    )
}
fn resize(window: Hwnd, width: i32, height: i32) {
    let (width, height) = bounded(width, height);
    let scale = scale(window);
    set_window_pos(
        window,
        HWND_TOP,
        (
            0,
            0,
            (width as f32 * scale).round() as i32,
            (height as f32 * scale).round() as i32,
        ),
        SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
    );
}
/// Taille actuelle ramenée dans les bornes (après un changement de borne).
fn enforce_bounds(window: Hwnd) {
    if let Some(area) = client_area(window) {
        let scale = scale(window);
        let width = (area.right as f32 / scale).round() as i32;
        let height = (area.bottom as f32 / scale).round() as i32;
        if bounded(width, height) != (width, height) {
            resize(window, width, height);
        }
    }
}
fn set_style(window: Hwnd, resizable: bool) {
    // SAFETY : lecture et écriture du style d'une fenêtre de ce thread.
    unsafe {
        let style = GetWindowLongPtrW(window, GWL_STYLE) as u32;
        let frame = WS_THICKFRAME | WS_MAXIMIZEBOX;
        let style = if resizable {
            style | frame
        } else {
            style & !frame
        };
        SetWindowLongPtrW(window, GWL_STYLE, style as isize);
    }
    set_window_pos(
        window,
        HWND_TOP,
        (0, 0, 0, 0),
        SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
    );
}
fn show(window: Hwnd, command: i32) {
    // SAFETY : commande d'affichage sur une fenêtre de ce thread.
    unsafe { ShowWindow(window, command) };
}
fn set_fullscreen(window: Hwnd, enter: bool) {
    let current = FULLSCREEN.with(Cell::get);
    match (enter, current) {
        (true, None) => {
            // SAFETY : simple lecture de l'état de la fenêtre.
            let maximized = unsafe { IsZoomed(window) } != 0;
            if maximized {
                show(window, SW_RESTORE);
            }
            let (Some(rect), Some(monitor)) = (window_rect(window), monitor_area(window, false))
            else {
                return;
            };
            FULLSCREEN.with(|f| f.set(Some(Restore { rect, maximized })));
            set_style(window, false);
            dwm::set_rounded(window, false);
            let area = screen_rect(monitor);
            set_window_pos(
                window,
                HWND_TOP,
                (area.x, area.y, area.width, area.height),
                SWP_NOACTIVATE,
            );
        }
        (false, Some(restore)) => {
            FULLSCREEN.with(|f| f.set(None));
            set_style(window, RESIZABLE.with(Cell::get));
            if !state::options().transparent {
                dwm::set_rounded(window, true);
            }
            let area = screen_rect(restore.rect);
            set_window_pos(
                window,
                HWND_TOP,
                (area.x, area.y, area.width, area.height),
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
            if restore.maximized {
                show(window, SW_MAXIMIZE);
            }
        }
        _ => {}
    }
}

/// Exécute un changement sur la fenêtre courante (scope déjà entré par l'appelant).
pub(crate) fn execute(window: Hwnd, change: WindowChange) -> crate::Result<()> {
    if let Some(reason) = invalid_change(&change) {
        return Err(crate::Error::invalid_window_options(reason));
    }
    match change {
        WindowChange::Size { width, height } => resize(window, width, height),
        WindowChange::MinSize(size) => {
            let (width, height) = size.unwrap_or((1, 1));
            state::OPTIONS.with(|options| {
                let mut value = options.get();
                value.min_width = width;
                value.min_height = height;
                options.set(value);
            });
            enforce_bounds(window);
        }
        WindowChange::MaxSize(size) => {
            MAX_SIZE.with(|m| m.set(size));
            enforce_bounds(window);
        }
        WindowChange::AspectRatio(ratio) => ASPECT.with(|a| a.set(ratio)),
        WindowChange::Resizable(resizable) => {
            RESIZABLE.with(|r| r.set(resizable));
            if !fullscreen() {
                set_style(window, resizable);
            }
        }
        WindowChange::Topmost(topmost) => {
            state::OPTIONS.with(|options| {
                let mut value = options.get();
                value.topmost = topmost;
                options.set(value);
            });
            let after = if topmost {
                HWND_TOPMOST
            } else {
                HWND_NOTOPMOST
            };
            set_window_pos(
                window,
                after,
                (0, 0, 0, 0),
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
        WindowChange::Position { x, y } => {
            let scale = system_scale();
            move_to(
                window,
                (
                    (x as f32 * scale).round() as i32,
                    (y as f32 * scale).round() as i32,
                ),
            );
        }
        WindowChange::Center | WindowChange::KeepOnScreen => {
            let (Some(rect), Some(area)) = (window_rect(window), monitor_area(window, true)) else {
                return Ok(());
            };
            let (rect, area) = (screen_rect(rect), screen_rect(area));
            let target = if change == WindowChange::Center {
                centered(rect.width, rect.height, area)
            } else {
                keep_on_screen(rect, area)
            };
            if target != (rect.x, rect.y) {
                move_to(window, target);
            }
        }
        WindowChange::Maximized(maximize) => {
            // SAFETY : simple lecture de l'état de la fenêtre.
            let zoomed = unsafe { IsZoomed(window) } != 0;
            if maximize && !zoomed {
                set_fullscreen(window, false);
                show(window, SW_MAXIMIZE);
            } else if !maximize && zoomed {
                show(window, SW_RESTORE);
            }
        }
        WindowChange::Fullscreen(enter) => set_fullscreen(window, enter),
        WindowChange::Visible(true) => show(
            window,
            if state::options().activate {
                SW_SHOW
            } else {
                SW_SHOWNOACTIVATE
            },
        ),
        WindowChange::Visible(false) => {
            show(window, SW_HIDE);
            shadow::update(window);
        }
        WindowChange::Focus => {
            // SAFETY : lecture d'état puis commandes d'affichage sur une fenêtre de ce thread.
            unsafe {
                if IsIconic(window) != 0 {
                    ShowWindow(window, SW_RESTORE);
                }
                ShowWindow(window, SW_SHOW);
                SetForegroundWindow(window);
            }
        }
    }
    Ok(())
}

/// État actuel de la fenêtre courante ; réduite, elle garde la géométrie d'avant.
pub(crate) fn snapshot(window: Hwnd) -> WindowState {
    let previous = REPORTED.with(Cell::get).unwrap_or_default();
    // SAFETY : simples lectures de l'état de la fenêtre.
    let (minimized, maximized) = unsafe { (IsIconic(window) != 0, IsZoomed(window) != 0) };
    let scale = scale(window);
    let options = state::options();
    let mut snapshot = WindowState {
        scale,
        maximized,
        minimized,
        fullscreen: fullscreen(),
        topmost: options.topmost,
        resizable: RESIZABLE.with(Cell::get),
        dragging: state::sizing(),
        ..previous
    };
    if !minimized {
        let system = system_scale();
        if let Some(rect) = window_rect(window) {
            snapshot.x = (rect.left as f32 / system).round() as i32;
            snapshot.y = (rect.top as f32 / system).round() as i32;
        }
        if let Some(area) = client_area(window) {
            snapshot.width = (area.right as f32 / scale).round() as i32;
            snapshot.height = (area.bottom as f32 / scale).round() as i32;
        }
    }
    snapshot
}

/// Remet l'état à l'application s'il a changé depuis le dernier envoi.
pub(crate) fn notify(window: Hwnd) {
    if !runtime::initialized() {
        return;
    }
    let current = snapshot(window);
    if REPORTED.with(Cell::get) == Some(current) {
        return;
    }
    if runtime::window_changed(window, current) {
        REPORTED.with(|r| r.set(Some(current)));
    }
}

/// Bornes de la fenêtre courante en pixels physiques : taille minimale, maximale.
fn physical_limits(window: Hwnd) -> ((i32, i32), (i32, i32)) {
    let scale = scale(window);
    let options = state::options();
    let max = MAX_SIZE
        .with(Cell::get)
        .unwrap_or((i32::MAX / 4, i32::MAX / 4));
    let physical = |v: i32| (v as f32 * scale).round() as i32;
    (
        (physical(options.min_width), physical(options.min_height)),
        (physical(max.0), physical(max.1)),
    )
}

/// Rectangle de la fenêtre courante (pixels physiques de l'écran) pendant le geste
/// d'une poignée `edge` partie de `start`, le pointeur déplacé de `delta` : proportions
/// et bornes de la fenêtre, bords opposés à la poignée gardés en place.
pub(crate) fn grip_rect(window: Hwnd, edge: ResizeEdge, start: RECT, delta: (i32, i32)) -> RECT {
    let start = screen_rect(start);
    let (min, max) = physical_limits(window);
    let size = grip_size(
        edge,
        (start.width, start.height),
        delta,
        ASPECT.with(Cell::get),
        min,
        max,
    );
    let rect = anchored(edge, start, size);
    RECT {
        left: rect.x,
        top: rect.y,
        right: rect.x + rect.width,
        bottom: rect.y + rect.height,
    }
}

/// WM_SIZING : proportions imposées pendant un redimensionnement à la souris.
///
/// # Safety
/// `lparam` est celui de WM_SIZING : il pointe vers le RECT modifiable du geste.
pub(crate) unsafe fn sizing(window: Hwnd, edge: usize, lparam: isize) -> bool {
    let Some(ratio) = ASPECT.with(Cell::get) else {
        return false;
    };
    // SAFETY : pour WM_SIZING, lparam pointe vers un RECT fourni par Windows.
    let rect = unsafe { &mut *(lparam as *mut RECT) };
    let edge = edge as u32;
    let edges = Edges {
        left: matches!(edge, WMSZ_LEFT | WMSZ_TOPLEFT | WMSZ_BOTTOMLEFT),
        top: matches!(edge, WMSZ_TOP | WMSZ_TOPLEFT | WMSZ_TOPRIGHT),
        right: matches!(edge, WMSZ_RIGHT | WMSZ_TOPRIGHT | WMSZ_BOTTOMRIGHT),
        bottom: matches!(edge, WMSZ_BOTTOM | WMSZ_BOTTOMLEFT | WMSZ_BOTTOMRIGHT),
    };
    let size = constrain_aspect(
        (rect.right - rect.left, rect.bottom - rect.top),
        ratio,
        edges,
    );
    // Les bornes s'appliquent après : réduire les deux côtés ensemble garde la proportion.
    let (min, max) = physical_limits(window);
    let (width, height) = bound_size(size, min, max, true);
    if edges.left {
        rect.left = rect.right - width;
    } else {
        rect.right = rect.left + width;
    }
    if edges.top {
        rect.top = rect.bottom - height;
    } else {
        rect.bottom = rect.top + height;
    }
    true
}

/// Taille maximale de la fenêtre courante en pixels physiques (WM_GETMINMAXINFO).
pub(crate) fn max_track_size(window: Hwnd) -> Option<(i32, i32)> {
    let scale = scale(window);
    MAX_SIZE.with(Cell::get).map(|(width, height)| {
        (
            (width as f32 * scale).floor() as i32,
            (height as f32 * scale).floor() as i32,
        )
    })
}
