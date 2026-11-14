//! Thread d'une icône : connexion privée au bus de session, nom `StatusNotifierItem`,
//! inscription auprès du `StatusNotifierWatcher`, service des appels de l'hôte et signaux
//! de mise à jour. La fenêtre garde un [`Tray`] : sa destruction arrête le thread, qui
//! ferme la connexion (le bus libère le nom, l'hôte retire l'icône).
use super::{dbusmenu, item, pixmap::Pixmap};
use crate::{
    platform::linux::{
        ffi::dbus::{Connection, METHOD_CALL, Message, SIGNAL, Writer},
        portal::Waker,
    },
    window::{TrayEvent, TrayIcon, TrayMenuItem},
};
use std::sync::{
    atomic::{AtomicU32, Ordering},
    mpsc::{Receiver, Sender, TryRecvError, channel},
};

pub(super) const ITEM_PATH: &str = "/StatusNotifierItem";
pub(super) const MENU_PATH: &str = "/MenuBar";
pub(super) const PROPERTIES: &str = "org.freedesktop.DBus.Properties";
const INTROSPECTABLE: &str = "org.freedesktop.DBus.Introspectable";
pub(super) const UNKNOWN_METHOD: &str = "org.freedesktop.DBus.Error.UnknownMethod";
pub(super) const UNKNOWN_PROPERTY: &str = "org.freedesktop.DBus.Error.UnknownProperty";
const WATCHER: &str = "org.kde.StatusNotifierWatcher";
/// Attente des messages entre deux lectures des mises à jour de la fenêtre : borne le
/// délai d'un changement d'icône et de l'arrêt du thread.
const WAIT_MS: i32 = 250;

/// Ce que publie une icône : identité, titre (infobulle), images et menu.
#[derive(Clone, Debug)]
pub(super) struct Description {
    pub id: String,
    pub title: String,
    pub pixmaps: Vec<Pixmap>,
    pub menu: Vec<TrayMenuItem>,
}
impl Description {
    /// Décode l'image (sur le thread de l'interface) ; une image illisible est une erreur.
    fn new(icon: &TrayIcon) -> crate::Result<Self> {
        let id = crate::platform::linux::shell::app_id();
        Ok(Self {
            title: if icon.tooltip.is_empty() {
                id.clone()
            } else {
                icon.tooltip.clone()
            },
            id,
            pixmaps: super::pixmap::tray_pixmaps(&icon.image)?,
            menu: icon.menu.clone(),
        })
    }
}

enum Update {
    Set(Description),
    Stop,
}

/// Icône posée par une fenêtre ; retirée quand la valeur est détruite.
pub(in crate::platform::linux) struct Tray {
    updates: Sender<Update>,
    events: Receiver<TrayEvent>,
}
impl Tray {
    /// Pose une icône, ou met à jour `current` ; `None` la retire.
    pub fn apply(
        current: &mut Option<Tray>,
        icon: Option<TrayIcon>,
        wake: &Waker,
    ) -> crate::Result<()> {
        let Some(icon) = icon else {
            *current = None;
            return Ok(());
        };
        let description = Description::new(&icon)?;
        match current {
            Some(tray) => {
                let _ = tray.updates.send(Update::Set(description));
            }
            None => *current = Some(Self::start(description, wake.clone())),
        }
        Ok(())
    }
    fn start(description: Description, wake: Waker) -> Self {
        let (updates, pending) = channel();
        let (sender, events) = channel();
        let spawned = std::thread::Builder::new()
            .name("karbon-ui-tray".into())
            .spawn(move || serve(description, &pending, &sender, &wake));
        if let Err(error) = spawned {
            eprintln!("karbon-ui: tray icon unavailable ({error})");
        }
        Self { updates, events }
    }
    /// Clics et choix reçus depuis le dernier appel.
    pub fn events(&self) -> Vec<TrayEvent> {
        self.events.try_iter().collect()
    }
}
impl Drop for Tray {
    fn drop(&mut self) {
        let _ = self.updates.send(Update::Stop);
    }
}

/// Corps du thread : jusqu'à l'arrêt demandé ou la perte du bus.
fn serve(
    mut description: Description,
    updates: &Receiver<Update>,
    events: &Sender<TrayEvent>,
    wake: &Waker,
) {
    let Some(bus) = Connection::session() else {
        eprintln!("karbon-ui: tray icon unavailable (no D-Bus session bus)");
        return;
    };
    static NEXT: AtomicU32 = AtomicU32::new(1);
    let name = format!(
        "org.kde.StatusNotifierItem-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    );
    request_name(&bus, &name);
    // Le gardien peut démarrer (ou redémarrer) après l'application : s'y inscrire alors.
    bus.add_match(&format!(
        "type='signal',sender='org.freedesktop.DBus',interface='org.freedesktop.DBus',\
         member='NameOwnerChanged',arg0='{WATCHER}'"
    ));
    if !register(&bus, &name) {
        eprintln!(
            "karbon-ui: no StatusNotifierWatcher on the session bus; the tray icon appears \
             when one starts (GNOME needs the AppIndicator extension)"
        );
    }
    let notify = |event: TrayEvent| {
        if events.send(event).is_ok() {
            wake();
        }
    };
    // Révision du menu, annoncée par `LayoutUpdated` à chaque changement.
    let mut revision = 1_u32;
    loop {
        loop {
            match updates.try_recv() {
                Ok(Update::Set(next)) => {
                    description = next;
                    revision += 1;
                    announce(&bus, revision);
                }
                Ok(Update::Stop) | Err(TryRecvError::Disconnected) => return,
                Err(TryRecvError::Empty) => break,
            }
        }
        if !bus.wait(WAIT_MS) {
            return;
        }
        while let Some(message) = bus.pop() {
            match message.kind() {
                METHOD_CALL => {
                    if let Some(reply) = dispatch(&message, &description, revision, &notify) {
                        bus.send(&reply);
                    }
                }
                SIGNAL if message.member() == "NameOwnerChanged" => {
                    let mut args = message.reader();
                    let (_, _, owner) = (args.string(), args.string(), args.string());
                    if owner.is_some_and(|owner| !owner.is_empty()) {
                        register(&bus, &name);
                    }
                }
                _ => {}
            }
        }
        bus.flush();
    }
}

/// `org.freedesktop.DBus.RequestName` : le nom bien connu de l'icône.
fn request_name(bus: &Connection, name: &str) {
    if let Some(mut call) = Message::method_call(
        "org.freedesktop.DBus",
        "/org/freedesktop/DBus",
        "org.freedesktop.DBus",
        "RequestName",
    ) {
        call.writer().string(name).u32(0);
        bus.call(&call, 2000);
    }
}

/// `RegisterStatusNotifierItem` ; `false` sans gardien sur le bus.
fn register(bus: &Connection, name: &str) -> bool {
    let Some(mut call) = Message::method_call(
        WATCHER,
        "/StatusNotifierWatcher",
        WATCHER,
        "RegisterStatusNotifierItem",
    ) else {
        return false;
    };
    call.writer().string(name);
    bus.call(&call, 2000).is_some()
}

/// Signaux d'une mise à jour : image, titre, infobulle, menu.
fn announce(bus: &Connection, revision: u32) {
    for member in ["NewIcon", "NewTitle", "NewToolTip"] {
        if let Some(signal) = Message::signal(ITEM_PATH, item::INTERFACE, member) {
            bus.send(&signal);
        }
    }
    if let Some(mut signal) = Message::signal(MENU_PATH, dbusmenu::INTERFACE, "LayoutUpdated") {
        signal.writer().u32(revision).i32(0);
        bus.send(&signal);
    }
    bus.flush();
}

/// Réponse à un appel de l'hôte ; `None` s'il n'en attend pas.
fn dispatch(
    message: &Message,
    description: &Description,
    revision: u32,
    notify: &dyn Fn(TrayEvent),
) -> Option<Message> {
    let path = message.path();
    let interface = message.interface();
    if interface == INTROSPECTABLE {
        let mut reply = message.reply()?;
        reply.writer().string(&introspection(&path));
        return Some(reply);
    }
    match (path.as_str(), interface.as_str()) {
        (ITEM_PATH, PROPERTIES) => item::properties(message, description),
        (ITEM_PATH, _) => item::call(message, notify),
        (MENU_PATH, PROPERTIES) => dbusmenu::properties(message),
        (MENU_PATH, _) => dbusmenu::call(message, description, revision, notify),
        _ => message.error(UNKNOWN_METHOD, "unknown object"),
    }
}

/// Description XML des objets (certains hôtes la lisent avant d'appeler).
fn introspection(path: &str) -> String {
    let interface = match path {
        ITEM_PATH => item::INTROSPECTION,
        MENU_PATH => dbusmenu::INTROSPECTION,
        _ => "",
    };
    format!(
        "<!DOCTYPE node PUBLIC \"-//freedesktop//DTD D-BUS Object Introspection 1.0//EN\" \
         \"http://www.freedesktop.org/standards/dbus/1.0/introspect.dtd\">\n<node>\
         <interface name=\"{INTROSPECTABLE}\"><method name=\"Introspect\"><arg type=\"s\" \
         direction=\"out\"/></method></interface><interface name=\"{PROPERTIES}\">\
         <method name=\"Get\"><arg type=\"s\" direction=\"in\"/><arg type=\"s\" direction=\"in\"/>\
         <arg type=\"v\" direction=\"out\"/></method><method name=\"GetAll\"><arg type=\"s\" \
         direction=\"in\"/><arg type=\"a{{sv}}\" direction=\"out\"/></method></interface>\
         {interface}</node>"
    )
}

/// Variante de signature `signature`, écrite par `write`.
pub(super) fn variant(out: &mut Writer, signature: &str, write: impl FnOnce(&mut Writer)) {
    out.open(b'v', Some(signature));
    write(out);
    out.close();
}

/// `org.freedesktop.DBus.Properties` d'une interface : `Get` et `GetAll` en lecture
/// (`write` écrit la valeur d'un nom de `names` en variante), `Set` refusé.
pub(super) fn properties_call(
    message: &Message,
    interface: &str,
    names: &[&str],
    write: impl Fn(&str, &mut Writer),
) -> Option<Message> {
    let mut args = message.reader();
    let asked = args.string().unwrap_or_default();
    if !asked.is_empty() && asked != interface {
        return message.error(UNKNOWN_PROPERTY, "unknown interface");
    }
    match message.member().as_str() {
        "Get" => {
            let name = args.string().unwrap_or_default();
            if !names.contains(&name.as_str()) {
                return message.error(UNKNOWN_PROPERTY, "unknown property");
            }
            let mut reply = message.reply()?;
            write(&name, &mut reply.writer());
            Some(reply)
        }
        "GetAll" => {
            let mut reply = message.reply()?;
            {
                let mut out = reply.writer();
                out.open(b'a', Some("{sv}"));
                for name in names {
                    out.open(b'e', None).string(name);
                    write(name, &mut out);
                    out.close();
                }
                out.close();
            }
            Some(reply)
        }
        _ => message.error(UNKNOWN_METHOD, "read-only properties"),
    }
}
