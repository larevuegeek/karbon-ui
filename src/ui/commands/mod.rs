//! Commands of an application and their keyboard shortcuts (KUI-044), and the command
//! palette (KUI-042).
//!
//! - [`Shortcut`]: a key, by its name or its place, with Ctrl, Shift, Alt and the Windows
//!   key; read and written in clear, in English or French (« Ctrl+Shift+D »,
//!   « Ctrl+Maj+D »), matched against a [`KeyPress`](crate::ui::runtime::KeyPress).
//! - [`Keymap`]: the registry of [`Command`]s (identifier, name, description, context,
//!   default shortcuts) and the shortcuts the user changed: the command a key leads to
//!   in the active contexts, the [`Conflict`]s, the text of a shortcut for menus and
//!   tooltips, and the changes read and written in clear or in a
//!   [`LocalStorage`](crate::storage::LocalStorage).
//! - [`CommandPalette`]: a search field and the commands that match what is typed
//!   ([`fuzzy_score`]), with their shortcuts; keyboard and mouse.
//!
//! See `docs/commands.md`.
mod keymap;
mod palette;
mod places;
mod search;
mod shortcut;
#[cfg(test)]
mod tests;

pub use keymap::{Command, Conflict, GLOBAL, Keymap};
pub use palette::{CommandPalette, PaletteItem, PaletteMsg, PaletteReply, PaletteState};
pub use search::{fuzzy_score, rank};
pub use shortcut::{Shortcut, ShortcutError, ShortcutKey, ShortcutNames};
