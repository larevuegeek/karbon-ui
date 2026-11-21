//! Ce que montre une barre de menus : ses menus, leurs entrées, sous-menus, cases et
//! raccourcis. Décrits à chaque vue par l'application, comme le reste de l'interface.
use std::sync::Arc;

/// Mark shown before the label of an entry.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Check {
    /// No mark column content.
    #[default]
    None,
    /// A check mark when `true`: an option turned on or off (« Grille »).
    Checked(bool),
    /// A dot when `true`: one choice among the entries next to it (« Perspective »,
    /// « Dessus »).
    Radio(bool),
}

/// An entry of a [`Menu`]: a command, a submenu or a separator.
pub struct MenuEntry<M> {
    pub(super) label: Arc<str>,
    pub(super) message: Option<M>,
    pub(super) shortcut: Arc<str>,
    pub(super) enabled: bool,
    pub(super) check: Check,
    pub(super) children: Vec<MenuEntry<M>>,
    pub(super) separator: bool,
}

impl<M> MenuEntry<M> {
    /// A command labelled `label` that sends `message` when chosen.
    pub fn new(label: impl Into<Arc<str>>, message: M) -> Self {
        Self {
            message: Some(message),
            ..Self::empty(label.into())
        }
    }

    /// An entry that opens `entries` on its right, when the pointer rests on it or when
    /// it is chosen.
    pub fn submenu(label: impl Into<Arc<str>>, entries: Vec<MenuEntry<M>>) -> Self {
        Self {
            children: entries,
            ..Self::empty(label.into())
        }
    }

    /// A line between groups of entries.
    pub fn separator() -> Self {
        Self {
            separator: true,
            ..Self::empty("".into())
        }
    }

    fn empty(label: Arc<str>) -> Self {
        Self {
            label,
            message: None,
            shortcut: "".into(),
            enabled: true,
            check: Check::None,
            children: Vec::new(),
            separator: false,
        }
    }

    /// Show the shortcut of the command on the right (« Ctrl+S »). Display only: the
    /// application handles the keys in `Application::key`.
    pub fn shortcut(mut self, shortcut: impl Into<Arc<str>>) -> Self {
        self.shortcut = shortcut.into();
        self
    }

    /// Grey the entry out when `enabled` is `false`: it cannot be chosen.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Show a check mark when `checked` (an option turned on or off).
    pub fn checked(mut self, checked: bool) -> Self {
        self.check = Check::Checked(checked);
        self
    }

    /// Show a dot when `selected` (one choice among several).
    pub fn radio(mut self, selected: bool) -> Self {
        self.check = Check::Radio(selected);
        self
    }
}

/// A menu of a [`MenuBar`](super::MenuBar): « Fichier », « Édition », « Affichage ».
/// The letter after `&` in the title opens it with Alt (« &Fichier » : Alt+F); the `&`
/// is not shown.
pub struct Menu<M> {
    pub(super) title: Arc<str>,
    pub(super) mnemonic: Option<char>,
    pub(super) entries: Vec<MenuEntry<M>>,
}

impl<M> Menu<M> {
    /// The menu titled `title`, empty.
    pub fn new(title: &str) -> Self {
        let mnemonic = title
            .split_once('&')
            .and_then(|(_, after)| after.chars().next())
            .map(|c| c.to_ascii_uppercase());
        Self {
            title: title.replacen('&', "", 1).into(),
            mnemonic,
            entries: Vec::new(),
        }
    }

    /// Add an entry.
    pub fn entry(mut self, entry: MenuEntry<M>) -> Self {
        self.entries.push(entry);
        self
    }

    /// Add a separator.
    pub fn separator(self) -> Self {
        self.entry(MenuEntry::separator())
    }

    /// The letter that opens the menu with Alt, in capitals.
    pub fn mnemonic(&self) -> Option<char> {
        self.mnemonic
    }
}
