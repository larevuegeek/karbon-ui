//! Procédure de fenêtre : entrée des messages Win32, garde contre les paniques et envoi
//! des `UiEvent` au runtime avec les suites qu'il demande (capture, presse-papiers…).
pub(crate) mod caption;
mod clipboard;
mod dispatch;
pub(crate) mod file_drop;
pub(crate) mod grip;
mod keyboard;
mod more_keys;
mod nonclient;
mod pointer;
mod pointer_lock;
mod procedure;
mod raw_input;
mod scancode;
pub(crate) mod text;
pub(crate) use procedure::*;
