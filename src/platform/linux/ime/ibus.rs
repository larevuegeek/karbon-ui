//! Client IBus (D-Bus) : un contexte de saisie par boucle de fenêtres, créé sur le bus
//! privé d'IBus (adresse de `IBUS_ADDRESS`, sinon du fichier
//! `~/.config/ibus/bus/<machine-id>-<hôte>-<écran>`, comme libibus), à défaut par le
//! portail IBus du bus de session.
//!
//! Les touches sont transmises par `ProcessKeyEvent` sans attendre, comme le fait GTK :
//! la réponse (`Request::Answered`) dit si le moteur la prend ; sinon l'application la
//! traite, dans l'ordre (`x11/ime.rs`). Elle reçoit aussi par signaux le texte validé (`CommitText`), la composition (`UpdatePreeditText`,
//! `HidePreeditText`) ou une touche à traiter elle-même (`ForwardKeyEvent`). Le contexte
//! annonce qu'il affiche la composition dans le champ (`PREEDIT_TEXT`) et qu'il suit le
//! focus (`FOCUS`).
use super::Request;
use crate::platform::linux::ffi::dbus::{
    Connection, ERROR, METHOD_RETURN, Message, Reader, SIGNAL,
};

const INTERFACE: &str = "org.freedesktop.IBus.InputContext";
/// Capacités : composition affichée par l'application, focus.
const CAPABILITIES: u32 = 1 | 1 << 3;
/// État d'une touche relâchée (`IBUS_RELEASE_MASK`).
const RELEASE: u32 = 1 << 30;

/// Adresse du bus privé d'IBus pour l'écran X de `DISPLAY` : un socket Unix seulement
/// (libdbus accepte aussi `unixexec:`, qui lance un programme ; l'adresse vient de
/// l'environnement ou d'un fichier de l'utilisateur, ce n'est pas une frontière de
/// confiance, mais rien ne justifie de l'exécuter).
fn address() -> Option<String> {
    if let Ok(address) = std::env::var("IBUS_ADDRESS")
        && address.starts_with("unix:")
    {
        return Some(address);
    }
    let machine = ["/etc/machine-id", "/var/lib/dbus/machine-id"]
        .iter()
        .find_map(|p| std::fs::read_to_string(p).ok())?;
    let config = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|c| !c.is_empty())
        .or_else(|| std::env::var("HOME").ok().map(|h| format!("{h}/.config")))?;
    let file = bus_file(&config, &machine, &std::env::var("DISPLAY").ok()?)?;
    bus_address(&std::fs::read_to_string(file).ok()?).filter(|a| a.starts_with("unix:"))
}
/// Fichier où le démon IBus publie son adresse : `<config>/ibus/bus/<machine-id>-<hôte>-
/// <écran>` (hôte `unix` pour un écran local, comme libibus).
pub(super) fn bus_file(config: &str, machine: &str, display: &str) -> Option<String> {
    let (host, screen) = display.split_once(':')?;
    let host = if host.is_empty() { "unix" } else { host };
    let number = screen.split('.').next().filter(|n| !n.is_empty())?;
    Some(format!(
        "{config}/ibus/bus/{}-{host}-{number}",
        machine.trim()
    ))
}
/// Adresse D-Bus lue dans ce fichier (ligne `IBUS_ADDRESS=`).
pub(super) fn bus_address(file: &str) -> Option<String> {
    file.lines()
        .find_map(|line| line.strip_prefix("IBUS_ADDRESS="))
        .map(str::to_owned)
        .filter(|a| !a.is_empty())
}
/// Texte d'un `IBusText` (variante de la structure `(sa{sv}sv)` : nom du type,
/// pièces jointes, texte, attributs).
fn text(reader: &mut Reader<'_>) -> Option<String> {
    let mut variant = reader.enter()?;
    let mut fields = variant.enter()?;
    fields.string()?;
    fields.skip();
    fields.string()
}
pub(in crate::platform::linux) struct Ibus {
    connection: Connection,
    /// Service qui porte le contexte (démon IBus, ou portail).
    service: &'static str,
    context: String,
    focused: bool,
    cursor: Option<[i32; 4]>,
}
impl Ibus {
    /// Contexte de saisie, sur le bus privé d'IBus ou par le portail ; `None` sans IBus.
    pub fn connect() -> Option<Self> {
        let private = address()
            .and_then(|a| Connection::open(&a))
            .and_then(|c| Self::create(c, "org.freedesktop.IBus", "org.freedesktop.IBus"));
        private.or_else(|| {
            let session = Connection::session()?;
            Self::create(
                session,
                "org.freedesktop.portal.IBus",
                "org.freedesktop.IBus.Portal",
            )
        })
    }
    fn create(connection: Connection, service: &'static str, interface: &str) -> Option<Self> {
        let mut call = Message::method_call(
            service,
            "/org/freedesktop/IBus",
            interface,
            "CreateInputContext",
        )?;
        call.writer().string("karbon-ui");
        let context = connection.call(&call, 1000)?.reader().string()?;
        connection.add_match(&format!(
            "type='signal',interface='{INTERFACE}',path='{context}'"
        ));
        let this = Self {
            connection,
            service,
            context,
            focused: false,
            cursor: None,
        };
        this.send("SetCapabilities", |w| {
            w.u32(CAPABILITIES);
        });
        Some(this)
    }
    fn send(
        &self,
        method: &str,
        args: impl FnOnce(&mut crate::platform::linux::ffi::dbus::Writer<'_>),
    ) {
        let Some(mut message) =
            Message::method_call(self.service, &self.context, INTERFACE, method)
        else {
            return;
        };
        args(&mut message.writer());
        self.connection.send(&message);
        self.connection.flush();
    }
    /// Descripteur de la connexion (signaux à lire).
    pub fn fd(&self) -> Option<i32> {
        self.connection.fd()
    }
    /// Un champ de saisie a le focus (ou plus aucun).
    pub fn focus(&mut self, focused: bool) {
        if self.focused == focused {
            return;
        }
        self.focused = focused;
        self.cursor = None;
        self.send(if focused { "FocusIn" } else { "FocusOut" }, |_| {});
    }
    pub fn focused(&self) -> bool {
        self.focused
    }
    /// Caret du champ, en pixels de l'écran : la fenêtre des candidats se place dessous.
    pub fn cursor(&mut self, rect: [i32; 4]) {
        if self.cursor == Some(rect) {
            return;
        }
        self.cursor = Some(rect);
        self.send("SetCursorLocation", |w| {
            w.i32(rect[0]).i32(rect[1]).i32(rect[2]).i32(rect[3]);
        });
    }
    /// Abandonne la composition en cours (le texte composé a été validé par l'application).
    pub fn reset(&self) {
        self.send("Reset", |_| {});
    }
    /// Transmet une touche (`keyval` : symbole XKB, `keycode` : code evdev, `state` :
    /// modificateurs X) sans attendre ; le numéro de sa réponse (`Request::Answered`), ou
    /// `None` si elle n'a pas pu partir.
    pub fn key(&self, keyval: u32, keycode: u32, state: u32, pressed: bool) -> Option<u32> {
        let mut message =
            Message::method_call(self.service, &self.context, INTERFACE, "ProcessKeyEvent")?;
        let state = if pressed { state } else { state | RELEASE };
        message.writer().u32(keyval).u32(keycode).u32(state);
        let serial = self.connection.send_serial(&message);
        self.connection.flush();
        serial
    }
    /// Demandes reçues du moteur depuis le dernier appel, dans l'ordre.
    pub fn requests(&self) -> Vec<Request> {
        self.connection.pump();
        let mut requests = Vec::new();
        while let Some(message) = self.connection.pop() {
            // Réponse à une touche : prise (`true`), ou non (`false`, erreur du moteur).
            if matches!(message.kind(), METHOD_RETURN | ERROR) {
                let taken =
                    message.kind() == METHOD_RETURN && message.reader().bool() == Some(true);
                requests.push(Request::Answered {
                    serial: message.reply_serial(),
                    taken,
                });
                continue;
            }
            if message.kind() != SIGNAL
                || message.interface() != INTERFACE
                || message.path() != self.context
            {
                continue;
            }
            let mut reader = message.reader();
            match message.member().as_str() {
                "CommitText" => {
                    if let Some(text) = text(&mut reader) {
                        requests.push(Request::Commit(text));
                    }
                }
                "UpdatePreeditText" => {
                    let text = text(&mut reader).unwrap_or_default();
                    let cursor = reader.u32().unwrap_or(0) as usize;
                    let visible = reader.bool().unwrap_or(true);
                    requests.push(if visible {
                        Request::Preedit(text, cursor)
                    } else {
                        Request::Preedit(String::new(), 0)
                    });
                }
                "HidePreeditText" => requests.push(Request::Preedit(String::new(), 0)),
                "ForwardKeyEvent" => {
                    if let (Some(keyval), Some(keycode), Some(state)) =
                        (reader.u32(), reader.u32(), reader.u32())
                    {
                        requests.push(Request::Forward {
                            keyval,
                            keycode,
                            state,
                        });
                    }
                }
                _ => {}
            }
        }
        requests
    }
}
impl Drop for Ibus {
    fn drop(&mut self) {
        self.send("Destroy", |_| {});
    }
}
