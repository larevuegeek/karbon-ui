//! Pont UI Automation. Les callbacks COM lisent un snapshot ; les mutations
//! repassent sur le thread UI par une file possédée, sans pointeur de pile envoyé.
//!
//! `tree` décrit les nœuds publiés, `bridge` relie les fournisseurs COM au thread UI,
//! `sync` reconstruit le snapshot depuis le runtime et `notify` émet les événements ;
//! `provider` et les modules de patterns implémentent les interfaces COM.
mod bridge;
mod choices;
mod expand;
mod ffi;
mod notify;
mod provider;
mod range_value;
mod sync;
#[cfg(test)]
mod tests;
mod text;
mod tooltip;
mod tree;

use sync::Stamp;
pub(crate) use sync::{Synced, sync, timer_fired};

use super::{
    ffi::Hwnd,
    present::runtime,
    window::{actions::execute_action, state, timer},
};
use crate::{
    ui::{
        element::{CheckState, ChoiceState, ElementId, Role},
        geometry::{Point, Rect},
        runtime::AccessibilityAction,
    },
    window::WindowAction,
};
use bridge::{Bridge, Command, lock};
use ffi::*;
use notify::{raise, raise_property};
use std::{
    cell::RefCell,
    ptr::null_mut,
    sync::{Arc, Mutex, atomic::Ordering},
};
use tree::{Node, NodeId, Snapshot, physical};
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, WPARAM},
        UI::Accessibility::{
            IRawElementProviderSimple, UIA_PROPERTY_ID, UiaDisconnectProvider, UiaRect,
            UiaReturnRawElementProvider, UiaRootObjectId,
        },
    },
    core::{HRESULT, Interface, Result},
};
use windows_sys::Win32::{
    Graphics::Gdi::InvalidateRect,
    UI::Input::KeyboardAndMouse::{GetFocus, ReleaseCapture, SetFocus},
};

pub(crate) struct NativeState {
    bridge: Arc<Bridge>,
    root: IRawElementProviderSimple,
    last: Option<Stamp>,
}
static STATE: state::WindowLocal<RefCell<Option<NativeState>>> =
    state::WindowLocal::new(|state| &state.accessibility);
pub(crate) fn initialize(window: Hwnd) -> crate::Result<()> {
    let bridge = Bridge::new(window);
    let empty = Node {
        key: NodeId::Root,
        parent: None,
        name: state::options().title.into(),
        help: String::new(),
        control_type: 50032,
        is_dialog: false,
        enabled: true,
        focusable: state::options().activate,
        focused: false,
        offscreen: true,
        invokable: false,
        value: None,
        text: None,
        choice: None,
        selection_container: None,
        range: None,
        expanded: None,
        current: None,
        read_only: true,
        bounds: UiaRect::default(),
        clipped: UiaRect::default(),
    };
    *lock(&bridge.snapshot) = Arc::new(Snapshot::new(vec![Arc::new(empty)]));
    let root = bridge
        .interface::<IRawElementProviderSimple>(NodeId::Root)
        .map_err(|e| crate::Error::Platform {
            operation: "UI Automation provider",
            code: e.code().0,
        })?;
    STATE.with(|state| {
        *state.borrow_mut() = Some(NativeState {
            bridge,
            root,
            last: None,
        })
    });
    Ok(())
}
pub(crate) fn shutdown() {
    if let Some(state) = STATE.with(|state| state.borrow_mut().take()) {
        state.bridge.close();
        // SAFETY: le fournisseur racine est vivant ; UIA libère ses références distantes.
        let _ = unsafe { UiaDisconnectProvider(&state.root) };
    }
}
pub(crate) fn get_object(window: Hwnd, wparam: usize, lparam: isize) -> Option<isize> {
    if lparam as i32 != UiaRootObjectId {
        return None;
    }
    // Garder la référence locale hors du RefCell pendant les callbacks possibles de UIA.
    let object = STATE.with(|state| {
        state.borrow().as_ref().and_then(|s| {
            s.bridge
                .interface::<IRawElementProviderSimple>(NodeId::Root)
                .ok()
        })
    });
    object.map(|provider| {
        // SAFETY: paramètres WM_GETOBJECT transmis tels quels ; UIA prend sa propre référence.
        unsafe {
            UiaReturnRawElementProvider(HWND(window), WPARAM(wparam), LPARAM(lparam), &provider)
        }
        .0
    })
}
pub(crate) fn handle_message(window: Hwnd, message: u32, request_id: usize) -> Option<isize> {
    if message != COMMAND_MESSAGE {
        return None;
    }
    let bridge = STATE.with(|s| s.borrow().as_ref().map(|s| Arc::clone(&s.bridge)))?;
    let request = {
        let mut requests = lock(&bridge.requests);
        requests
            .iter()
            .position(|r| r.id == request_id)
            .and_then(|i| requests.remove(i))
    };
    let Some(request) = request else {
        return Some(0);
    };
    if request.cancelled.load(Ordering::Acquire) {
        return Some(0);
    }
    let result = apply(window, &request.command);
    timer::sync(window);
    *lock(&request.reply) = Some(result);
    Some(0)
}
/// Donne le focus clavier natif à la fenêtre ; échec si Windows le refuse.
fn focus_window(window: Hwnd) -> HRESULT {
    if !state::options().activate {
        return UNSUPPORTED;
    }
    // SAFETY: fenêtre du thread courant ; seul le handle est transmis.
    let focused = unsafe {
        SetFocus(window);
        GetFocus()
    };
    if focused != window {
        return INVALID_OPERATION;
    }
    OK
}
fn apply(window: Hwnd, command: &Command) -> HRESULT {
    let action = match command {
        Command::Focus(NodeId::Root) => {
            let result = focus_window(window);
            if result.is_err() {
                return result;
            }
            sync(window);
            return OK;
        }
        Command::Focus(NodeId::Element(id)) => {
            let result = focus_window(window);
            if result.is_err() {
                return result;
            }
            AccessibilityAction::Focus(ElementId(*id))
        }
        Command::Focus(NodeId::SelectOption(owner, key)) => {
            let result = focus_window(window);
            if result.is_err() {
                return result;
            }
            AccessibilityAction::HighlightOption {
                id: ElementId(*owner),
                option: *key,
            }
        }
        Command::SelectChoice(NodeId::SelectOption(owner, key)) => {
            AccessibilityAction::ChooseOption {
                id: ElementId(*owner),
                option: *key,
            }
        }
        Command::Expand(NodeId::Element(id), expanded) => AccessibilityAction::ExpandSelect {
            id: ElementId(*id),
            expanded: *expanded,
        },
        Command::Invoke(NodeId::Element(id)) => AccessibilityAction::Invoke(ElementId(*id)),
        Command::Toggle(NodeId::Element(id)) => AccessibilityAction::Toggle(ElementId(*id)),
        Command::SelectChoice(NodeId::Element(id)) => {
            AccessibilityAction::SelectChoice(ElementId(*id))
        }
        Command::SetNumber(NodeId::Element(id), value) => AccessibilityAction::SetNumber {
            id: ElementId(*id),
            value: *value,
        },
        Command::SetValue(NodeId::Element(id), value) => AccessibilityAction::SetValue {
            id: ElementId(*id),
            value: value.clone(),
        },
        Command::SelectText(NodeId::Element(id), range, expected) => {
            AccessibilityAction::SelectText {
                id: ElementId(*id),
                range: range.clone(),
                expected: Arc::clone(expected),
            }
        }
        Command::RevealText(NodeId::Element(id), byte) => AccessibilityAction::RevealText {
            id: ElementId(*id),
            byte: *byte,
        },
        Command::Invoke(NodeId::Caption(button)) => {
            let action = match button {
                0 => WindowAction::Minimize,
                1 => WindowAction::ToggleMaximize,
                2 => WindowAction::Close,
                _ => return UNAVAILABLE,
            };
            execute_action(window, action);
            sync(window);
            return OK;
        }
        _ => return UNSUPPORTED,
    };
    let captured = runtime::has_capture();
    let result = HRESULT(runtime::accessibility_action(window, action));
    if captured && !runtime::has_capture() {
        // SAFETY: libère la capture souris prise par ce thread pour l'action.
        unsafe { ReleaseCapture() };
    }
    if result.is_ok() {
        // SAFETY: invalidation complète de la fenêtre du thread courant.
        unsafe { InvalidateRect(window, std::ptr::null(), 0) };
        super::events::text::sync(window);
        sync(window);
    }
    result
}
