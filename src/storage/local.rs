//! Magasin clé → valeur d'une application, gardé dans un fichier JSON : écriture atomique
//! (fichier temporaire puis renommage), fichier réservé à l'utilisateur, tailles bornées,
//! fichier abîmé mis de côté.
use super::json;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// Longest key, in bytes.
pub const MAX_KEY_BYTES: usize = 256;
/// Total size of the keys and values of a store, in bytes (like browsers' 5 MiB).
pub const MAX_TOTAL_BYTES: usize = 5 << 20;

/// Persistent key → string store of an application, like the web's `localStorage`:
/// small preferences of the interface (a sort order, a dismissed banner, a window
/// position), read at once and kept in memory, written to a JSON file on each change.
///
/// The file is written atomically (a temporary file renamed over it: an interrupted
/// write never leaves a half-written file), readable by the user only on macOS and
/// Linux (Windows profiles are private to their user already). A file that cannot be
/// read as a store is moved aside (`*.corrupt`) and the store starts empty, like a
/// browser in private mode. Values are not encrypted: keep passwords and tokens in
/// [`crate::secrets`].
///
/// ```
/// use karbon_ui::storage::LocalStorage;
///
/// let store = LocalStorage::in_memory();
/// store.set("sort", "title").unwrap();
/// assert_eq!(store.get("sort").as_deref(), Some("title"));
/// store.remove("sort").unwrap();
/// assert_eq!(store.get("sort"), None);
/// ```
pub struct LocalStorage {
    state: Mutex<State>,
}

struct State {
    path: Option<PathBuf>,
    map: BTreeMap<String, String>,
}

impl LocalStorage {
    /// Store kept in `path` (read now if it exists; created on the first change).
    pub fn open(path: impl Into<PathBuf>) -> Self {
        let store = Self::in_memory();
        store.open_file(path);
        store
    }
    /// Store kept in memory only: changes are lost when the process ends.
    pub fn in_memory() -> Self {
        Self {
            state: Mutex::new(State {
                path: None,
                map: BTreeMap::new(),
            }),
        }
    }
    /// Keeps the store in `path` from now on, replacing its content with the file's.
    pub fn open_file(&self, path: impl Into<PathBuf>) {
        let path = path.into();
        let map = read(&path);
        let mut state = self.state();
        state.map = map;
        state.path = Some(path);
    }
    /// Value of `key`, if set.
    pub fn get(&self, key: &str) -> Option<String> {
        self.state().map.get(key).cloned()
    }
    /// Sets `key` to `value` and writes the file.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidInput`](crate::Error::InvalidInput) for an empty key, a key
    /// longer than [`MAX_KEY_BYTES`] or a store that would exceed [`MAX_TOTAL_BYTES`]
    /// (nothing is changed); [`Error::Io`](crate::Error::Io) when the file cannot be
    /// written (the value is kept in memory for the rest of the session).
    pub fn set(&self, key: &str, value: &str) -> crate::Result<()> {
        if key.is_empty() || key.len() > MAX_KEY_BYTES {
            return Err(crate::Error::invalid_input(
                "storage keys are 1 to 256 bytes long",
            ));
        }
        let mut state = self.state();
        if state.map.get(key).map(String::as_str) == Some(value) {
            return Ok(());
        }
        let total: usize = state
            .map
            .iter()
            .filter(|(k, _)| k.as_str() != key)
            .map(|(k, v)| k.len() + v.len())
            .sum();
        if total + key.len() + value.len() > MAX_TOTAL_BYTES {
            return Err(crate::Error::invalid_input(
                "storage quota exceeded (5 MiB)",
            ));
        }
        state.map.insert(key.to_string(), value.to_string());
        state.save()
    }
    /// Removes `key` and writes the file; removing an absent key does nothing.
    ///
    /// # Errors
    ///
    /// [`Error::Io`](crate::Error::Io) when the file cannot be written.
    pub fn remove(&self, key: &str) -> crate::Result<()> {
        let mut state = self.state();
        if state.map.remove(key).is_none() {
            return Ok(());
        }
        state.save()
    }
    /// Removes every key and writes the file.
    ///
    /// # Errors
    ///
    /// [`Error::Io`](crate::Error::Io) when the file cannot be written.
    pub fn clear(&self) -> crate::Result<()> {
        let mut state = self.state();
        state.map.clear();
        state.save()
    }
    /// Keys of the store, sorted.
    pub fn keys(&self) -> Vec<String> {
        self.state().map.keys().cloned().collect()
    }
    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// The process-wide store, like the page's `localStorage`: in memory until
/// [`open_local`] gives it a file.
///
/// ```
/// use karbon_ui::storage;
///
/// storage::local().set("banner_dismissed", "1").unwrap();
/// assert_eq!(storage::local().get("banner_dismissed").as_deref(), Some("1"));
/// ```
pub fn local() -> &'static LocalStorage {
    static LOCAL: OnceLock<LocalStorage> = OnceLock::new();
    LOCAL.get_or_init(LocalStorage::in_memory)
}

/// Keeps the process-wide store ([`local`]) in `path`, typically
/// `<data folder>/local-storage.json`; call it once at start-up.
pub fn open_local(path: impl Into<PathBuf>) {
    local().open_file(path);
}

impl State {
    /// Écrit le fichier : texte complet dans un fichier temporaire voisin, synchronisé
    /// sur le disque, puis renommé par-dessus l'ancien.
    fn save(&self) -> crate::Result<()> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        let temporary = sibling(path, &format!("tmp-{}", std::process::id()));
        let result = write_private(&temporary, json::write(&self.map).as_bytes())
            .and_then(|()| std::fs::rename(&temporary, path));
        if result.is_err() {
            let _ = std::fs::remove_file(&temporary);
        }
        Ok(result?)
    }
}

/// `chemin.extension` → `chemin.extension.suffixe`.
fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".{suffix}"));
    path.with_file_name(name)
}

/// Contenu du fichier ; absent, vide. Illisible comme magasin : mis de côté
/// (`*.corrupt`), vide.
fn read(path: &Path) -> BTreeMap<String, String> {
    let Ok(bytes) = std::fs::read(path) else {
        return BTreeMap::new();
    };
    if let Some(map) = std::str::from_utf8(&bytes).ok().and_then(json::parse) {
        return map;
    }
    let _ = std::fs::rename(path, sibling(path, "corrupt"));
    BTreeMap::new()
}

/// Fichier créé lisible par son propriétaire seul (Unix), écrit et synchronisé.
fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}
