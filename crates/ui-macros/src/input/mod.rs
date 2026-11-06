//! Entrées des macros : fichiers lus relativement à l'appelant (`.ui`, `.css`, `.svg`)
//! et lecture des arguments Rust (`ui_file!`, `styles!`, `css_file!`).
mod cursor;
pub mod files;
pub use cursor::Cursor;
pub use files::{Reader, Source};
