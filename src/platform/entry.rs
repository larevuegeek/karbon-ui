use super::*;
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
