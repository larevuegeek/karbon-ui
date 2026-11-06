//! Validation des descripteurs de fenêtre, indépendante de l'OS : tout backend
//! interactif la fait avant d'ouvrir ses fenêtres (Windows : `window::run_windows`).
// Le backend headless n'ouvre aucune fenêtre : sans backend interactif, rien ne l'appelle.
#![cfg_attr(not(windows), allow(dead_code))]
use crate::window::{ExitPolicy, PresentMode, WindowDescriptor, WindowOptions};
use std::collections::HashSet;

pub(crate) fn validate(windows: &[WindowDescriptor], policy: ExitPolicy) -> crate::Result<()> {
    if windows.is_empty() {
        return Err(crate::Error::invalid_input(
            "at least one window is required",
        ));
    }
    let mut ids = HashSet::new();
    if let Some(window) = windows.iter().find(|window| !ids.insert(window.id)) {
        return Err(crate::Error::DuplicateWindowId(window.id));
    }
    if let ExitPolicy::MainWindowClosed(id) = policy
        && !ids.contains(&id)
    {
        return Err(crate::Error::invalid_input(
            "the exit policy's main window must be one of the initial windows",
        ));
    }
    Ok(())
}

/// Première incohérence des options d'une fenêtre, en anglais pour l'erreur publique.
pub(crate) fn invalid_options(options: &WindowOptions) -> Option<&'static str> {
    if options.width <= 0 || options.height <= 0 {
        Some("width and height must be positive")
    } else if options.min_width <= 0 || options.min_height <= 0 {
        Some("min_width and min_height must be positive")
    } else if options.click_through && !options.transparent {
        Some("click_through requires transparent")
    } else if options.click_through && options.presentation == PresentMode::Composition {
        Some("click_through requires the layered presentation, not PresentMode::Composition")
    } else if options.title.contains('\0') {
        Some("the title cannot contain a NUL character")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::{Application, ViewContext},
        ui::{element::Element, widgets},
        window::WindowId,
    };
    struct Empty;
    impl Application for Empty {
        type Message = ();
        fn update(&mut self, _: ()) -> bool {
            false
        }
        fn view(&self, _: &ViewContext) -> Element {
            widgets::text(1, "Vide")
        }
    }
    fn descriptor(id: u64) -> WindowDescriptor {
        WindowDescriptor::new(WindowId(id), WindowOptions::DEFAULT, Empty)
    }
    #[test]
    fn initial_windows_require_distinct_ids_and_an_existing_main_window() {
        assert!(matches!(
            validate(&[], ExitPolicy::LastWindowClosed),
            Err(crate::Error::InvalidInput { .. })
        ));
        let windows = [descriptor(1), descriptor(1)];
        assert!(matches!(
            validate(&windows, ExitPolicy::LastWindowClosed),
            Err(crate::Error::DuplicateWindowId(WindowId(1)))
        ));
        let windows = [descriptor(1), descriptor(2)];
        assert!(validate(&windows, ExitPolicy::LastWindowClosed).is_ok());
        assert!(validate(&windows, ExitPolicy::MainWindowClosed(WindowId(1))).is_ok());
        assert!(matches!(
            validate(&windows, ExitPolicy::MainWindowClosed(WindowId(3))),
            Err(crate::Error::InvalidInput { .. })
        ));
    }
}
