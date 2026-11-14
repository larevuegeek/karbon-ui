//! Icône d'une fenêtre dans la zone de notification du système (barre système) : image,
//! infobulle et menu, posée ou retirée par [`WindowCommand::Tray`](super::WindowCommand),
//! clics remis à [`Application::tray_event`](crate::application::Application::tray_event).
//! Guide complet : `docs/windows.md`, section « Icône de la fenêtre et zone de
//! notification » ; exemple : `cargo run --example tray_icon`.

/// Icon of a window in the system tray, set, updated or removed with
/// [`WindowCommand::Tray`](super::WindowCommand::Tray): the notification area on
/// Windows, the menu bar on macOS, a `StatusNotifierItem` on Linux (KDE Plasma, Xfce,
/// Cinnamon, MATE, LXQt, Budgie; GNOME with the AppIndicator extension).
///
/// Clicks and menu choices are delivered to that window's
/// [`Application::tray_event`](crate::application::Application::tray_event), even while
/// the window is hidden, with the same rules on every platform: a primary click sends
/// [`TrayEvent::Activated`], a secondary click (or Control-click on macOS) opens the
/// menu, and choosing an entry sends [`TrayEvent::MenuItem`] with its identifier. To
/// change the tooltip or a label, send the command again with the new icon: the icon is
/// updated in place.
///
/// ```
/// use karbon_ui::window::{TrayIcon, TrayMenuItem, WindowCommand, WindowId};
///
/// # let png = vec![0_u8; 8];
/// const SHOW: u32 = 1;
/// const QUIT: u32 = 2;
/// let tray = TrayIcon::new(png)
///     .tooltip("Player")
///     .item(SHOW, "Show")
///     .separator()
///     .item(QUIT, "Quit");
/// assert_eq!(tray.menu[1], TrayMenuItem::Separator);
/// // From `Application::take_window_commands`:
/// let command = WindowCommand::Tray(WindowId::MAIN, Some(tray));
/// # let _ = command;
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TrayIcon {
    /// Encoded image (PNG, JPEG, BMP…), decoded and scaled to the sizes the system uses
    /// (16 to 32 pixels on Windows and Linux, 18 points on macOS). A square image of at
    /// least 64 pixels with a transparent background stays sharp at every scale.
    pub image: Vec<u8>,
    /// Text shown when the pointer rests on the icon (Windows keeps 127 characters); on
    /// Linux it is also the item's title. Empty: no tooltip (Linux: the application name).
    pub tooltip: String,
    /// Menu of the icon, in order; no menu when empty.
    pub menu: Vec<TrayMenuItem>,
    /// The image is a single-colour shape on a transparent background: macOS draws only
    /// its alpha, tinted like the other menu bar icons (dark or light menu bar). Ignored
    /// on Windows and Linux, which show the image as it is.
    pub template: bool,
}

impl TrayIcon {
    /// An icon showing `image`, without tooltip or menu.
    pub fn new(image: impl Into<Vec<u8>>) -> Self {
        Self {
            image: image.into(),
            ..Self::default()
        }
    }
    /// Set the text shown when the pointer rests on the icon.
    pub fn tooltip(mut self, text: impl Into<String>) -> Self {
        self.tooltip = text.into();
        self
    }
    /// Add a menu entry; choosing it sends [`TrayEvent::MenuItem`] with `id`.
    pub fn item(mut self, id: u32, label: impl Into<String>) -> Self {
        self.menu.push(TrayMenuItem::Item {
            id,
            label: label.into(),
            enabled: true,
        });
        self
    }
    /// Add a greyed-out menu entry that cannot be chosen.
    pub fn disabled_item(mut self, id: u32, label: impl Into<String>) -> Self {
        self.menu.push(TrayMenuItem::Item {
            id,
            label: label.into(),
            enabled: false,
        });
        self
    }
    /// Add a separator line to the menu.
    pub fn separator(mut self) -> Self {
        self.menu.push(TrayMenuItem::Separator);
        self
    }
    /// Mark the image as a single-colour shape that macOS tints to match the menu bar
    /// (see the [`template`](Self#structfield.template) field).
    pub fn template(mut self, template: bool) -> Self {
        self.template = template;
        self
    }
}

/// Entry of a [`TrayIcon`] menu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TrayMenuItem {
    /// A command the user can choose.
    Item {
        /// Identifier sent back in [`TrayEvent::MenuItem`].
        id: u32,
        /// Text of the entry.
        label: String,
        /// `false` shows the entry greyed out and ignores it.
        enabled: bool,
    },
    /// A separator line.
    Separator,
}

/// What the user did with a window's [`TrayIcon`], delivered to
/// [`Application::tray_event`](crate::application::Application::tray_event).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrayEvent {
    /// The icon was clicked with the primary button, or chosen with the keyboard (Windows:
    /// Enter on the selected icon). Typically: show the window with
    /// [`WindowChange::Focus`](super::WindowChange::Focus).
    Activated,
    /// The menu entry with this identifier was chosen.
    MenuItem(u32),
}
