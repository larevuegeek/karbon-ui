//! Window configuration and window-level commands, shared by every platform backend.
//!
//! A process runs one or more windows, each described by a [`WindowDescriptor`] and
//! owning its own [`Application`](crate::application::Application). Start them with
//! [`platform::run`](crate::platform::run) or
//! [`platform::run_windows`](crate::platform::run_windows).
// Configuration et intentions communes à toutes les plateformes.
pub(crate) mod change;
pub(crate) mod validation;

pub use change::{WindowChange, WindowState};
mod dialog;
pub use dialog::{FileDialog, FileDialogKind, FileDialogResult, FileFilter};
mod tray;
pub use tray::{TrayEvent, TrayIcon, TrayMenuItem};
mod config;
pub use config::*;
