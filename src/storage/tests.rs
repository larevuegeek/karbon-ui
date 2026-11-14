//! Magasin sur disque : relecture, écriture atomique, fichier abîmé, limites, et lecture
//! d'un fichier écrit par `serde_json` (préférences existantes d'une application).
use super::{LocalStorage, MAX_KEY_BYTES, json};
use std::path::PathBuf;

/// Dossier temporaire propre au test, supprimé à la fin.
struct Folder(PathBuf);
impl Folder {
    fn new(name: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("karbon-ui-storage-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for Folder {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn values_survive_a_restart() {
    let folder = Folder::new("restart");
    let file = folder.0.join("nested").join("local.json");
    let store = LocalStorage::open(&file);
    store.set("sort", "title").unwrap();
    store
        .set("quote", "un \"guillemet\"\nune ligne, é, 🎵")
        .unwrap();
    store.set("gone", "x").unwrap();
    store.remove("gone").unwrap();
    let again = LocalStorage::open(&file);
    assert_eq!(again.get("sort").as_deref(), Some("title"));
    assert_eq!(
        again.get("quote").as_deref(),
        Some("un \"guillemet\"\nune ligne, é, 🎵")
    );
    assert_eq!(again.keys(), ["quote", "sort"]);
    // Aucun fichier temporaire ne reste à côté.
    let names: Vec<_> = std::fs::read_dir(file.parent().unwrap())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(names, ["local.json"]);
}

#[test]
fn a_damaged_file_is_set_aside() {
    let folder = Folder::new("damaged");
    let file = folder.0.join("local.json");
    std::fs::write(&file, "{\"sort\": \"tit").unwrap();
    let store = LocalStorage::open(&file);
    assert_eq!(store.get("sort"), None);
    assert!(folder.0.join("local.json.corrupt").exists());
    store.set("sort", "year").unwrap();
    assert_eq!(
        LocalStorage::open(&file).get("sort").as_deref(),
        Some("year")
    );
}

#[test]
fn keys_and_quota_are_bounded() {
    let store = LocalStorage::in_memory();
    assert!(store.set("", "x").is_err());
    assert!(store.set(&"k".repeat(MAX_KEY_BYTES + 1), "x").is_err());
    assert!(store.set("big", &"x".repeat(6 << 20)).is_err());
    assert_eq!(store.get("big"), None);
    store.set(&"k".repeat(MAX_KEY_BYTES), "ok").unwrap();
}

#[test]
fn reads_what_serde_json_writes() {
    // `serde_json::to_string_pretty` d'une table de chaînes, échappements compris.
    let text = "{\n  \"a\": \"1\",\n  \"b\\\"\": \"\\u00e9\\ud83c\\udfb5\\t\\/\"\n}";
    let map = json::parse(text).unwrap();
    assert_eq!(map["a"], "1");
    assert_eq!(map["b\""], "é🎵\t/");
    assert_eq!(json::parse(&json::write(&map)).unwrap(), map);
    assert_eq!(json::parse("{}").unwrap().len(), 0);
    assert!(json::parse("{\"a\": 1}").is_none());
    assert!(json::parse("[]").is_none());
}

#[cfg(unix)]
#[test]
fn the_file_is_private_to_its_user() {
    use std::os::unix::fs::PermissionsExt;
    let folder = Folder::new("private");
    let file = folder.0.join("local.json");
    LocalStorage::open(&file).set("k", "v").unwrap();
    let mode = std::fs::metadata(&file).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o600);
}
