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

/// Open `windows` and run the event loop until `exit_policy` is met.
///
/// A panic in application code closes its window and is returned as an error.
///
/// # Errors
///
/// - [`Error::Unsupported`](crate::Error::Unsupported) on targets without an interactive
///   backend (every target except Windows, macOS and Linux today), and on Linux without a
///   Wayland compositor (`WAYLAND_DISPLAY`) nor an X server (`DISPLAY`);
/// - [`Error::InvalidInput`](crate::Error::InvalidInput) when `windows` is empty or the
///   policy's main window is not among them;
/// - [`Error::DuplicateWindowId`](crate::Error::DuplicateWindowId) when two windows share
///   an ID, including a window opened later with
///   [`WindowCommand::Open`](crate::window::WindowCommand::Open);
/// - [`Error::InvalidWindowOptions`](crate::Error::InvalidWindowOptions) when a window's
///   options are inconsistent;
/// - [`Error::AlreadyRunning`](crate::Error::AlreadyRunning) when an event loop already
///   runs on this thread;
/// - any error returned by
///   [`Application::load`](crate::application::Application::load);
/// - [`Error::Panicked`](crate::Error::Panicked) when application code panics;
/// - [`Error::Platform`](crate::Error::Platform) or
///   [`Error::DeviceLost`](crate::Error::DeviceLost) when a native call fails without
///   recovery.
pub fn run_windows(
    windows: Vec<crate::window::WindowDescriptor>,
    exit_policy: crate::window::ExitPolicy,
) -> crate::Result<()> {
    Native::run_windows(windows, exit_policy)
}

/// Open a single window running `application` and return when it closes.
///
/// Shorthand for [`run_windows`] with one window of ID
/// [`WindowId::MAIN`](crate::window::WindowId::MAIN) and
/// [`ExitPolicy::LastWindowClosed`](crate::window::ExitPolicy::LastWindowClosed).
///
/// # Errors
///
/// See [`run_windows`].
pub fn run(
    options: crate::window::WindowOptions,
    application: impl crate::application::Application,
) -> crate::Result<()> {
    run_windows(
        vec![crate::window::WindowDescriptor::new(
            crate::window::WindowId::MAIN,
            options,
            application,
        )],
        crate::window::ExitPolicy::LastWindowClosed,
    )
}
