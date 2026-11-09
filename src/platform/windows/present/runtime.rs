//! Requêtes du backend au runtime UI de la fenêtre courante : entrées, commandes et
//! tâches de l'application, curseur, capture, texte focalisé, accessibilité, réveils.
use super::{
    super::{
        ffi::{Hwnd, invalidate},
        window::record_error,
    },
    DRAWING, composition,
};
use crate::ui::{
    rendering::scene::coverage,
    runtime::{Cursor, EventResult, UiEvent},
};
#[cfg(test)]
use composition::Presenter;
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

pub(crate) fn take_window_commands() -> Vec<crate::window::WindowCommand> {
    DRAWING.with(|slot| {
        slot.try_borrow_mut()
            .ok()
            .as_mut()
            .and_then(|state| state.as_mut())
            .map_or_else(Vec::new, |state| state.runtime.take_window_commands())
    })
}

pub(crate) fn take_task_commands() -> Vec<crate::task::TaskCommand> {
    DRAWING.with(|slot| {
        slot.try_borrow_mut()
            .ok()
            .as_mut()
            .and_then(|state| state.as_mut())
            .map_or_else(Vec::new, |state| state.runtime.take_task_commands())
    })
}
pub(crate) fn can_receive_tasks() -> bool {
    DRAWING.with(|slot| slot.try_borrow().is_ok_and(|state| state.is_some()))
}
pub(crate) fn complete_task(window: Hwnd, result: crate::task::TaskResult) {
    let changed = DRAWING.with(|slot| {
        let mut value = slot.borrow_mut();
        let Some(state) = value.as_mut() else {
            return false;
        };
        state.runtime.task_completed(result)
    });
    // Coalescer les reconstructions au prochain paint plutôt que remesurer par résultat.
    if changed {
        invalidate(window);
    }
}

pub(crate) fn cursor() -> Cursor {
    DRAWING.with(|slot| {
        slot.borrow()
            .as_ref()
            .map(|s| s.runtime.cursor())
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
        slot.borrow()
            .as_ref()
            .is_some_and(|s| s.runtime.has_capture())
    })
}
pub(crate) fn focused_input() -> Option<crate::ui::text_input::InputGeometry> {
    DRAWING.with(|slot| {
        slot.borrow()
            .as_ref()
            .and_then(|s| s.runtime.focused_input_geometry())
    })
}
pub(crate) fn initialized() -> bool {
    DRAWING.with(|slot| slot.borrow().is_some())
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
    let result = DRAWING.with(|slot| -> crate::Result<i32> {
        let mut value = slot.borrow_mut();
        let Some(state) = value.as_mut() else {
            return Ok(UIA_E_ELEMENTNOTAVAILABLE as i32);
        };
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
    });
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
    let result = DRAWING.with(|slot| -> crate::Result<EventResult> {
        let mut value = slot.borrow_mut();
        if let Some(state) = value.as_mut() {
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
        } else {
            Ok(EventResult::default())
        }
    });
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
        if let Some(s) = slot.borrow().as_ref() {
            println!("karbon-ui: UI stats: {:?}", s.runtime.stats());
            if let Some(profile) = &s.profile {
                profile.emit(s.renderer.label(), "snapshot");
            }
        }
    });
}

pub(crate) fn finish_profile() {
    DRAWING.with(|slot| {
        if let Some(state) = slot.borrow_mut().as_mut()
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
    let result = DRAWING.with(|slot| -> crate::Result<()> {
        if let Ok(mut state) = slot.try_borrow_mut()
            && let Some(state) = state.as_mut()
        {
            state.runtime.begin_frame_work(std::time::Instant::now());
            if state.runtime.rebuild_pending() {
                state.prepare(window)?;
            }
        }
        Ok(())
    });
    if let Err(error) = result {
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
    let delivered = DRAWING.with(|slot| {
        let Ok(mut value) = slot.try_borrow_mut() else {
            return None;
        };
        value
            .as_mut()
            .map(|drawing| drawing.runtime.window_changed(state))
    });
    if delivered == Some(true) {
        invalidate(window);
    }
    delivered.is_some()
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
