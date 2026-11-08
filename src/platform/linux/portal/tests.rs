//! Parties du dialogue de fichiers qui ne demandent pas de portail : URI, chemin de la
//! requête, options et leur encodage D-Bus, lecture de la réponse.
use super::file_chooser::{Value, options, path_from_uri, request_path, response, write_options};
use crate::{platform::linux::ffi::dbus::Message, window::FileDialog};
use std::path::PathBuf;

#[test]
fn file_uris_are_percent_decoded_into_local_paths() {
    let path = |uri| path_from_uri(uri);
    assert_eq!(
        path("file:///home/a/Ma%20musique/%C3%A9t%C3%A9.flac"),
        Some(PathBuf::from("/home/a/Ma musique/été.flac"))
    );
    assert_eq!(
        path("file://localhost/tmp/a%25b"),
        Some(PathBuf::from("/tmp/a%b"))
    );
    // Échappement incomplet ou invalide : gardé tel quel.
    assert_eq!(
        path("file:///x%2/y%zz%+1"),
        Some(PathBuf::from("/x%2/y%zz%+1"))
    );
    // Octets non UTF-8 : chemin Unix quelconque.
    use std::os::unix::ffi::OsStrExt;
    assert_eq!(
        path("file:///b%FF").unwrap().as_os_str().as_bytes(),
        b"/b\xff"
    );
    assert_eq!(path("file://serveur/partage/x"), None);
    assert_eq!(path("https://exemple.org/x"), None);
    assert_eq!(path("file://"), None);
}

#[test]
fn the_request_path_follows_the_portal_naming() {
    assert_eq!(
        request_path(":1.42", "karbon_7_0"),
        "/org/freedesktop/portal/desktop/request/1_42/karbon_7_0"
    );
}

#[test]
fn options_follow_the_dialog_kind() {
    let keys = |dialog: &FileDialog| -> Vec<&str> {
        options(dialog, "t").into_iter().map(|(k, _)| k).collect()
    };
    let open = FileDialog::open_files()
        .filter("Audio", &["mp3", "flac"])
        .filter("Tout", &[])
        .directory("/home/a");
    let encoded = options(&open, "t");
    assert_eq!(encoded[0], ("handle_token", Value::Text("t".into())));
    assert!(encoded.contains(&("modal", Value::Bool(true))));
    assert!(encoded.contains(&("multiple", Value::Bool(true))));
    let audio = (
        "Audio".to_owned(),
        vec!["*.mp3".to_owned(), "*.flac".to_owned()],
    );
    assert!(encoded.contains(&("current_filter", Value::Filter(audio.clone()))));
    assert!(encoded.contains(&(
        "filters",
        Value::Filters(vec![audio, ("Tout".to_owned(), vec!["*".to_owned()])])
    )));
    assert!(encoded.contains(&("current_folder", Value::Bytes(b"/home/a\0".to_vec()))));

    let folder = FileDialog::pick_folder().filter("Audio", &["mp3"]);
    assert_eq!(
        keys(&folder),
        ["handle_token", "modal", "multiple", "directory"]
    );
    let save = FileDialog::save_file()
        .file_name("liste.m3u")
        .filter("M3U", &["m3u"]);
    assert_eq!(
        keys(&save),
        [
            "handle_token",
            "modal",
            "current_name",
            "current_filter",
            "filters"
        ]
    );
    assert!(!keys(&FileDialog::open_file()).contains(&"filters"));
}

#[test]
fn options_are_encoded_as_a_dictionary_of_variants() {
    let dialog = FileDialog::open_file()
        .filter("Audio", &["flac"])
        .directory("/tmp");
    let mut call = Message::method_call(
        "org.freedesktop.portal.Desktop",
        "/org/freedesktop/portal/desktop",
        "org.freedesktop.portal.FileChooser",
        "OpenFile",
    )
    .expect("message");
    {
        let mut writer = call.writer();
        writer.string("").string("Titre");
        write_options(&mut writer, &options(&dialog, "t"));
    }
    let mut reader = call.reader();
    assert_eq!(reader.string().as_deref(), Some(""));
    assert_eq!(reader.string().as_deref(), Some("Titre"));
    let mut dictionary = reader.enter().expect("a{sv}");
    let mut keys = Vec::new();
    while let Some(mut entry) = dictionary.enter() {
        let key = entry.string().expect("key");
        let mut variant = entry.enter().expect("variant");
        if key == "filters" {
            let mut list = variant.enter().expect("a(sa(us))");
            let mut filter = list.enter().expect("(sa(us))");
            assert_eq!(filter.string().as_deref(), Some("Audio"));
            let mut patterns = filter.enter().expect("a(us)");
            let mut pattern = patterns.enter().expect("(us)");
            assert_eq!(pattern.u32(), Some(0));
            assert_eq!(pattern.string().as_deref(), Some("*.flac"));
        }
        if key == "modal" {
            assert_eq!(variant.bool(), Some(true));
        }
        keys.push(key);
    }
    assert_eq!(
        keys,
        [
            "handle_token",
            "modal",
            "multiple",
            "current_filter",
            "filters",
            "current_folder"
        ]
    );
}

#[test]
fn responses_give_the_chosen_paths_or_nothing_when_cancelled() {
    let signal = |code: u32, uris: &[&str]| {
        let mut message = Message::signal(
            "/org/freedesktop/portal/desktop/request/1_42/t",
            "org.freedesktop.portal.Request",
            "Response",
        )
        .expect("signal");
        {
            let mut writer = message.writer();
            writer.u32(code).open(b'a', Some("{sv}"));
            writer.open(b'e', None).string("current_filter");
            writer
                .open(b'v', Some("s"))
                .string("ignoré")
                .close()
                .close();
            writer.open(b'e', None).string("uris");
            writer.open(b'v', Some("as")).open(b'a', Some("s"));
            for uri in uris {
                writer.string(uri);
            }
            writer.close().close().close().close();
        }
        message
    };
    assert_eq!(
        response(&signal(0, &["file:///a/b%20c.mp3", "file:///d.flac"])),
        Ok(vec![PathBuf::from("/a/b c.mp3"), PathBuf::from("/d.flac")])
    );
    assert_eq!(response(&signal(1, &[])), Ok(Vec::new()));
    assert!(response(&signal(2, &[])).is_err());
}
