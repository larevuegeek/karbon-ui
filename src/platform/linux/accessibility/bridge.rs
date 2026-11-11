pub(super) use super::tree::{ROOT, Tree};
use super::*;
pub(super) use crate::{
    platform::linux::ffi::dbus::{Connection, METHOD_CALL, METHOD_RETURN, Message},
    ui::runtime::{AccessibilityAction, UiRuntime},
    window::WindowId,
};
pub(super) use std::collections::{HashMap, HashSet};

/// Fenêtre exposée par le pont (implémentée par la boucle de fenêtres).
pub(in crate::platform::linux) trait Host {
    fn id(&self) -> WindowId;
    fn title(&self) -> &str;
    /// Taille logique de la zone de contenu.
    fn size(&self) -> (i32, i32);
    fn runtime(&self) -> &UiRuntime;
    /// La fenêtre a le clavier.
    fn active(&self) -> bool;
    /// Applique une action du lecteur d'écran sur un arbre à jour ; `false` si le moteur
    /// la refuse.
    fn act(&mut self, action: AccessibilityAction) -> crate::Result<bool>;
}
/// Délai des appels de connexion (bus de session, registre) : un démon sain répond en
/// quelques millisecondes ; gelé, il ne retarde pas le démarrage de plus d'une seconde.
pub(super) const TIMEOUT: i32 = 500;

pub(in crate::platform::linux) struct Bridge {
    pub(super) bus: Connection,
    /// Nom unique de l'application sur le bus d'accessibilité.
    pub(super) name: String,
    /// Bureau du registre, parent de l'application.
    pub(super) desktop: (String, String),
    pub(super) app: String,
    pub(super) trees: HashMap<WindowId, Tree>,
    /// Écoutes d'événements déclarées au registre (nom du client, événement).
    pub(super) listeners: HashSet<(String, String)>,
    /// Numéro de la demande des écoutes déjà déclarées, dont la réponse arrive par `pump`.
    pub(super) registered: Option<u32>,
    pub(super) announced: events::Announced,
}
impl Bridge {
    /// Connexion au bus d'accessibilité et enregistrement auprès du registre ; `None`
    /// sans session D-Bus ou sans at-spi2.
    pub fn connect() -> Option<Self> {
        let session = Connection::session()?;
        let call = Message::method_call(
            "org.a11y.Bus",
            "/org/a11y/bus",
            "org.a11y.Bus",
            "GetAddress",
        )?;
        let address = session.call(&call, TIMEOUT)?.reader().string()?;
        drop(session);
        let bus = Connection::open(&address)?;
        let name = bus.unique_name();
        let mut embed = Message::method_call(
            "org.a11y.atspi.Registry",
            ROOT,
            "org.a11y.atspi.Socket",
            "Embed",
        )?;
        embed.writer().reference(&name, ROOT);
        let reply = bus.call(&embed, TIMEOUT)?;
        let desktop = {
            let mut reader = reply.reader();
            let mut parent = reader.enter()?;
            (parent.string()?, parent.string()?)
        };
        for member in ["EventListenerRegistered", "EventListenerDeregistered"] {
            bus.add_match(&format!(
                "type='signal',interface='org.a11y.atspi.Registry',member='{member}'"
            ));
        }
        // Départ d'un client (lecteur d'écran arrêté sans se désinscrire).
        bus.add_match(
            "type='signal',sender='org.freedesktop.DBus',interface='org.freedesktop.DBus',\
             member='NameOwnerChanged'",
        );
        let mut bridge = Self {
            bus,
            name,
            desktop,
            app: app_name(),
            trees: HashMap::new(),
            listeners: HashSet::new(),
            registered: None,
            announced: Default::default(),
        };
        bridge.registered_events();
        Some(bridge)
    }
    /// Demande les écoutes déjà déclarées au registre, sans attendre : la réponse est lue
    /// par `pump` (`registered_reply`).
    pub(super) fn registered_events(&mut self) {
        let Some(call) = Message::method_call(
            "org.a11y.atspi.Registry",
            "/org/a11y/atspi/registry",
            "org.a11y.atspi.Registry",
            "GetRegisteredEvents",
        ) else {
            return;
        };
        self.registered = self.bus.send_serial(&call);
        self.bus.flush();
    }
    /// Réponse à `registered_events` : écoutes `(client, événement)`.
    fn registered_reply(&mut self, reply: &Message) {
        let mut reader = reply.reader();
        if let Some(mut list) = reader.enter() {
            while let Some(mut pair) = list.enter() {
                if let (Some(bus), Some(event)) = (pair.string(), pair.string()) {
                    self.listeners.insert((bus, event));
                }
            }
        }
    }
    /// Descripteur de la connexion, à surveiller par la boucle.
    pub fn fd(&self) -> Option<i32> {
        self.bus.fd()
    }
    /// Un client écoute des événements (lecteur d'écran, inspecteur).
    pub fn listened(&self) -> bool {
        !self.listeners.is_empty()
    }
    /// Traite les messages reçus : appels des clients et écoutes déclarées.
    pub fn pump<H: Host>(&mut self, windows: &mut [H]) {
        self.bus.pump();
        while let Some(message) = self.bus.pop() {
            if message.kind() == METHOD_RETURN
                && self.registered.is_some_and(|s| s == message.reply_serial())
            {
                self.registered = None;
                self.registered_reply(&message);
                continue;
            }
            if message.kind() != METHOD_CALL {
                self.signal(&message);
                continue;
            }
            self.refresh(windows);
            if let Some(reply) = self.call(&message, windows) {
                self.bus.send(&reply);
            }
        }
        self.bus.flush();
    }
    pub(super) fn signal(&mut self, message: &Message) {
        let mut reader = message.reader();
        match (message.interface().as_str(), message.member().as_str()) {
            ("org.a11y.atspi.Registry", member) => {
                let (Some(bus), Some(event)) = (reader.string(), reader.string()) else {
                    return;
                };
                match member {
                    "EventListenerRegistered" => {
                        self.listeners.insert((bus, event));
                    }
                    "EventListenerDeregistered" => {
                        self.listeners.remove(&(bus, event));
                    }
                    _ => {}
                }
            }
            // Un client parti sans se désinscrire (lecteur d'écran arrêté) : ses écoutes
            // sont oubliées, sinon les événements seraient calculés pour personne.
            ("org.freedesktop.DBus", "NameOwnerChanged") => {
                let (Some(name), Some(_old), Some(new)) =
                    (reader.string(), reader.string(), reader.string())
                else {
                    return;
                };
                if new.is_empty() {
                    self.listeners.retain(|(bus, _)| *bus != name);
                }
            }
            _ => {}
        }
    }
    /// Instantanés à jour pour les fenêtres ouvertes.
    pub(super) fn refresh<H: Host>(&mut self, windows: &[H]) {
        self.trees
            .retain(|id, _| windows.iter().any(|w| w.id() == *id));
        for w in windows {
            let version = w.runtime().accessibility_version();
            let current = self.trees.get(&w.id()).is_some_and(|t| {
                t.version == version && t.size == w.size() && t.title == w.title()
            });
            if !current {
                let tree = Tree::new(version, w.title(), w.size(), w.runtime().accessibility());
                self.trees.insert(w.id(), tree);
            }
        }
    }
}
/// Nom de l'application : nom de l'exécutable.
pub(super) fn app_name() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "karbon-ui".into())
}
