use super::*;
/// Window identity chosen by the application, independent of the native handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowId(pub u64);
impl WindowId {
    /// Identifier of the window opened by [`platform::run`](crate::platform::run).
    pub const MAIN: Self = Self(1);
}

/// A window to open: it owns its own application and UI state.
pub struct WindowDescriptor {
    /// Identity of the window; must be unique among open windows.
    pub id: WindowId,
    /// Initial window configuration.
    pub options: WindowOptions,
    // Lu uniquement par un backend interactif ; sans lui (cibles non Windows), le
    // backend headless refuse d'ouvrir des fenêtres et ne consomme pas l'application.
    #[cfg_attr(
        not(any(target_os = "windows", target_os = "macos")),
        allow(
            dead_code,
            reason = "seul un backend interactif ouvre les fenêtres décrites"
        )
    )]
    pub(crate) application: Box<dyn crate::application::DynApplication>,
}
impl WindowDescriptor {
    /// Describe a window running `application`. Each window has its own application,
    /// so windows of one process may use different `Message` types.
    pub fn new(
        id: WindowId,
        options: WindowOptions,
        application: impl crate::application::Application,
    ) -> Self {
        Self {
            id,
            options,
            application: Box::new(application),
        }
    }
}

/// Deferred window command returned by
/// [`Application::take_window_commands`](crate::application::Application::take_window_commands).
///
/// Commands are executed after the application callback returns: no native operation
/// runs during an application callback.
pub enum WindowCommand {
    /// Open a new window.
    Open(WindowDescriptor),
    /// Native action requested by a custom title bar.
    Action(WindowId, WindowAction),
    /// Change the size, position or state of an open window. Changing a window that is
    /// already closed does nothing.
    Change(WindowId, WindowChange),
    /// Close a window. Closing a window that is already closed does nothing.
    Close(WindowId),
    /// Close every window and leave the event loop.
    Exit,
    /// Show a native file chooser, modal to the window. The choice is delivered to that
    /// window's [`Application::task_completed`](crate::application::Application::task_completed)
    /// as a [`TaskResult`](crate::task::TaskResult) with this identifier, carrying a
    /// [`FileDialogResult`] (no path when the user cancels), or a failure when the system
    /// dialog cannot be shown. Nothing is delivered if the window is closed.
    FileDialog(WindowId, crate::task::TaskId, FileDialog),
}

/// When the event loop started by [`platform::run_windows`](crate::platform::run_windows)
/// returns.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ExitPolicy {
    /// Exit once every window is closed (the default).
    #[default]
    LastWindowClosed,
    /// Closing this window closes all the others and exits.
    MainWindowClosed(WindowId),
}

/// Initial configuration of a window.
///
/// Start from [`WindowOptions::DEFAULT`] and override fields with struct update syntax.
/// Positions and sizes are in logical pixels (96 DPI), scaled by the system DPI.
/// Opening a window fails with [`Error::InvalidWindowOptions`](crate::Error::InvalidWindowOptions)
/// when a size is not positive, the title contains a NUL character, or `click_through` is
/// set without `transparent` or with [`PresentMode::Composition`].
#[derive(Clone, Copy)]
pub struct WindowOptions {
    /// Window title.
    pub title: &'static str,
    /// Initial horizontal position of the window (ignored under Wayland, where the
    /// compositor places windows).
    pub x: i32,
    /// Initial vertical position of the window (ignored under Wayland).
    pub y: i32,
    /// Initial width; must be positive.
    pub width: i32,
    /// Initial height; must be positive.
    pub height: i32,
    /// Minimum width the user can resize to; must be positive.
    pub min_width: i32,
    /// Minimum height the user can resize to; must be positive.
    pub min_height: i32,
    /// Rendering backend to use.
    pub renderer: RenderMode,
    /// How rendered frames reach the screen; see [`PresentMode`].
    pub presentation: PresentMode,
    /// Use a per-pixel transparent background instead of the theme's opaque one.
    pub transparent: bool,
    /// Let pointer input pass through the window; requires `transparent`.
    pub click_through: bool,
    /// Activate the window when shown and list it in the taskbar; `false` creates a
    /// non-activating tool window (not available under Wayland).
    pub activate: bool,
    /// Keep the window above non-topmost windows (not available under Wayland).
    pub topmost: bool,
    /// Use the light theme instead of the dark one.
    pub light: bool,
    /// Record bounded measurements of native painting; reported on F8 and on close.
    pub profile: bool,
    /// Disable animations and transitions, as for the system "reduce motion" setting.
    pub reduced_motion: bool,
    /// Draw the built-in title bar. `false`: the whole client area belongs to the
    /// application (close with Alt+F4).
    pub title_bar: bool,
}
impl WindowOptions {
    /// Default options: a 1040x800 activated window with the dark theme, the built-in
    /// title bar and automatic renderer selection.
    pub const DEFAULT: Self = Self {
        title: "Native UI - J2",
        x: 180,
        y: 120,
        width: 1040,
        height: 800,
        min_width: 760,
        min_height: 760,
        renderer: RenderMode::Auto,
        presentation: PresentMode::Auto,
        transparent: false,
        click_through: false,
        activate: true,
        topmost: false,
        light: false,
        profile: false,
        reduced_motion: false,
        title_bar: true,
    };
}

/// Rendering backend of a window.
///
/// On macOS, [`Auto`](Self::Auto) (the default) and [`Gpu`](Self::Gpu) draw windows
/// with Metal whenever a Metal device exists, and fall back to Core Graphics when there
/// is none, when the GPU reports an error (the window then stays on Core Graphics) or
/// for a frame without an available drawable; [`Software`](Self::Software) always uses
/// Core Graphics. Offscreen rendering (previews, PNG export, benchmarks) always uses
/// Core Graphics on macOS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderMode {
    /// Use the GPU when available and fall back to software rendering.
    Auto,
    /// Force GPU rendering.
    Gpu,
    /// Force software (CPU) rendering.
    Software,
}

/// On-screen presentation path of a native window.
///
/// Offscreen rendering (benchmarks, previews, PNG export) always reads the
/// pixels back to the CPU and is not affected by this setting.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PresentMode {
    /// DirectComposition swap chain, with the layered path as a fallback when
    /// composition cannot be set up or when `click_through` is requested.
    #[default]
    Auto,
    /// DirectComposition swap chain without CPU readback. Opening the window
    /// fails if composition is unavailable; incompatible with `click_through`.
    Composition,
    /// Legacy path: GPU → CPU readback, then `UpdateLayeredWindow`.
    Layered,
}

impl WindowOptions {
    /// Build options from command-line flags, starting from [`DEFAULT`](Self::DEFAULT).
    ///
    /// Accepted flags: `--renderer=auto|gpu|software`,
    /// `--present=auto|composition|layered`, `--transparent`,
    /// `--click-through`, `--no-activate`, `--topmost`, `--light`, `--profile` and
    /// `--reduced-motion`. Pass the arguments without the program name, e.g.
    /// `std::env::args().skip(1)`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidWindowOptions`](crate::Error::InvalidWindowOptions) for an unknown
    /// flag, or for `--click-through` without `--transparent` or with
    /// `--present=composition`.
    pub fn from_args(args: impl IntoIterator<Item = String>) -> crate::Result<Self> {
        let mut options = Self::DEFAULT;
        for arg in args {
            match arg.as_str() {
                "--renderer=auto" => options.renderer = RenderMode::Auto,
                "--renderer=gpu" => options.renderer = RenderMode::Gpu,
                "--renderer=software" => options.renderer = RenderMode::Software,
                "--present=auto" => options.presentation = PresentMode::Auto,
                "--present=composition" => options.presentation = PresentMode::Composition,
                "--present=layered" => options.presentation = PresentMode::Layered,
                "--transparent" => options.transparent = true,
                "--click-through" => options.click_through = true,
                "--no-activate" => options.activate = false,
                "--topmost" => options.topmost = true,
                "--light" => options.light = true,
                "--profile" => options.profile = true,
                "--reduced-motion" => options.reduced_motion = true,
                _ => {
                    return Err(crate::Error::invalid_window_options(format!(
                        "unknown flag `{arg}`"
                    )));
                }
            }
        }
        if options.click_through && !options.transparent {
            return Err(crate::Error::invalid_window_options(
                "--click-through requires --transparent",
            ));
        }
        if options.click_through && options.presentation == PresentMode::Composition {
            return Err(crate::Error::invalid_window_options(
                "--click-through requires the layered presentation and is incompatible with --present=composition",
            ));
        }
        Ok(options)
    }
}
/// Edge or corner of a window, for the
/// [`resize_region`](crate::ui::element::Element::resize_region) of a frameless window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResizeEdge {
    /// Left edge.
    Left,
    /// Right edge.
    Right,
    /// Top edge.
    Top,
    /// Bottom edge.
    Bottom,
    /// Top-left corner.
    TopLeft,
    /// Top-right corner.
    TopRight,
    /// Bottom-left corner.
    BottomLeft,
    /// Bottom-right corner.
    BottomRight,
}

/// Native window action, typically triggered by a custom title bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowAction {
    /// Minimize the window.
    Minimize,
    /// Maximize the window, or restore it if it is already maximized.
    ToggleMaximize,
    /// Close the window.
    Close,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::{Application, Message, ViewContext},
        ui::element::Element,
    };
    #[test]
    fn windows_of_one_process_may_use_different_message_types() {
        struct Editor;
        impl Application for Editor {
            type Message = String;
            fn view(&self, _: &ViewContext) -> Element {
                Element::column(1, Vec::new())
            }
            fn update(&mut self, _: String) -> bool {
                false
            }
        }
        struct Inspector;
        impl Application for Inspector {
            type Message = u64;
            fn view(&self, _: &ViewContext) -> Element {
                Element::column(1, Vec::new())
            }
            fn update(&mut self, _: u64) -> bool {
                true
            }
        }
        let mut windows = [
            WindowDescriptor::new(WindowId::MAIN, WindowOptions::DEFAULT, Editor),
            WindowDescriptor::new(WindowId(2), WindowOptions::DEFAULT, Inspector),
        ];
        // L'adaptateur de chaque fenêtre convertit vers son propre type de message.
        assert!(!windows[0].application.update(Message::new(String::new())));
        assert!(windows[1].application.update(Message::new(7_u64)));
    }
    #[test]
    fn profiling_is_disabled_by_default_and_can_be_requested_explicitly() {
        assert!(
            !WindowOptions::from_args(Vec::<String>::new())
                .unwrap()
                .profile
        );
        assert!(
            WindowOptions::from_args(["--profile".into()])
                .unwrap()
                .profile
        );
    }
    #[test]
    fn presentation_defaults_to_auto_and_can_be_selected_explicitly() {
        let parse = |args: &[&str]| {
            WindowOptions::from_args(args.iter().map(|arg| arg.to_string()))
                .map(|options| options.presentation)
        };
        assert_eq!(parse(&[]).unwrap(), PresentMode::Auto);
        assert_eq!(parse(&["--present=layered"]).unwrap(), PresentMode::Layered);
        assert_eq!(
            parse(&["--present=composition"]).unwrap(),
            PresentMode::Composition
        );
        assert_eq!(parse(&["--present=auto"]).unwrap(), PresentMode::Auto);
        assert!(matches!(
            parse(&["--present=dib"]),
            Err(crate::Error::InvalidWindowOptions { reason }) if reason.contains("--present=dib")
        ));
        // Le clic traversant exige WS_EX_LAYERED : la composition explicite est refusée.
        let click_through = ["--transparent", "--click-through"];
        assert_eq!(parse(&click_through).unwrap(), PresentMode::Auto);
        assert!(matches!(
            parse(&[click_through[0], click_through[1], "--present=composition"]),
            Err(crate::Error::InvalidWindowOptions { .. })
        ));
        assert!(matches!(
            parse(&[click_through[1]]),
            Err(crate::Error::InvalidWindowOptions { .. })
        ));
    }
}
