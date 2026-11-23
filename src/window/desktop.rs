//! Le bureau autour des fenêtres : la liste des écrans (KUI-025), une boîte de message
//! du système (KUI-027) et la place d'une fenêtre à retenir d'une séance à l'autre
//! (KUI-028). Sous Windows ; ailleurs, les deux premières rendent
//! `Error::Unsupported`.
use super::{WindowChange, WindowState};

/// A display of the desktop, in physical pixels of the virtual screen.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Monitor {
    /// The system's name of the display (`\\.\DISPLAY1`).
    pub name: String,
    /// Left edge of the display.
    pub x: i32,
    /// Top edge of the display.
    pub y: i32,
    /// Width of the display.
    pub width: i32,
    /// Height of the display.
    pub height: i32,
    /// The part not covered by the taskbar: `(x, y, width, height)`.
    pub work_area: (i32, i32, i32, i32),
    /// Physical pixels per logical pixel (`1.5` at 150 %).
    pub scale: f32,
    /// The main display, where windows open by default.
    pub primary: bool,
}

/// The displays of the desktop, the main one first. Windows only for now.
pub fn monitors() -> crate::Result<Vec<Monitor>> {
    #[cfg(target_os = "windows")]
    {
        crate::platform::desktop::monitors()
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err(crate::Error::Unsupported {
            reason: "the list of displays is only implemented on Windows",
        })
    }
}

/// The buttons of a [`message_box`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum MessageButtons {
    /// « OK ».
    Ok,
    /// « OK » and « Annuler ».
    OkCancel,
    /// « Oui » and « Non ».
    YesNo,
    /// « Oui », « Non » and « Annuler ».
    YesNoCancel,
}

/// The icon of a [`message_box`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum MessageIcon {
    /// Information.
    Information,
    /// A warning.
    Warning,
    /// An error.
    Error,
    /// A question.
    Question,
}

/// The button the user chose in a [`message_box`]; `Cancel` also when the box is closed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum MessageAnswer {
    /// « OK ».
    Ok,
    /// « Annuler », or the box closed.
    Cancel,
    /// « Oui ».
    Yes,
    /// « Non ».
    No,
}

/// Show a message box of the system with `title`, `text`, `icon` and `buttons`, in the
/// language of the system, and wait for the answer. It stays above the windows of the
/// application. The call blocks until the user answers: make it from a background
/// thread or a task and send the answer back through an
/// [`Inbox`](crate::task::Inbox), never from inside an application callback. Meant for
/// what the interface itself cannot show (a crash report before any window opens, an
/// error at startup). Windows only for now.
///
/// ```no_run
/// use karbon_ui::window::{MessageAnswer, MessageButtons, MessageIcon, message_box};
///
/// let answer = message_box("Éditeur", "Reprendre le travail non enregistré ?", MessageIcon::Question, MessageButtons::YesNo)?;
/// if answer == MessageAnswer::Yes { /* … */ }
/// # Ok::<(), karbon_ui::Error>(())
/// ```
pub fn message_box(
    title: &str,
    text: &str,
    icon: MessageIcon,
    buttons: MessageButtons,
) -> crate::Result<MessageAnswer> {
    if title.contains('\0') || text.contains('\0') {
        return Err(crate::Error::invalid_input("message box text contains NUL"));
    }
    #[cfg(target_os = "windows")]
    {
        crate::platform::desktop::message_box(title, text, icon, buttons)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (icon, buttons);
        Err(crate::Error::Unsupported {
            reason: "message boxes are only implemented on Windows",
        })
    }
}

/// Where a window was and how large, to put it back at the next launch: its position
/// and size (logical pixels, as [`WindowState`] gives them) and whether it was
/// maximized. Kept as a line of text (« 120 80 1600 900 maximized »).
///
/// ```
/// use karbon_ui::window::{Placement, WindowChange};
///
/// let placement = Placement::parse("120 80 1600 900 maximized").unwrap();
/// assert_eq!(placement.to_text(), "120 80 1600 900 maximized");
/// // At launch: the place, the size, back on a display if that one was unplugged,
/// // then maximized.
/// let changes = placement.changes();
/// assert_eq!(changes.last(), Some(&WindowChange::Maximized(true)));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placement {
    /// Horizontal position of the outer top-left corner.
    pub x: i32,
    /// Vertical position of the outer top-left corner.
    pub y: i32,
    /// Client width, logical pixels.
    pub width: i32,
    /// Client height, logical pixels.
    pub height: i32,
    /// The window was maximized.
    pub maximized: bool,
}

impl Placement {
    /// The placement to remember after `state`: a maximized or minimized window keeps
    /// the place it had before (`previous`), with the maximized mark of `state`.
    /// `None` while the user is still moving it, or with no place known.
    pub fn after(state: &WindowState, previous: Option<Placement>) -> Option<Self> {
        if state.dragging || state.fullscreen {
            return previous;
        }
        if state.maximized || state.minimized {
            return previous.map(|p| Self {
                maximized: state.maximized || p.maximized && state.minimized,
                ..p
            });
        }
        Some(Self {
            x: state.x,
            y: state.y,
            width: state.width,
            height: state.height,
            maximized: false,
        })
    }

    /// The line of text that keeps it.
    pub fn to_text(&self) -> String {
        let mut text = format!("{} {} {} {}", self.x, self.y, self.width, self.height);
        if self.maximized {
            text.push_str(" maximized");
        }
        text
    }

    /// The placement written by [`to_text`](Self::to_text); `None` for an unreadable
    /// line or a size under 100 × 80.
    pub fn parse(text: &str) -> Option<Self> {
        let mut words = text.split_whitespace();
        let mut number = || words.next()?.parse::<i32>().ok();
        let (x, y, width, height) = (number()?, number()?, number()?, number()?);
        let maximized = match words.next() {
            None => false,
            Some("maximized") => true,
            Some(_) => return None,
        };
        (width >= 100 && height >= 80).then_some(Self {
            x,
            y,
            width,
            height,
            maximized,
        })
    }

    /// The changes that put a window back there: position, size, back on a display if
    /// that one is gone, then maximized if it was.
    pub fn changes(&self) -> Vec<WindowChange> {
        let mut changes = vec![
            WindowChange::Position {
                x: self.x,
                y: self.y,
            },
            WindowChange::Size {
                width: self.width,
                height: self.height,
            },
            WindowChange::KeepOnScreen,
        ];
        if self.maximized {
            changes.push(WindowChange::Maximized(true));
        }
        changes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(x: i32, maximized: bool, dragging: bool) -> WindowState {
        WindowState {
            x,
            y: 40,
            width: 1280,
            height: 720,
            maximized,
            dragging,
            ..WindowState::default()
        }
    }

    #[test]
    fn a_placement_keeps_the_place_before_maximizing_and_reads_back() {
        let normal = Placement::after(&state(100, false, false), None).unwrap();
        assert_eq!(normal.to_text(), "100 40 1280 720");
        // Agrandie : la place d'avant est gardée, avec la marque.
        let maximized = Placement::after(&state(0, true, false), Some(normal)).unwrap();
        assert_eq!(maximized.to_text(), "100 40 1280 720 maximized");
        // Pendant un glisser : rien de neuf.
        assert_eq!(
            Placement::after(&state(500, false, true), Some(normal)),
            Some(normal)
        );
        assert_eq!(Placement::parse(&maximized.to_text()), Some(maximized));
        // Témoins : une ligne illisible, une fenêtre minuscule.
        assert_eq!(Placement::parse("100 40 large 720"), None);
        assert_eq!(Placement::parse("1 2 30 40"), None);
        assert_eq!(Placement::parse("1 2 300 400 tordue"), None);
        // Les changements qui la rendent : place, taille, sur un écran, agrandie.
        assert_eq!(maximized.changes().len(), 4);
        assert_eq!(normal.changes()[2], WindowChange::KeepOnScreen);
    }
}
