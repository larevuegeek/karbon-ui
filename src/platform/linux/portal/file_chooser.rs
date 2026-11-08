//! Dialogue de fichiers par le portail `org.freedesktop.portal.FileChooser` : `OpenFile`
//! (fichier, fichiers, dossier) ou `SaveFile`, puis attente du signal `Response` de la
//! requête créée par le portail.
//!
//! La requête est identifiée par un jeton (`handle_token`) : son chemin
//! (`/org/freedesktop/portal/desktop/request/<expéditeur>/<jeton>`) est connu avant
//! l'appel, et la règle du signal est posée avant lui, pour ne pas manquer une réponse
//! immédiate. Les anciens portails renvoient un autre chemin : les deux sont acceptés.
use crate::{
    platform::linux::ffi::dbus::{Connection, Message, SIGNAL, Writer},
    task::CancellationToken,
    window::{FileDialog, FileDialogKind},
};
use std::{
    os::unix::ffi::{OsStrExt, OsStringExt},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

const SERVICE: &str = "org.freedesktop.portal.Desktop";
const OBJECT: &str = "/org/freedesktop/portal/desktop";
const INTERFACE: &str = "org.freedesktop.portal.FileChooser";
const REQUEST: &str = "org.freedesktop.portal.Request";
/// Attente au plus de l'appel (le portail peut être démarré à la demande par le bus).
const CALL_TIMEOUT: i32 = 30_000;
/// Intervalle de vérification de la fermeture de la fenêtre pendant l'attente.
const POLL: i32 = 200;

/// Filtre du portail : nom et motifs glob (`*.flac`).
pub(super) type Filter = (String, Vec<String>);

/// Valeur d'une option de `OpenFile` ou `SaveFile` (dictionnaire `a{sv}`).
#[derive(Debug, PartialEq)]
pub(super) enum Value {
    Bool(bool),
    Text(String),
    /// Tableau d'octets `ay` (chemin terminé par un octet nul).
    Bytes(Vec<u8>),
    /// Liste de filtres `a(sa(us))`.
    Filters(Vec<Filter>),
    /// Filtre `(sa(us))`.
    Filter(Filter),
}

/// Montre `dialog` et attend le choix ; les chemins choisis (aucun si l'utilisateur
/// annule), ou le message d'échec. Rend la main sans choix quand `closed` est annulé
/// (fenêtre fermée) : le dialogue est alors fermé.
pub(super) fn show(
    dialog: &FileDialog,
    parent: &str,
    closed: &CancellationToken,
) -> Result<Vec<PathBuf>, String> {
    let connection =
        Connection::session().ok_or_else(|| "D-Bus session bus unavailable".to_owned())?;
    let token = handle_token();
    let expected = request_path(&connection.unique_name(), &token);
    connection.add_match(&format!(
        "type='signal',interface='{REQUEST}',member='Response'"
    ));
    let method = if dialog.kind == FileDialogKind::SaveFile {
        "SaveFile"
    } else {
        "OpenFile"
    };
    let mut call = Message::method_call(SERVICE, OBJECT, INTERFACE, method)
        .ok_or_else(|| "cannot create the D-Bus message".to_owned())?;
    {
        let mut writer = call.writer();
        // Titre vide : celui du système.
        writer
            .string(parent)
            .string(dialog.title.as_deref().unwrap_or(""));
        write_options(&mut writer, &options(dialog, &token));
    }
    let reply = connection
        .request(&call, CALL_TIMEOUT)
        .map_err(|e| format!("XDG desktop portal FileChooser unavailable: {e}"))?;
    let handle = reply.reader().string().unwrap_or_else(|| expected.clone());
    loop {
        while let Some(message) = connection.pop() {
            let path = message.path();
            if message.kind() == SIGNAL
                && message.interface() == REQUEST
                && message.member() == "Response"
                && (path == handle || path == expected)
            {
                let mut paths = response(&message)?;
                if dialog.kind != FileDialogKind::OpenFiles {
                    paths.truncate(1);
                }
                return Ok(paths);
            }
        }
        if closed.is_cancelled() {
            close(&connection, &handle);
            return Ok(Vec::new());
        }
        if !connection.wait(POLL) {
            return Err("D-Bus session bus disconnected".to_owned());
        }
    }
}
/// Ferme le dialogue de la requête `handle` (fenêtre fermée).
fn close(connection: &Connection, handle: &str) {
    if let Some(message) = Message::method_call(SERVICE, handle, REQUEST, "Close") {
        connection.send(&message);
        connection.flush();
    }
}
/// Jeton de requête unique dans le processus (lettres, chiffres et `_` seulement).
fn handle_token() -> String {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    format!(
        "karbon_{}_{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}
/// Chemin de la requête créée par le portail pour l'expéditeur `sender` (`:1.42`) et le
/// jeton `token`.
pub(super) fn request_path(sender: &str, token: &str) -> String {
    let sender = sender.trim_start_matches(':').replace('.', "_");
    format!("{OBJECT}/request/{sender}/{token}")
}
/// Options de l'appel, dans l'ordre d'écriture.
pub(super) fn options(dialog: &FileDialog, token: &str) -> Vec<(&'static str, Value)> {
    let mut options = vec![
        ("handle_token", Value::Text(token.to_owned())),
        ("modal", Value::Bool(true)),
    ];
    match dialog.kind {
        FileDialogKind::SaveFile => {
            if let Some(name) = &dialog.file_name {
                options.push(("current_name", Value::Text(name.clone())));
            }
        }
        FileDialogKind::PickFolder => {
            options.push(("multiple", Value::Bool(false)));
            options.push(("directory", Value::Bool(true)));
        }
        FileDialogKind::OpenFile | FileDialogKind::OpenFiles => {
            let multiple = dialog.kind == FileDialogKind::OpenFiles;
            options.push(("multiple", Value::Bool(multiple)));
        }
    }
    if dialog.kind != FileDialogKind::PickFolder && !dialog.filters.is_empty() {
        let filters: Vec<Filter> = dialog
            .filters
            .iter()
            .map(|filter| {
                let mut patterns: Vec<String> = filter
                    .extensions
                    .iter()
                    .map(|e| format!("*.{}", e.trim_start_matches('.')))
                    .collect();
                if patterns.is_empty() {
                    patterns.push("*".to_owned());
                }
                (filter.name.clone(), patterns)
            })
            .collect();
        // Le premier filtre est sélectionné, comme sous Windows.
        options.push(("current_filter", Value::Filter(filters[0].clone())));
        options.push(("filters", Value::Filters(filters)));
    }
    if let Some(directory) = &dialog.directory {
        let mut bytes = directory.as_os_str().as_bytes().to_vec();
        bytes.push(0);
        options.push(("current_folder", Value::Bytes(bytes)));
    }
    options
}
/// Écrit les options en dictionnaire `a{sv}`.
pub(super) fn write_options(writer: &mut Writer<'_>, options: &[(&'static str, Value)]) {
    writer.open(b'a', Some("{sv}"));
    for (key, value) in options {
        writer.open(b'e', None).string(key);
        match value {
            Value::Bool(value) => {
                writer.open(b'v', Some("b")).bool(*value).close();
            }
            Value::Text(value) => {
                writer.open(b'v', Some("s")).string(value).close();
            }
            Value::Bytes(bytes) => {
                writer.open(b'v', Some("ay")).open(b'a', Some("y"));
                for byte in bytes {
                    writer.byte(*byte);
                }
                writer.close().close();
            }
            Value::Filters(filters) => {
                writer
                    .open(b'v', Some("a(sa(us))"))
                    .open(b'a', Some("(sa(us))"));
                for filter in filters {
                    write_filter(writer, filter);
                }
                writer.close().close();
            }
            Value::Filter(filter) => {
                writer.open(b'v', Some("(sa(us))"));
                write_filter(writer, filter);
                writer.close();
            }
        }
        writer.close();
    }
    writer.close();
}
/// Filtre `(sa(us))` : nom, puis motifs glob (type 0).
fn write_filter(writer: &mut Writer<'_>, (name, patterns): &Filter) {
    writer
        .open(b'r', None)
        .string(name)
        .open(b'a', Some("(us)"));
    for pattern in patterns {
        writer.open(b'r', None).u32(0).string(pattern).close();
    }
    writer.close().close();
}
/// Chemins de la réponse (`u` code, `a{sv}` résultats) : code 0, choix fait (`uris`) ;
/// 1, annulé par l'utilisateur ; autre, dialogue interrompu par le système.
pub(super) fn response(message: &Message) -> Result<Vec<PathBuf>, String> {
    let mut reader = message.reader();
    match reader.u32() {
        Some(0) => {}
        Some(1) => return Ok(Vec::new()),
        _ => return Err("the file chooser was interrupted".to_owned()),
    }
    let mut uris = Vec::new();
    if let Some(mut results) = reader.enter() {
        while let Some(mut entry) = results.enter() {
            if entry.string().as_deref() != Some("uris") {
                continue;
            }
            if let Some(mut variant) = entry.enter()
                && let Some(mut list) = variant.enter()
            {
                while let Some(uri) = list.string() {
                    uris.push(uri);
                }
            }
        }
    }
    Ok(uris.iter().filter_map(|uri| path_from_uri(uri)).collect())
}
/// Chemin local d'une URI `file://` (hôte vide ou `localhost`), décodée de ses `%XX` ;
/// `None` pour une autre URI.
pub(super) fn path_from_uri(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    let path = match rest.find('/') {
        Some(0) => rest,
        Some(slash) if &rest[..slash] == "localhost" => &rest[slash..],
        _ => return None,
    };
    let bytes = path.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let escaped = (bytes[i] == b'%')
            .then(|| bytes.get(i + 1..i + 3))
            .flatten()
            .filter(|hex| hex.iter().all(u8::is_ascii_hexdigit))
            .and_then(|hex| std::str::from_utf8(hex).ok())
            .and_then(|hex| u8::from_str_radix(hex, 16).ok());
        match escaped {
            Some(byte) => {
                decoded.push(byte);
                i += 3;
            }
            None => {
                decoded.push(bytes[i]);
                i += 1;
            }
        }
    }
    Some(PathBuf::from(std::ffi::OsString::from_vec(decoded)))
}
