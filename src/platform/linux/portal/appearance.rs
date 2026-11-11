//! Thème du bureau par le portail `org.freedesktop.portal.Settings` : commun à GNOME, KDE
//! et aux autres bureaux qui fournissent un portail, sous Wayland comme sous X11.
use crate::platform::linux::ffi::dbus::{Connection, Message, SIGNAL};

const SERVICE: &str = "org.freedesktop.portal.Desktop";
const OBJECT: &str = "/org/freedesktop/portal/desktop";
const INTERFACE: &str = "org.freedesktop.portal.Settings";
/// Attente au plus de la réponse (le portail peut être démarré à la demande par le bus).
const CALL_TIMEOUT: i32 = 2_000;
/// Valeur de `color-scheme` : 0, pas de préférence ; 1, sombre ; 2, clair.
const PREFER_DARK: u32 = 1;
/// Espace de noms et clé du thème.
const NAMESPACE: &str = "org.freedesktop.appearance";
const KEY: &str = "color-scheme";
/// Signaux de changement du thème seulement (le bus filtre les autres réglages).
const CHANGED: &str = "type='signal',interface='org.freedesktop.portal.Settings',member='SettingChanged',arg0='org.freedesktop.appearance',arg1='color-scheme'";

/// Le bureau préfère le thème sombre ; `None` sans portail ou sans ce réglage. Bloquant :
/// à appeler hors de la boucle de fenêtres.
pub(in crate::platform::linux) fn prefers_dark() -> Option<bool> {
    let connection = Connection::session()?;
    let mut call = Message::method_call(SERVICE, OBJECT, INTERFACE, "Read")?;
    call.writer().string(NAMESPACE).string(KEY);
    parse(&connection.call(&call, CALL_TIMEOUT)?)
}
/// Suit le thème du bureau en direct : `changed(sombre)` à chaque signal `SettingChanged`
/// de `color-scheme`, jusqu'à la perte du bus. Bloquant : sur un thread à lui.
pub(in crate::platform::linux) fn watch(changed: impl Fn(bool)) {
    let Some(connection) = Connection::session() else {
        return;
    };
    connection.add_match(CHANGED);
    while connection.wait(-1) {
        while let Some(message) = connection.pop() {
            if message.kind() == SIGNAL
                && message.member() == "SettingChanged"
                && let Some(dark) = parse_change(&message)
            {
                changed(dark);
            }
        }
    }
}
/// Signal `SettingChanged` (espace de noms, clé, valeur) : le thème, s'il s'agit de lui.
pub(super) fn parse_change(message: &Message) -> Option<bool> {
    let mut reader = message.reader();
    if reader.string()? != NAMESPACE || reader.string()? != KEY {
        return None;
    }
    let mut variant = reader.enter()?;
    let code = if variant.kind() == b'v' {
        variant.enter()?.u32()?
    } else {
        variant.u32()?
    };
    Some(code == PREFER_DARK)
}

/// Réponse de `Read` : une variante contenant la valeur, ou une variante dans une variante
/// selon la version du portail.
pub(super) fn parse(reply: &Message) -> Option<bool> {
    let mut reader = reply.reader();
    let mut variant = reader.enter()?;
    let code = if variant.kind() == b'v' {
        variant.enter()?.u32()?
    } else {
        variant.u32()?
    };
    Some(code == PREFER_DARK)
}
