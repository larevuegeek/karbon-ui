//! Boutons de la barre de titre dessinée par le moteur : survol, appui, relâchement.
use super::{
    super::{
        ffi::{Hwnd, client_area, scale, win_rect},
        window::actions::execute_action,
    },
    pointer::{client_rect, logical_point},
    state,
};
use crate::ui::title_bar::{CaptionButton, TITLE_BAR_HEIGHT, button_at};
use std::cell::Cell;
use windows_sys::Win32::{
    Foundation::FALSE,
    Graphics::Gdi::InvalidateRect,
    UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture},
};

pub(crate) static PRESSED_BUTTON: state::WindowLocal<Cell<Option<CaptionButton>>> =
    state::WindowLocal::new(|state| &state.pressed);
pub(crate) static HOVERED_BUTTON: state::WindowLocal<Cell<Option<CaptionButton>>> =
    state::WindowLocal::new(|state| &state.hovered);

pub(super) fn button_from_message(window: Hwnd, lparam: isize) -> Option<CaptionButton> {
    if !state::options().title_bar {
        return None;
    }
    button_at(&client_rect(window)?, &logical_point(window, lparam))
}

fn invalidate_title_bar(window: Hwnd) {
    if let Some(mut title_bar) = client_area(window) {
        title_bar.bottom = (TITLE_BAR_HEIGHT as f32 * scale(window)).ceil() as i32;
        let title_bar = win_rect(title_bar);
        // Limiter le redessin à la bande du haut, sans effacer tout le contenu.
        // SAFETY : `title_bar` est un RECT local valide pendant l'appel.
        unsafe { InvalidateRect(window, &title_bar, FALSE) };
    }
}

pub(super) fn update_hover(window: Hwnd, button: Option<CaptionButton>) {
    let previous = HOVERED_BUTTON.with(|hover| hover.replace(button));
    if previous != button {
        invalidate_title_bar(window);
    }
}

/// Appui sur un bouton de la barre de titre ; faux si le point n'en désigne aucun.
pub(super) fn press(window: Hwnd, lparam: isize) -> bool {
    let Some(button) = button_from_message(window, lparam) else {
        return false;
    };
    PRESSED_BUTTON.with(|pressed| pressed.set(Some(button)));
    update_hover(window, Some(button));
    // Recevoir le relâchement même si la souris quitte la fenêtre.
    // SAFETY : capture demandée pour la fenêtre qui traite le message.
    unsafe { SetCapture(window) };
    invalidate_title_bar(window);
    true
}

/// Relâchement après un appui sur un bouton ; faux si aucun bouton n'était enfoncé.
pub(super) fn release(window: Hwnd, lparam: isize) -> bool {
    let pressed = PRESSED_BUTTON.with(|button| button.replace(None));
    let Some(button) = pressed else {
        return false;
    };
    let released = button_from_message(window, lparam);
    update_hover(window, released);
    // SAFETY : sans argument ; libère la capture prise au WM_LBUTTONDOWN.
    unsafe { ReleaseCapture() };
    invalidate_title_bar(window);
    // Appuyer puis relâcher sur le même bouton valide le clic.
    if released == Some(button) {
        execute_action(window, button.action());
    }
    true
}

/// La souris a été reprise par une autre interaction : le clic en cours est annulé.
pub(super) fn cancel(window: Hwnd) {
    if PRESSED_BUTTON.with(|button| button.replace(None)).is_some() {
        invalidate_title_bar(window);
    }
}
