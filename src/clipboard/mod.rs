//! The system clipboard, for the application itself: text that other programs read (the
//! Notepad, a browser), and data in a format of its own that only its own windows and
//! instances understand (objects copied from one level to another). Text fields keep
//! their own clipboard path; these functions are for everything else.
//!
//! ```no_run
//! use karbon_ui::clipboard::{self, ClipboardContent};
//!
//! // Copy: the objects in the editor's own format, and a text that any program reads.
//! clipboard::write(&ClipboardContent::text("Caisse, Tonneau").with("karbon-editor/objets", b"{...}".to_vec()))?;
//! // Paste: the editor's format first, the text otherwise.
//! if let Some(bytes) = clipboard::read_format("karbon-editor/objets")? {
//!     # let _ = bytes;
//! } else if let Some(text) = clipboard::read_text()? {
//!     # let _ = text;
//! }
//! # Ok::<(), karbon_ui::Error>(())
//! ```
//!
//! Windows only for now: elsewhere every call returns
//! [`Error::Unsupported`](crate::Error::Unsupported). See `docs/clipboard.md`.
mod content;

pub use content::{ClipboardContent, read_format, read_text, write, write_text};
