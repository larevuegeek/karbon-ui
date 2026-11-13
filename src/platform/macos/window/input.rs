//! Entrées reçues par la vue (`view.rs`) : pointeur (clic, mouvement, défilement, menu
//! contextuel, poignées de redimensionnement et zones de déplacement), clavier et commandes
//! d'édition de la barre de menus, remis au runtime de la session.
use super::event_loop::*;
use super::{change, events};

/// Bouton ou geste du pointeur reçu par la vue.
#[derive(Clone, Copy)]
pub(in crate::platform::macos) enum Pointer {
    Move,
    Down,
    Up,
    Context,
    Leave,
    Scroll,
}
pub(in crate::platform::macos) fn pointer(
    id: WindowId,
    kind: Pointer,
    point: PointF,
    event: &NSEvent,
) {
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
pub(in crate::platform::macos) fn key(id: WindowId, event: &NSEvent, down: bool) -> bool {
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
pub(in crate::platform::macos) fn edit(id: WindowId, command: crate::ui::text_input::EditCommand) {
    with(id, |s| s.input(UiEvent::Edit(command)));
}
