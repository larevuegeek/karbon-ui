//! Bootstrap `data-bs-*` behaviours mapped onto native components.
//!
//! When a template uses Bootstrap's declarative attributes (`data-bs-toggle="collapse"`,
//! `"tab"`, `"dropdown"`, `"modal"`, `"offcanvas"`, `data-bs-dismiss="alert"`…), the
//! template compiler wires the triggers to [`BootstrapAction`] messages and the targets
//! to [`shown`]: collapses and tab panes receive the `show` / `active` classes that
//! Bootstrap's JavaScript would add, dropdown menus open in a native popover anchored
//! to their toggle, modals in a native dialog. The runtime applies these messages
//! itself and rebuilds the view; the application does not handle them.
//!
//! The open/closed state lives in a per-thread table keyed by the target's `id`
//! (a window's UI thread). [`set_shown`] presets it, for instance before exporting a
//! screenshot with a menu open.
use std::{cell::RefCell, collections::HashMap};

/// Message emitted by a Bootstrap trigger; applied by the runtime, never delivered to
/// `Application::update`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BootstrapAction {
    /// Opens or closes `target` (collapse, dropdown, modal, offcanvas). Opening it closes
    /// the other members of `group` (accordion items sharing a `data-bs-parent`).
    Toggle {
        /// Identifier of the target.
        target: &'static str,
        /// State before any action (the `show` class written in the template).
        default: bool,
        /// Targets closed when `target` opens.
        group: &'static [&'static str],
    },
    /// Shows `target` and hides the other members of `group` (tabs and pills).
    Show {
        /// Identifier of the pane to show.
        target: &'static str,
        /// Panes of the same tab set.
        group: &'static [&'static str],
    },
    /// Hides `target` (close buttons, `data-bs-dismiss`, a popover or dialog closing).
    Hide(&'static str),
}

thread_local! {
    static STATE: RefCell<HashMap<String, bool>> = RefCell::new(HashMap::new());
}

/// Returns whether `key` is shown: the last action applied to it, else `default`.
pub fn shown(key: &str, default: bool) -> bool {
    STATE.with(|s| s.borrow().get(key).copied().unwrap_or(default))
}
/// Sets the state of `key`, as if the user had opened or closed it.
pub fn set_shown(key: &str, shown: bool) {
    STATE.with(|s| s.borrow_mut().insert(key.to_owned(), shown));
}
/// Forgets every state of this thread: targets return to their template defaults.
pub fn reset() {
    STATE.with(|s| s.borrow_mut().clear());
}
/// Applies `action` to the state of this thread.
pub fn apply(action: &BootstrapAction) {
    match action {
        BootstrapAction::Toggle {
            target,
            default,
            group,
        } => {
            let open = !shown(target, *default);
            if open {
                for other in group.iter().filter(|g| *g != target) {
                    set_shown(other, false);
                }
            }
            set_shown(target, open);
        }
        BootstrapAction::Show { target, group } => {
            for other in group.iter() {
                set_shown(other, false);
            }
            set_shown(target, true);
        }
        BootstrapAction::Hide(target) => set_shown(target, false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn toggles_groups_and_tabs() {
        reset();
        const GROUP: &[&str] = &["a", "b"];
        assert!(shown("a", true));
        apply(&BootstrapAction::Toggle {
            target: "b",
            default: false,
            group: GROUP,
        });
        assert!(shown("b", false) && !shown("a", true));
        apply(&BootstrapAction::Show {
            target: "a",
            group: GROUP,
        });
        assert!(shown("a", false) && !shown("b", true));
        apply(&BootstrapAction::Hide("a"));
        assert!(!shown("a", true));
        reset();
    }
}
