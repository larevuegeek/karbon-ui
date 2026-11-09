//! Thème du bureau par le portail `org.freedesktop.portal.Settings` : commun à GNOME, KDE
//! et aux autres bureaux qui fournissent un portail, sous Wayland comme sous X11.
use crate::platform::linux::ffi::dbus::{Connection, Message};

const SERVICE: &str = "org.freedesktop.portal.Desktop";
const OBJECT: &str = "/org/freedesktop/portal/desktop";
const INTERFACE: &str = "org.freedesktop.portal.Settings";
/// Attente au plus de la réponse (le portail peut être démarré à la demande par le bus).
const CALL_TIMEOUT: i32 = 2_000;
/// Valeur de `color-scheme` : 0, pas de préférence ; 1, sombre ; 2, clair.
const PREFER_DARK: u32 = 1;

/// Le bureau préfère le thème sombre ; `None` sans portail ou sans ce réglage. Bloquant :
/// à appeler hors de la boucle de fenêtres.
pub(in crate::platform::linux) fn prefers_dark() -> Option<bool> {
    let connection = Connection::session()?;
    let mut call = Message::method_call(SERVICE, OBJECT, INTERFACE, "Read")?;
    call.writer()
        .string("org.freedesktop.appearance")
        .string("color-scheme");
    parse(&connection.call(&call, CALL_TIMEOUT)?)
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
