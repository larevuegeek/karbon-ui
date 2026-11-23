//! Le registre des commandes et leurs raccourcis : ceux de départ, ceux que
//! l'utilisateur a changés (lus et écrits en clair), la touche qui mène à une commande
//! selon les contextes actifs, et les conflits.
use super::{Shortcut, ShortcutNames};
use crate::ui::runtime::KeyPress;
use std::collections::BTreeMap;

/// The context of the commands that work everywhere.
pub const GLOBAL: &str = "";

/// A command of the application: an identifier (« file.save »), a name for menus and the
/// command palette (« Enregistrer »), a description for tooltips, the context where its
/// keys work (`GLOBAL` by default: everywhere) and its default shortcuts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Command {
    /// Stable identifier, used in the saved shortcuts.
    pub id: String,
    /// Name shown in menus and in the command palette.
    pub name: String,
    /// One sentence that says what the command does (tooltips, palette).
    pub description: String,
    /// Where its keys work: a panel (« view », « tree ») or [`GLOBAL`].
    pub context: String,
    /// Shortcuts the command has before the user changes them.
    pub defaults: Vec<Shortcut>,
}

impl Command {
    /// The command `id` named `name`, global, without shortcut.
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            context: GLOBAL.into(),
            defaults: Vec::new(),
        }
    }

    /// The same command with this description.
    pub fn description(mut self, text: impl Into<String>) -> Self {
        self.description = text.into();
        self
    }

    /// The same command, whose keys work in `context` only.
    pub fn context(mut self, context: impl Into<String>) -> Self {
        self.context = context.into();
        self
    }

    /// The same command with one more default shortcut.
    pub fn shortcut(mut self, shortcut: Shortcut) -> Self {
        self.defaults.push(shortcut);
        self
    }
}

/// Two or more commands of the same context on the same shortcut: the key cannot tell
/// them apart.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conflict {
    /// The context where they meet.
    pub context: String,
    /// The shortcut they share.
    pub shortcut: Shortcut,
    /// Their identifiers, in the order of the registry.
    pub commands: Vec<String>,
}

/// The commands of an application and their shortcuts: the defaults, and those the user
/// changed. It says which command a key leads to in the active contexts, which commands
/// share a key, and the text of a shortcut for menus and tooltips; the changes are read
/// and written in clear, one line per command (« file.save = Ctrl+S, Ctrl+Shift+S »).
///
/// ```
/// use karbon_ui::ui::{commands::{Command, GLOBAL, Keymap, Shortcut, ShortcutNames}, runtime::{KeyCode, KeyPress}};
///
/// let mut keymap = Keymap::new();
/// keymap.add(Command::new("object.duplicate", "Dupliquer").shortcut(Shortcut::key(KeyCode::D).ctrl()));
/// keymap.add(Command::new("object.delete", "Supprimer").context("view").shortcut(Shortcut::key(KeyCode::Delete)));
/// // The user moves Duplicate to Ctrl+Shift+D: the text is what an application saves.
/// keymap.set_shortcuts("object.duplicate", vec![Shortcut::parse("Ctrl+Maj+D").unwrap()]);
/// let saved = keymap.changes_text(ShortcutNames::French);
/// assert_eq!(saved, "object.duplicate = Ctrl+Maj+D\n");
/// // At the next launch.
/// let mut again = Keymap::new();
/// again.add(Command::new("object.duplicate", "Dupliquer").shortcut(Shortcut::key(KeyCode::D).ctrl()));
/// assert!(again.apply_changes(&saved).is_empty());
/// let mut press = KeyPress::new(KeyCode::D, true);
/// (press.control, press.shift) = (true, true);
/// assert_eq!(again.resolve(&press, &[GLOBAL]), Some("object.duplicate"));
/// assert_eq!(again.shortcut_text("object.duplicate", ShortcutNames::French), "Ctrl+Maj+D");
/// ```
#[derive(Clone, Debug, Default)]
pub struct Keymap {
    commands: Vec<Command>,
    /// Raccourcis changés par l'utilisateur, par commande (une liste vide : aucun).
    changed: BTreeMap<String, Vec<Shortcut>>,
}

impl Keymap {
    /// An empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add `command` (a command with the same identifier is replaced).
    pub fn add(&mut self, command: Command) {
        match self.commands.iter_mut().find(|c| c.id == command.id) {
            Some(existing) => *existing = command,
            None => self.commands.push(command),
        }
    }

    /// The commands, in the order they were added.
    pub fn commands(&self) -> &[Command] {
        &self.commands
    }

    /// The command `id`.
    pub fn command(&self, id: &str) -> Option<&Command> {
        self.commands.iter().find(|c| c.id == id)
    }

    /// The shortcuts of the command `id` in force: those the user chose, or the defaults.
    pub fn shortcuts(&self, id: &str) -> &[Shortcut] {
        match self.changed.get(id) {
            Some(changed) => changed,
            None => self.command(id).map_or(&[], |c| &c.defaults),
        }
    }

    /// Give the command `id` these shortcuts instead of its defaults (an empty list
    /// removes them all). Unknown commands are ignored.
    pub fn set_shortcuts(&mut self, id: &str, shortcuts: Vec<Shortcut>) {
        let Some(command) = self.command(id) else {
            return;
        };
        if command.defaults == shortcuts {
            self.changed.remove(id);
        } else {
            self.changed.insert(id.to_owned(), shortcuts);
        }
    }

    /// Give back its default shortcuts to the command `id`.
    pub fn reset(&mut self, id: &str) {
        self.changed.remove(id);
    }

    /// Give back their default shortcuts to every command.
    pub fn reset_all(&mut self) {
        self.changed.clear();
    }

    /// Whether the user changed the shortcuts of the command `id`.
    pub fn is_changed(&self, id: &str) -> bool {
        self.changed.contains_key(id)
    }

    /// The command that `press` leads to, among the commands of `contexts` (a panel's
    /// first, then wider ones, usually ending with [`GLOBAL`]): the first context that
    /// has one wins.
    pub fn resolve(&self, press: &KeyPress, contexts: &[&str]) -> Option<&str> {
        contexts.iter().find_map(|context| {
            self.commands
                .iter()
                .filter(|c| c.context == *context)
                .find(|c| self.shortcuts(&c.id).iter().any(|s| s.matches(press)))
                .map(|c| c.id.as_str())
        })
    }

    /// The first shortcut of the command `id` in clear, for a menu or a tooltip; empty
    /// when it has none.
    pub fn shortcut_text(&self, id: &str, names: ShortcutNames) -> String {
        self.shortcuts(id)
            .first()
            .map_or_else(String::new, |s| s.text(names))
    }

    /// Every shortcut shared by two commands of the same context.
    pub fn conflicts(&self) -> Vec<Conflict> {
        let mut seen: BTreeMap<(String, String), Conflict> = BTreeMap::new();
        for command in &self.commands {
            for shortcut in self.shortcuts(&command.id) {
                let key = (command.context.clone(), format!("{shortcut:?}"));
                let entry = seen.entry(key).or_insert_with(|| Conflict {
                    context: command.context.clone(),
                    shortcut: *shortcut,
                    commands: Vec::new(),
                });
                if !entry.commands.contains(&command.id) {
                    entry.commands.push(command.id.clone());
                }
            }
        }
        seen.into_values()
            .filter(|c| c.commands.len() > 1)
            .collect()
    }

    /// The commands of `context` that already use `shortcut`, other than `except`: what a
    /// settings page says before the user confirms a new key.
    pub fn users_of(&self, shortcut: &Shortcut, context: &str, except: &str) -> Vec<&str> {
        self.commands
            .iter()
            .filter(|c| c.context == context && c.id != except)
            .filter(|c| self.shortcuts(&c.id).contains(shortcut))
            .map(|c| c.id.as_str())
            .collect()
    }

    /// The user's changes in clear, one line per command (« file.save = Ctrl+S », « none »
    /// for a command without shortcut), in the order of the identifiers: what an
    /// application saves.
    pub fn changes_text(&self, names: ShortcutNames) -> String {
        let mut text = String::new();
        for (id, shortcuts) in &self.changed {
            let keys: Vec<String> = shortcuts.iter().map(|s| s.text(names)).collect();
            let keys = if keys.is_empty() {
                "none".to_owned()
            } else {
                keys.join(", ")
            };
            text.push_str(&format!("{id} = {keys}\n"));
        }
        text
    }

    /// Apply changes written by [`changes_text`](Self::changes_text) (or by hand): one
    /// line per command, `#` for comments; « none » or « aucun » removes the shortcuts.
    /// Unreadable lines are skipped; their descriptions are returned.
    pub fn apply_changes(&mut self, text: &str) -> Vec<String> {
        let mut errors = Vec::new();
        for (number, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            match self.read_line(line) {
                Ok((id, shortcuts)) => self.set_shortcuts(&id, shortcuts),
                Err(error) => errors.push(format!("line {}: {error}", number + 1)),
            }
        }
        errors
    }

    fn read_line(&self, line: &str) -> Result<(String, Vec<Shortcut>), String> {
        let (id, keys) = line.split_once('=').ok_or("no « = »")?;
        let id = id.trim();
        if self.command(id).is_none() {
            return Err(format!("unknown command « {id} »"));
        }
        let keys = keys.trim();
        if keys.eq_ignore_ascii_case("none") || keys.eq_ignore_ascii_case("aucun") {
            return Ok((id.to_owned(), Vec::new()));
        }
        let shortcuts = keys
            .split(", ")
            .map(|k| Shortcut::parse(k).map_err(|e| e.to_string()))
            .collect::<Result<Vec<_>, _>>()?;
        Ok((id.to_owned(), shortcuts))
    }

    /// Save the user's changes in `storage` under `key`.
    pub fn save(&self, storage: &crate::storage::LocalStorage, key: &str) -> crate::Result<()> {
        storage.set(key, &self.changes_text(ShortcutNames::English))
    }

    /// Read the changes saved by [`save`](Self::save); returns the unreadable lines.
    pub fn load(&mut self, storage: &crate::storage::LocalStorage, key: &str) -> Vec<String> {
        storage
            .get(key)
            .map_or_else(Vec::new, |text| self.apply_changes(&text))
    }
}
