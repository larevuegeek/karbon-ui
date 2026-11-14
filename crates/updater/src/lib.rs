//! Self-update for desktop applications built with karbon-ui (or anything else).
//!
//! 1. [`Updater::check`] asks a release endpoint whether a newer version exists.
//! 2. [`Update::download`] downloads its package and verifies its minisign signature.
//! 3. [`Package::install`] installs it and schedules the restart; the application then
//!    exits (with karbon-ui: `WindowCommand::Exit`).
//!
//! Every call blocks: run them in a background task (karbon-ui `Task`, a thread, or
//! `spawn_blocking`).
//!
//! ```no_run
//! use karbon_updater::Updater;
//!
//! const PUBLIC_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6…"; // `tauri signer` / minisign public key
//!
//! let updater = Updater::new(PUBLIC_KEY, env!("CARGO_PKG_VERSION"))
//!     .endpoint("https://example.com/releases/{{target}}-{{arch}}/{{current_version}}");
//! if let Some(update) = updater.check()? {
//!     println!("{} is available: {}", update.version, update.notes.as_deref().unwrap_or(""));
//!     let package = update.download(|done, total| println!("{done} / {total:?} bytes"))?;
//!     package.install()?;
//!     std::process::exit(0); // the new version starts once this process has exited
//! }
//! # Ok::<(), karbon_updater::Error>(())
//! ```
//!
//! The endpoint protocol, the signatures and the Windows installer arguments are those of
//! the Tauri updater, so a server and a signing key made for it work unchanged; see
//! `README.md` for the server format, the packages each platform expects and how to sign
//! them.
// Organisation : `updater.rs` (vérification), `update.rs` (téléchargement), `install/`
// (installation par plateforme), `installation.rs` (comment l'application a été installée),
// `manifest.rs` et `json.rs` (réponse du serveur), `signature.rs`, `version.rs`, `error.rs`.
mod error;
mod install;
mod installation;
mod json;
mod manifest;
mod signature;
mod update;
mod updater;
mod version;

pub use error::Error;
pub use installation::Installation;
pub use update::{Package, Update};
pub use updater::Updater;
