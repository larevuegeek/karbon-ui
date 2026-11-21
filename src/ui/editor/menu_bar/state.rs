//! État d'une barre de menus : le menu ouvert, le sous-menu ouvert, et le clavier (F10,
//! Alt+lettre, gauche et droite pour passer d'un menu à l'autre).
use super::model::Menu;
use crate::ui::runtime::{KeyCode, KeyPress};

/// A gesture on a [`MenuBar`](super::MenuBar), to pass to [`MenuBarState::update`]. It
/// carries the application's message of a chosen entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuBarMsg<M>(pub(super) Inner<M>);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Inner<M> {
    /// Une commande est choisie, dans le menu ou dans un sous-menu.
    Choose(Box<M>),
    /// Clic sur le titre du menu `index`.
    Title(usize),
    /// Le pointeur entre sur le titre du menu `index`.
    HoverTitle(usize),
    /// Le pointeur entre sur l'entrée `index` du menu ouvert ; `true` si elle ouvre un
    /// sous-menu.
    HoverEntry(usize, bool),
    /// L'entrée `index` du menu ouvert, qui ouvre un sous-menu, est choisie.
    OpenSubmenu(usize),
    /// Le menu doit se fermer (choix fait, Échap, clic ailleurs).
    Close,
    /// Le sous-menu doit se fermer (Échap, clic dans le menu).
    CloseSubmenu,
    /// Le pointeur quitte un titre ou une entrée : rien à faire.
    Ignore,
}

/// State of a [`MenuBar`](super::MenuBar), kept by the application: which menu and
/// which submenu are open.
///
/// ```
/// use karbon_ui::ui::editor::MenuBarState;
///
/// let mut menus = MenuBarState::default();
/// menus.open(0);
/// assert_eq!(menus.open_menu(), Some(0));
/// menus.close();
/// assert_eq!(menus.open_menu(), None);
/// ```
#[derive(Default)]
pub struct MenuBarState {
    pub(super) open: Option<usize>,
    pub(super) submenu: Option<usize>,
    /// La fermeture que le moteur envoie après le choix d'une entrée qui ouvre un
    /// sous-menu est ignorée : le menu reste ouvert.
    keep_next_close: bool,
}

/// Answer of [`MenuBarState::update`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuReply<M> {
    /// The bar changed on screen: return `true` from `Application::update`.
    pub rebuild: bool,
    /// The message of the entry chosen: apply it as if the view had sent it.
    pub command: Option<M>,
}

impl MenuBarState {
    /// Index of the open menu, if any.
    pub fn open_menu(&self) -> Option<usize> {
        self.open
    }

    /// Open the menu of `index`.
    pub fn open(&mut self, index: usize) {
        self.open = Some(index);
        self.submenu = None;
    }

    /// Close the open menu.
    pub fn close(&mut self) {
        self.open = None;
        self.submenu = None;
    }

    /// Apply a gesture of the bar. A chosen entry closes the menus and gives back its
    /// message in [`MenuReply::command`].
    pub fn update<M>(&mut self, message: MenuBarMsg<M>) -> MenuReply<M> {
        if let Inner::Choose(command) = message.0 {
            self.close();
            self.keep_next_close = false;
            return MenuReply {
                rebuild: true,
                command: Some(*command),
            };
        }
        MenuReply {
            rebuild: self.change(message),
            command: None,
        }
    }

    fn change<M>(&mut self, message: MenuBarMsg<M>) -> bool {
        match message.0 {
            Inner::Choose(_) => true,
            Inner::CloseSubmenu => self.submenu.take().is_some(),
            Inner::Title(index) => {
                if self.open == Some(index) {
                    self.close();
                } else {
                    self.open(index);
                }
                true
            }
            Inner::HoverTitle(index) => {
                let switch = self.open.is_some_and(|open| open != index);
                if switch {
                    self.open(index);
                }
                switch
            }
            Inner::HoverEntry(index, opens) => {
                let submenu = opens.then_some(index);
                let changed = self.submenu != submenu;
                self.submenu = submenu;
                changed
            }
            Inner::OpenSubmenu(index) => {
                self.submenu = Some(index);
                self.keep_next_close = true;
                true
            }
            Inner::Ignore => false,
            Inner::Close => {
                if std::mem::take(&mut self.keep_next_close) {
                    return false;
                }
                let open = self.open.is_some();
                self.close();
                open
            }
        }
    }

    /// Apply a key given to [`Application::key`](crate::application::Application::key):
    /// F10 opens the first menu (or closes the open one), Alt with the letter of a menu
    /// opens it, Left and Right go to the previous and next menus while one is open.
    /// `menus` are those of the bar. Returns `true` when the view must be rebuilt.
    pub fn key<M>(&mut self, press: KeyPress, menus: &[Menu<M>]) -> bool {
        if !press.pressed || menus.is_empty() {
            return false;
        }
        let count = menus.len();
        match press.key {
            KeyCode::F10 if !press.shift && !press.control => {
                if self.open.is_some() {
                    self.close();
                } else {
                    self.open(0);
                }
                true
            }
            KeyCode::Left | KeyCode::Right if self.open.is_some() && self.submenu.is_none() => {
                let open = self.open.unwrap_or(0);
                let next = if press.key == KeyCode::Right {
                    (open + 1) % count
                } else {
                    (open + count - 1) % count
                };
                self.open(next);
                true
            }
            key if press.alt && !press.control => {
                let Some(letter) = letter(key) else {
                    return false;
                };
                match menus.iter().position(|m| m.mnemonic == Some(letter)) {
                    Some(index) => {
                        self.open(index);
                        true
                    }
                    None => false,
                }
            }
            _ => false,
        }
    }
}

/// Lettre majuscule d'une touche de lettre.
fn letter(key: KeyCode) -> Option<char> {
    use KeyCode::*;
    const LETTERS: [KeyCode; 26] = [
        A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    ];
    LETTERS
        .iter()
        .position(|&k| k == key)
        .map(|index| char::from(b'A' + index as u8))
}
