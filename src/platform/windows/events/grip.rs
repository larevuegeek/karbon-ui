//! Poignée de redimensionnement de l'application (`data-resize-region`), suivie à la
//! main comme sous macOS : l'appui capture la souris, chaque mouvement donne la taille
//! « départ + déplacement du pointeur » en coordonnées de l'écran, avec les proportions
//! et les bornes de la fenêtre (`window::change::grip_size`), appliquée puis peinte
//! aussitôt : une taille par image. Le relâchement (ou la perte de la capture) termine
//! le geste ; `WindowState::dragging` vaut `true` entre les deux.
//!
//! Pas de boucle modale de Windows (`SC_SIZE`) : elle ne redimensionne qu'une fenêtre à
//! cadre épais et suit ses propres règles. La poignée vaut `HTCLIENT` au WM_NCHITTEST,
//! les bords d'une fenêtre fixe ne redimensionnent jamais.
use super::{
    super::{
        ffi::{Hwnd, scale},
        present::runtime,
        window::{change, state},
    },
    nonclient,
    pointer::point_from_message,
};
use crate::{ui::scene::PointF, window::ResizeEdge};
use std::cell::Cell;
use windows_sys::Win32::{
    Foundation::{POINT, RECT},
    Graphics::Gdi::UpdateWindow,
    UI::{
        Input::KeyboardAndMouse::{GetCapture, ReleaseCapture, SetCapture},
        WindowsAndMessaging::{
            GetCursorPos, HWND_TOP, IsZoomed, SWP_NOACTIVATE, SWP_NOZORDER, SetWindowPos,
        },
    },
};

/// Geste en cours : bord saisi, pointeur (écran, pixels physiques) et rectangle de la
/// fenêtre à l'appui.
#[derive(Clone, Copy)]
pub(crate) struct Grip {
    edge: ResizeEdge,
    cursor: POINT,
    rect: RECT,
}

static GRIP: state::WindowLocal<Cell<Option<Grip>>> = state::WindowLocal::new(|s| &s.grip);

/// Poignée sous `point` (pixels logiques de la fenêtre) ; aucune sur une fenêtre
/// agrandie ou en plein écran, dont la taille appartient à l'écran.
pub(super) fn at(window: Hwnd, point: PointF) -> Option<ResizeEdge> {
    // SAFETY : simple lecture de l'état de la fenêtre.
    if unsafe { IsZoomed(window) } != 0 || change::fullscreen() {
        return None;
    }
    runtime::resize_region_at(point)
}
fn cursor() -> Option<POINT> {
    let mut point = POINT::default();
    // SAFETY : `point` est une structure locale valide en écriture pendant l'appel.
    (unsafe { GetCursorPos(&mut point) } != 0).then_some(point)
}

/// WM_LBUTTONDOWN : un appui sur une poignée commence le geste ; `false` ailleurs.
pub(super) fn press(window: Hwnd, lparam: isize) -> bool {
    let p = point_from_message(lparam);
    let scale = scale(window);
    let point = PointF {
        x: p.x as f32 / scale,
        y: p.y as f32 / scale,
    };
    match (at(window, point), cursor()) {
        (Some(edge), Some(cursor)) => start(window, edge, cursor),
        _ => false,
    }
}
/// WM_MOUSEMOVE : nouvelle taille pendant le geste ; `false` sans geste en cours.
pub(super) fn drag(window: Hwnd) -> bool {
    match GRIP.with(Cell::get) {
        Some(_) => cursor().is_none_or(|cursor| follow(window, cursor)),
        None => false,
    }
}

/// Début du geste sur la poignée `edge`, le pointeur en `cursor` (écran, pixels
/// physiques) : souris capturée, redimensionnement en cours annoncé.
pub(crate) fn start(window: Hwnd, edge: ResizeEdge, cursor: POINT) -> bool {
    let Some(rect) = change::window_rect(window) else {
        return false;
    };
    GRIP.with(|g| g.set(Some(Grip { edge, cursor, rect })));
    // Recevoir mouvements et relâchement même hors de la fenêtre.
    // SAFETY : capture demandée pour la fenêtre qui traite le message.
    unsafe { SetCapture(window) };
    nonclient::enter_size_move(window);
    true
}
/// Pointeur en `cursor` pendant le geste : taille appliquée puis peinte aussitôt ;
/// `false` sans geste en cours.
pub(crate) fn follow(window: Hwnd, cursor: POINT) -> bool {
    let Some(grip) = GRIP.with(Cell::get) else {
        return false;
    };
    let delta = (cursor.x - grip.cursor.x, cursor.y - grip.cursor.y);
    let target = change::grip_rect(window, grip.edge, grip.rect, delta);
    let current = change::window_rect(window);
    if current.is_some_and(|r| {
        (r.left, r.top, r.right, r.bottom) == (target.left, target.top, target.right, target.bottom)
    }) {
        return true;
    }
    // SAFETY : fenêtre de ce thread ; aucun pointeur transmis. UpdateWindow peint la
    // nouvelle taille avant le mouvement suivant (les WM_MOUSEMOVE sont regroupés).
    unsafe {
        SetWindowPos(
            window,
            HWND_TOP,
            target.left,
            target.top,
            target.right - target.left,
            target.bottom - target.top,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );
        UpdateWindow(window);
    }
    true
}
/// Fin du geste (relâchement, capture perdue) : état final remis à l'application ;
/// `false` sans geste en cours.
pub(crate) fn finish(window: Hwnd) -> bool {
    // Retiré avant ReleaseCapture : WM_CAPTURECHANGED ne termine pas le geste deux fois.
    if GRIP.with(|g| g.take()).is_none() {
        return false;
    }
    // SAFETY : simple lecture, puis libération de la capture prise au début du geste.
    unsafe {
        if GetCapture() == window {
            ReleaseCapture();
        }
    }
    nonclient::exit_size_move(window);
    true
}
