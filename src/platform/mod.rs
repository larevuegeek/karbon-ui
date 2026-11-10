//! Platform backends: open windows and run the event loop, or drive an application
//! without any window.
//!
//! [`run`] opens a single window and returns when it closes; [`run_windows`] opens
//! several windows with an [`ExitPolicy`](crate::window::ExitPolicy). Interactive
//! backends exist for Windows, macOS (AppKit) and Linux (Wayland, or X11 without a
//! Wayland compositor); on other targets, and on Linux without a compositor nor an X
//! server, both return [`Error::Unsupported`](crate::Error::Unsupported).
//!
//! The [`headless`] backend is compiled on every target. It has no event loop; instead
//! [`HeadlessApp`](headless::HeadlessApp) drives an application in tests (clicks, typing,
//! keys, scrolling, resizing, time) with deterministic text metrics, and its software
//! rasteriser serves [`render_preview`] and [`render_benchmark`] on targets without a
//! native backend.
// Frontière interne : services qu'un backend fournit (voir backend.rs).
mod backend;
pub mod headless;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
pub(crate) mod offscreen;
// Filtres CPU, portées de la scène et géométrie communs aux renderers macOS et Linux.
#[cfg(any(target_os = "macos", target_os = "linux"))]
mod shared;
#[cfg(target_os = "windows")]
mod windows;

pub use crate::diagnostics::{render_benchmark, render_preview, write_png};
pub(crate) use backend::{Backend, Offscreen};

// Unique sélection du backend natif : toutes les entrées publiques passent par `Native`.
#[cfg(target_os = "windows")]
pub(crate) type Native = windows::Windows;
#[cfg(target_os = "macos")]
pub(crate) type Native = macos::MacOs;
#[cfg(target_os = "linux")]
pub(crate) type Native = linux::Linux;
#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
pub(crate) type Native = headless::Headless;
mod entry;
pub use entry::*;
