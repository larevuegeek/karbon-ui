//! Requêtes du backend au runtime UI de la fenêtre courante : entrées, commandes et
//! tâches de l'application, curseur, capture, texte focalisé, accessibilité, réveils.
use super::{
    super::{
        ffi::{Hwnd, invalidate},
        window::{record_error, state},
    },
    DRAWING, DrawingState, composition,
};
use crate::ui::{
    rendering::scene::coverage,
    runtime::{Cursor, EventResult, UiEvent},
};
#[cfg(test)]
use composition::Presenter;
use std::cell::Cell;
use windows_sys::Win32::{
    Foundation::{E_FAIL, E_INVALIDARG},
    UI::{
        Accessibility::{
            UIA_E_ELEMENTNOTAVAILABLE, UIA_E_ELEMENTNOTENABLED, UIA_E_INVALIDOPERATION,
            UIA_E_NOTSUPPORTED,
        },
        WindowsAndMessaging::DestroyWindow,
    },
};

/// Une peinture est arrivée pendant que l'état de dessin était emprunté : elle est
/// redemandée quand l'emprunt est rendu (voir [`exclusive`]).
static PAINT_DEFERRED: state::WindowLocal<Cell<bool>> =
    state::WindowLocal::new(|state| &state.paint_deferred);

/// Appelé par `paint::paint_window` quand l'état de dessin est occupé.
pub(super) fn defer_paint() {
    PAINT_DEFERRED.with(|deferred| deferred.set(true));
}

/// Emprunt exclusif de l'état de dessin de la fenêtre courante, pour un appel qui peut
/// atteindre l'application. Une application qui pompe des messages pendant cet appel
/// (`MessageBoxW` dans `update`) fait réentrer la procédure de fenêtre : les requêtes
/// imbriquées trouvent l'état occupé et s'en passent (`try_borrow`), la peinture
/// imbriquée est reportée puis redemandée ici, une fois l'emprunt rendu.
/// `None` : fenêtre sans état de dessin, ou état déjà emprunté plus haut dans la pile
/// (seul l'emprunt le plus extérieur redemande la peinture : un appel imbriqué qui
/// invaliderait la fenêtre relancerait sans fin la peinture reportée).
fn exclusive<R>(window: Hwnd, f: impl FnOnce(&mut DrawingState) -> R) -> Option<R> {
    let result = DRAWING.with(|slot| {
        let mut value = slot.try_borrow_mut().ok()?;
        Some(value.as_mut().map(f))
    })?;
    if PAINT_DEFERRED.with(|deferred| deferred.replace(false)) {
        invalidate(window);
    }
    result
}

pub(crate) fn take_window_commands(window: Hwnd) -> Vec<crate::window::WindowCommand> {
    exclusive(window, |state| state.runtime.take_window_commands()).unwrap_or_default()
}

pub(crate) fn take_task_commands(window: Hwnd) -> Vec<crate::task::TaskCommand> {
    exclusive(window, |state| state.runtime.take_task_commands()).unwrap_or_default()
}
pub(crate) fn can_receive_tasks() -> bool {
    DRAWING.with(|slot| slot.try_borrow().is_ok_and(|state| state.is_some()))
}
/// Livre un résultat de tâche. Jamais appelé pendant un emprunt de l'état : `deliver`
/// vérifie d'abord [`can_receive_tasks`], les autres appels viennent de la boucle de
/// messages. Sans état de dessin (fenêtre fermée), le résultat est abandonné.
pub(crate) fn complete_task(window: Hwnd, result: crate::task::TaskResult) {
    let changed = exclusive(window, |state| state.runtime.task_completed(result));
    // Coalescer les reconstructions au prochain paint plutôt que remesurer par résultat.
    if changed == Some(true) {
        invalidate(window);
    }
}

pub(crate) fn cursor() -> Cursor {
    DRAWING.with(|slot| {
        slot.try_borrow()
            .ok()
            .and_then(|s| s.as_ref().map(|s| s.runtime.cursor()))
            .unwrap_or(Cursor::Arrow)
    })
}
/// Le menu contextuel intégré d'un champ de saisie est ouvert (tests).
#[cfg(test)]
pub(crate) fn edit_menu_open() -> bool {
    DRAWING.with(|slot| {
        slot.borrow()
            .as_ref()
            .is_some_and(|s| !s.runtime.edit_menu_rows().is_empty())
    })
}
pub(crate) fn has_capture() -> bool {
    DRAWING.with(|slot| {
        slot.try_borrow()
            .is_ok_and(|s| s.as_ref().is_some_and(|s| s.runtime.has_capture()))
    })
}
pub(crate) fn focused_input() -> Option<crate::ui::text_input::InputGeometry> {
    DRAWING.with(|slot| {
        slot.try_borrow()
            .ok()?
            .as_ref()?
            .runtime
            .focused_input_geometry()
    })
}
/// L'état de dessin existe et peut être lu. Faux pendant un emprunt (message imbriqué) :
/// la synchronisation du texte et de l'état de la fenêtre attend alors le suivant.
pub(crate) fn initialized() -> bool {
    can_receive_tasks()
}
pub(crate) fn next_wake() -> Option<std::time::Instant> {
    DRAWING.with(|slot| slot.try_borrow().ok()?.as_ref()?.runtime.next_wake())
}
pub(crate) fn tooltip() -> Option<crate::ui::tooltip::TooltipInfo> {
    DRAWING.with(|slot| slot.try_borrow().ok()?.as_ref()?.runtime.tooltip())
}
/// Un défilement est en cours dans la fenêtre courante.
pub(crate) fn scrolling() -> bool {
    DRAWING.with(|slot| {
        slot.try_borrow()
            .ok()
            .and_then(|s| s.as_ref().map(|s| s.runtime.is_scrolling()))
            .unwrap_or(false)
    })
}
pub(crate) fn accessibility_version()
-> Option<(u64, u64, u64, u64, Option<crate::ui::element::ElementId>)> {
    DRAWING.with(|slot| {
        slot.try_borrow()
            .ok()?
            .as_ref()
            .map(|s| s.runtime.accessibility_version())
    })
}
pub(crate) fn accessibility_snapshot() -> Option<Vec<crate::ui::runtime::AccessibleElement>> {
    DRAWING.with(|slot| {
        slot.try_borrow()
            .ok()?
            .as_ref()
            .map(|s| s.runtime.accessibility())
    })
}
pub(crate) fn accessibility_action(
    window: Hwnd,
    action: crate::ui::runtime::AccessibilityAction,
) -> i32 {
    use crate::ui::runtime::AccessibilityError;
    let result = exclusive(window, |state| -> crate::Result<i32> {
        state.prepare(window)?;
        let result = state.runtime.accessibility_action(action).map_or_else(
            |e| match e {
                AccessibilityError::Unavailable => UIA_E_ELEMENTNOTAVAILABLE as i32,
                AccessibilityError::Disabled => UIA_E_ELEMENTNOTENABLED as i32,
                AccessibilityError::ReadOnly => UIA_E_INVALIDOPERATION as i32,
                AccessibilityError::Unsupported => UIA_E_NOTSUPPORTED as i32,
                AccessibilityError::InvalidValue => E_INVALIDARG,
            },
            |_| 0,
        );
        if result >= 0 {
            state.prepare(window)?;
        }
        Ok(result)
    })
    // Sans état, ou état occupé par un appel de l'application en cours.
    .unwrap_or(Ok(UIA_E_ELEMENTNOTAVAILABLE as i32));
    match result {
        Ok(result) => result,
        Err(error) => {
            record_error(error);
            // SAFETY : destruction sur le thread propriétaire ; Windows valide le handle.
            unsafe { DestroyWindow(window) };
            E_FAIL
        }
    }
}
pub(crate) fn input(window: Hwnd, event: UiEvent) -> EventResult {
    // Déplacement du pointeur : l'arbre de l'image précédente suffit pour le test de
    // position et le glisser ; reconstruction, styles et layout attendent la prochaine
    // image, une seule fois pour tous les déplacements reçus entre-temps. Réveil
    // d'animation : l'horloge avance ici, l'image n'est préparée qu'une fois, au dessin.
    let deferred = matches!(event, UiEvent::PointerMove(_) | UiEvent::Timer(_));
    let result = exclusive(window, |state| -> crate::Result<EventResult> {
        let paints = state.runtime.stats().paints;
        if !deferred {
            state.prepare(window)?;
        }
        let mut result = state.runtime.event(event);
        if deferred {
            result.repaint |= state.runtime.has_state_styles();
        } else if result.repaint || state.runtime.has_state_styles() {
            state.prepare(window)?;
        }
        // prepare peut déjà avoir produit l'image, notamment avant Timer(now).
        // Cela ne signifie pas qu'elle a été présentée par WM_PAINT.
        result.repaint |= state.runtime.stats().paints != paints;
        Ok(result)
    })
    // Sans état, ou entrée reçue pendant un appel de l'application (boucle modale
    // ouverte par elle) : l'événement est ignoré.
    .unwrap_or_else(|| Ok(EventResult::default()));
    match result {
        Ok(result) => result,
        Err(error) => {
            record_error(error);
            // SAFETY : destruction sur le thread propriétaire ; Windows valide le handle.
            unsafe { DestroyWindow(window) };
            EventResult::default()
        }
    }
}
pub(crate) fn print_stats() {
    DRAWING.with(|slot| {
        if let Ok(s) = slot.try_borrow()
            && let Some(s) = s.as_ref()
        {
            println!("karbon-ui: UI stats: {:?}", s.runtime.stats());
            if let Some(profile) = &s.profile {
                profile.emit(s.renderer.label(), "snapshot");
            }
        }
    });
}

pub(crate) fn finish_profile() {
    DRAWING.with(|slot| {
        if let Ok(mut state) = slot.try_borrow_mut()
            && let Some(state) = state.as_mut()
            && let Some(profile) = state.profile.take()
        {
            profile.emit(state.renderer.label(), "close");
        }
    });
}

#[cfg(test)]
pub(crate) fn profile_report() -> Option<String> {
    DRAWING.with(|slot| {
        let state = slot.borrow();
        let state = state.as_ref()?;
        Some(
            state
                .profile
                .as_ref()?
                .report(state.renderer.label(), "test"),
        )
    })
}

/// Redimensionnement interactif (WM_ENTERSIZEMOVE … WM_EXITSIZEMOVE).
pub(crate) fn set_live_resize(live: bool) {
    DRAWING.with(|slot| {
        if let Ok(mut state) = slot.try_borrow_mut()
            && let Some(state) = state.as_mut()
        {
            state.runtime.set_live_resize(live);
        }
    });
}
/// Zone de déplacement de l'application (`data-drag-region`) sous `point`.
pub(crate) fn drag_region_at(point: crate::ui::scene::PointF) -> bool {
    DRAWING.with(|slot| {
        slot.try_borrow().is_ok_and(|state| {
            state
                .as_ref()
                .is_some_and(|state| state.runtime.drag_region_at(point))
        })
    })
}
/// Image d'animation qui doit aussi reconstruire la vue (un lot de données arrivé pendant
/// un défilement) : ce travail se fait tout de suite, à la place de l'attente de
/// [`wait_for_frame`]. Fait après elle, il s'ajoutait au temps de l'image, qui manquait
/// alors son affichage (11 ms de préparation après 12 ms d'attente inactive). Une tranche
/// du travail étalé de l'application passe au même moment.
pub(crate) fn prepare_pending_rebuild(window: Hwnd) {
    let result = exclusive(window, |state| -> crate::Result<()> {
        state.runtime.begin_frame_work(std::time::Instant::now());
        if state.runtime.rebuild_pending() {
            state.prepare(window)?;
        }
        Ok(())
    });
    if let Some(Err(error)) = result {
        record_error(error);
        // SAFETY : destruction sur le thread propriétaire ; Windows valide le handle.
        unsafe { DestroyWindow(window) };
    }
}
/// L'attente de l'écran est finie : voir `UiRuntime::hold_rebuild`.
pub(crate) fn hold_rebuild() {
    DRAWING.with(|slot| {
        if let Ok(mut state) = slot.try_borrow_mut()
            && let Some(state) = state.as_mut()
        {
            state.runtime.hold_rebuild(std::time::Instant::now());
        }
    });
}
/// Image d'animation : attend sa place dans la file de présentation avant que l'horloge
/// soit lue (voir `graphics::swapchain::FrameSlot`), pour que chaque image soit
/// échantillonnée à la même distance de son affichage.
pub(crate) fn wait_for_frame() {
    DRAWING.with(|slot| {
        if let Ok(mut state) = slot.try_borrow_mut()
            && let Some(state) = state.as_mut()
            && matches!(
                state.presenter,
                super::composition::Presenter::Composition(_)
            )
        {
            state.renderer.wait_for_frame();
        }
    });
}
/// Poignée de redimensionnement de l'application (`data-resize-region`) sous `point`.
pub(crate) fn resize_region_at(
    point: crate::ui::scene::PointF,
) -> Option<crate::window::ResizeEdge> {
    DRAWING.with(|slot| {
        let state = slot.try_borrow().ok()?;
        state.as_ref()?.runtime.resize_region_at(point)
    })
}
/// Remet l'état de la fenêtre à l'application ; `false` si elle n'est pas joignable
/// (runtime en cours d'usage ou pas encore initialisé), l'état est alors renvoyé plus tard.
pub(crate) fn window_changed(window: Hwnd, state: crate::window::WindowState) -> bool {
    let delivered = exclusive(window, |drawing| drawing.runtime.window_changed(state));
    if delivered == Some(true) {
        invalidate(window);
    }
    delivered.is_some()
}
/// Remet un clic ou un choix de menu de l'icône de la zone de notification à
/// l'application ; abandonné si le runtime n'est pas joignable (emprunté plus haut).
pub(crate) fn tray_event(window: Hwnd, event: crate::window::TrayEvent) {
    if exclusive(window, |drawing| drawing.runtime.tray_event(event)) == Some(true) {
        invalidate(window);
    }
}
/// Test de contenu pour WM_NCHITTEST. Seule la composition en a besoin : une fenêtre
/// layered laisse déjà Windows ignorer ses pixels transparents. Sans scène lisible
/// (peinture en cours, fenêtre non initialisée), le point est considéré couvert.
pub(crate) fn content_at(point: crate::ui::scene::PointF) -> bool {
    DRAWING.with(|slot| {
        slot.try_borrow().map_or(true, |state| {
            state.as_ref().is_none_or(|state| {
                state.presenter.path() == composition::Path::Layered
                    || coverage::covers(state.runtime.scene(), point)
            })
        })
    })
}

/// Chemin de présentation et chaîne d'échange rattachée au visuel (taille, alpha).
#[cfg(test)]
pub(crate) type PresentationState = (composition::Path, Option<(u32, u32, bool)>);
#[cfg(test)]
pub(crate) fn presentation_state() -> Option<PresentationState> {
    DRAWING.with(|slot| {
        let state = slot.borrow();
        let state = state.as_ref()?;
        let bound = match &state.presenter {
            Presenter::Composition(visual) => {
                let chain = visual.bound()?;
                assert_eq!(
                    state.renderer.swap_chain(),
                    Some(chain),
                    "le visuel doit afficher la chaîne courante du renderer"
                );
                Some(super::super::graphics::describe_swap_chain(chain))
            }
            Presenter::Layered(_) => None,
        };
        Some((state.presenter.path(), bound))
    })
}
#[cfg(test)]
pub(crate) fn simulate_device_loss() {
    DRAWING.with(|slot| {
        if let Some(state) = slot.borrow_mut().as_mut() {
            state.renderer.simulate_device_loss();
        }
    });
}
