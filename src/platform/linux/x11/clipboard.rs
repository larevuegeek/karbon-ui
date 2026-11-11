//! Presse-papiers X11 (sélection `CLIPBOARD`, ICCCM) : une fenêtre invisible du backend
//! possède la sélection quand l'application copie (elle survit aux fenêtres de
//! l'application) et sert les demandes des autres clients (`TARGETS`, `UTF8_STRING`,
//! `STRING`, `TEXT`) ; une lecture demande la conversion en `UTF8_STRING` et attend la
//! réponse (une seconde au plus par étape), les autres événements reçus entre-temps étant
//! gardés pour la boucle. Au-delà de 256 Ko, le texte passe par morceaux (`INCR`), dans les
//! deux sens, comme le fait GTK.
use super::connection::Connection;
use crate::platform::linux::ffi::xcb::*;
use std::time::{Duration, Instant};

/// Attente au plus d'une réponse du propriétaire de la sélection, ou d'un morceau.
const READ_TIMEOUT: Duration = Duration::from_secs(1);
/// Taille d'un morceau, et au-delà de laquelle un texte servi passe par morceaux.
const CHUNK: usize = 256 * 1024;
/// Texte lu au plus (ICCCM ne borne pas un transfert par morceaux).
const MAX_READ: usize = 64 << 20;
/// Transfert servi abandonné faute de nouvelles du demandeur.
const TRANSFER_TIMEOUT: Duration = Duration::from_secs(30);

/// Texte servi par morceaux : le demandeur supprime la propriété pour recevoir la suite.
struct Transfer {
    requestor: u32,
    property: u32,
    kind: u32,
    bytes: Vec<u8>,
    offset: usize,
    last: Instant,
}

pub(super) struct Clipboard {
    /// Fenêtre invisible qui possède la sélection et reçoit les conversions.
    window: u32,
    /// Texte copié par l'application, tant qu'elle possède la sélection.
    text: Option<String>,
    transfers: Vec<Transfer>,
}
impl Clipboard {
    pub fn new(connection: &Connection) -> Self {
        let xcb = connection.xcb;
        // SAFETY : connexion valide ; identifiant neuf ; masque et valeur locaux.
        let window = unsafe {
            let window = (xcb.xcb_generate_id)(connection.raw);
            let events = XCB_EVENT_MASK_PROPERTY_CHANGE;
            (xcb.xcb_create_window)(
                connection.raw,
                0,
                window,
                connection.root,
                -1,
                -1,
                1,
                1,
                0,
                XCB_WINDOW_CLASS_INPUT_ONLY,
                0,
                XCB_CW_EVENT_MASK,
                (&raw const events).cast(),
            );
            window
        };
        Self {
            window,
            text: None,
            transfers: Vec::new(),
        }
    }
    /// L'application copie `text` (`time` : instant de l'entrée qui l'a demandé). ICCCM :
    /// une demande plus ancienne que le dernier changement de propriétaire est ignorée
    /// sans `SelectionClear` ; le texte n'est gardé que si la fenêtre possède bien la
    /// sélection, sinon `read` rendrait un texte périmé à la place de celui du vrai
    /// propriétaire.
    pub fn write(&mut self, connection: &Connection, text: &str, time: u32) {
        let xcb = connection.xcb;
        let selection = connection.atoms.clipboard;
        // SAFETY : fenêtre du presse-papiers valide ; réponse possédée, libérée ici.
        let owned = unsafe {
            (xcb.xcb_set_selection_owner)(connection.raw, self.window, selection, time);
            let cookie = (xcb.xcb_get_selection_owner)(connection.raw, selection);
            let reply =
                (xcb.xcb_get_selection_owner_reply)(connection.raw, cookie, std::ptr::null_mut());
            let owner = reply.as_ref().map(|r| r.owner);
            libc::free(reply.cast());
            owner == Some(self.window)
        };
        self.text = owned.then(|| text.to_owned());
    }
    /// `SelectionClear` : un autre client possède la sélection.
    pub fn cleared(&mut self, event: &xcb_selection_clear_event_t) {
        if event.owner == self.window {
            self.text = None;
        }
    }
    /// `SelectionRequest` : un client demande le texte copié dans le format `target`.
    pub fn request(&mut self, connection: &Connection, event: &xcb_selection_request_event_t) {
        let atoms = connection.atoms;
        let xcb = connection.xcb;
        // Propriété vide (ancien client) : la cible elle-même.
        let property = if event.property == 0 {
            event.target
        } else {
            event.property
        };
        let mut answered = 0;
        if let Some(text) = &self.text
            && event.selection == atoms.clipboard
        {
            if event.target == atoms.targets {
                let targets = [
                    atoms.targets,
                    atoms.utf8_string,
                    XCB_ATOM_STRING,
                    atoms.text,
                ];
                // SAFETY : requérant donné par le serveur ; données locales.
                unsafe {
                    (xcb.xcb_change_property)(
                        connection.raw,
                        XCB_PROP_MODE_REPLACE,
                        event.requestor,
                        property,
                        XCB_ATOM_ATOM,
                        32,
                        targets.len() as u32,
                        targets.as_ptr().cast(),
                    )
                };
                answered = property;
            } else if [atoms.utf8_string, XCB_ATOM_STRING, atoms.text].contains(&event.target) {
                let kind = if event.target == XCB_ATOM_STRING {
                    XCB_ATOM_STRING
                } else {
                    atoms.utf8_string
                };
                // `STRING` est du Latin-1 : les caractères hors de ce jeu deviennent « ? ».
                let bytes: Vec<u8> = if kind == XCB_ATOM_STRING {
                    text.chars()
                        .map(|c| u8::try_from(u32::from(c)).unwrap_or(b'?'))
                        .collect()
                } else {
                    text.as_bytes().to_vec()
                };
                if bytes.len() > CHUNK {
                    self.start_transfer(connection, event.requestor, property, kind, bytes);
                } else {
                    put(connection, event.requestor, property, kind, &bytes);
                }
                answered = property;
            }
        }
        let notify = xcb_selection_notify_event_t {
            response_type: XCB_SELECTION_NOTIFY,
            pad0: 0,
            sequence: 0,
            time: event.time,
            requestor: event.requestor,
            selection: event.selection,
            target: event.target,
            property: answered,
        };
        // SAFETY : événement local de 24 octets, complété à 32 (taille d'un événement
        // envoyé) dans un tampon local.
        unsafe {
            let mut bytes = [0u8; 32];
            std::ptr::copy_nonoverlapping(
                (&raw const notify).cast::<u8>(),
                bytes.as_mut_ptr(),
                size_of::<xcb_selection_notify_event_t>(),
            );
            (xcb.xcb_send_event)(connection.raw, 0, event.requestor, 0, bytes.as_ptr().cast());
        }
        connection.flush();
    }
    /// Début d'un transfert par morceaux : la propriété annonce `INCR` et la taille ; les
    /// suppressions de la propriété par le demandeur appellent les morceaux.
    fn start_transfer(
        &mut self,
        connection: &Connection,
        requestor: u32,
        property: u32,
        kind: u32,
        bytes: Vec<u8>,
    ) {
        let now = Instant::now();
        self.transfers.retain(|t| {
            now.duration_since(t.last) < TRANSFER_TIMEOUT
                && (t.requestor, t.property) != (requestor, property)
        });
        let xcb = connection.xcb;
        let size = u32::try_from(bytes.len()).unwrap_or(u32::MAX);
        let events = XCB_EVENT_MASK_PROPERTY_CHANGE;
        // SAFETY : requérant donné par le serveur ; valeurs locales lues pendant l'appel.
        unsafe {
            (xcb.xcb_change_window_attributes)(
                connection.raw,
                requestor,
                XCB_CW_EVENT_MASK,
                (&raw const events).cast(),
            );
            (xcb.xcb_change_property)(
                connection.raw,
                XCB_PROP_MODE_REPLACE,
                requestor,
                property,
                connection.atoms.incr,
                32,
                1,
                (&raw const size).cast(),
            );
        }
        self.transfers.push(Transfer {
            requestor,
            property,
            kind,
            bytes,
            offset: 0,
            last: now,
        });
    }
    /// `PropertyNotify` : une propriété supprimée par un demandeur appelle le morceau
    /// suivant de son transfert, puis un morceau vide qui le termine.
    pub fn property_changed(
        &mut self,
        connection: &Connection,
        event: &xcb_property_notify_event_t,
    ) {
        if event.state != XCB_PROPERTY_DELETE {
            return;
        }
        let Some(index) = self
            .transfers
            .iter()
            .position(|t| (t.requestor, t.property) == (event.window, event.atom))
        else {
            return;
        };
        let transfer = &mut self.transfers[index];
        let end = (transfer.offset + CHUNK).min(transfer.bytes.len());
        let chunk = &transfer.bytes[transfer.offset..end];
        put(
            connection,
            transfer.requestor,
            transfer.property,
            transfer.kind,
            chunk,
        );
        transfer.last = Instant::now();
        if chunk.is_empty() {
            let requestor = transfer.requestor;
            self.transfers.remove(index);
            let none = 0u32;
            // SAFETY : fenêtre du demandeur ; ses événements ne nous servent plus.
            unsafe {
                (connection.xcb.xcb_change_window_attributes)(
                    connection.raw,
                    requestor,
                    XCB_CW_EVENT_MASK,
                    (&raw const none).cast(),
                )
            };
        } else {
            transfer.offset = end;
        }
        connection.flush();
    }
    /// Texte du presse-papiers : celui de l'application si elle le possède, sinon celui
    /// du propriétaire, converti en `UTF8_STRING`, par morceaux s'il le faut. Les autres
    /// événements reçus pendant l'attente sont rangés dans `deferred`.
    pub fn read(
        &self,
        connection: &Connection,
        time: u32,
        deferred: &mut std::collections::VecDeque<*mut xcb_generic_event_t>,
    ) -> Option<String> {
        if let Some(text) = &self.text {
            return Some(text.clone());
        }
        let atoms = connection.atoms;
        let xcb = connection.xcb;
        // SAFETY : fenêtre du presse-papiers valide.
        unsafe {
            (xcb.xcb_convert_selection)(
                connection.raw,
                self.window,
                atoms.clipboard,
                atoms.utf8_string,
                atoms.karbon_clipboard,
                time,
            )
        };
        connection.flush();
        let window = self.window;
        // SAFETY : événement valide d'au moins 32 octets, du type testé.
        let property = wait(connection, deferred, |event| unsafe {
            ((*event).response_type & 0x7f == XCB_SELECTION_NOTIFY)
                .then(|| &*event.cast::<xcb_selection_notify_event_t>())
                .filter(|n| n.requestor == window)
                .map(|n| n.property)
        })?;
        if property == 0 {
            // Le propriétaire ne peut pas convertir en texte.
            return None;
        }
        // Lue et supprimée : pour `INCR`, la suppression appelle le premier morceau.
        let (kind, bytes) = connection.take_property(window, property);
        if kind != atoms.incr {
            return Some(String::from_utf8_lossy(&bytes).into_owned());
        }
        let mut text = Vec::new();
        loop {
            // SAFETY : idem.
            wait(connection, deferred, |event| unsafe {
                ((*event).response_type & 0x7f == XCB_PROPERTY_NOTIFY)
                    .then(|| &*event.cast::<xcb_property_notify_event_t>())
                    .filter(|n| {
                        (n.window, n.atom, n.state) == (window, property, XCB_PROPERTY_NEW_VALUE)
                    })
                    .map(drop)
            })?;
            let (_, chunk) = connection.take_property(window, property);
            if chunk.is_empty() {
                return Some(String::from_utf8_lossy(&text).into_owned());
            }
            text.extend_from_slice(&chunk);
            if text.len() > MAX_READ {
                return None;
            }
        }
    }
    pub fn close(&self, connection: &Connection) {
        // SAFETY : fenêtre créée par `new`, détruite une fois.
        unsafe { (connection.xcb.xcb_destroy_window)(connection.raw, self.window) };
    }
}
/// Pose `bytes` (format 8, type `kind`) dans la propriété `property` de `window`.
fn put(connection: &Connection, window: u32, property: u32, kind: u32, bytes: &[u8]) {
    // SAFETY : fenêtre d'un client donnée par le serveur ; données lues pendant l'appel.
    unsafe {
        (connection.xcb.xcb_change_property)(
            connection.raw,
            XCB_PROP_MODE_REPLACE,
            window,
            property,
            kind,
            8,
            bytes.len() as u32,
            bytes.as_ptr().cast(),
        )
    };
}
/// Attend un événement que `matches` reconnaît (une seconde au plus) ; les autres sont
/// rangés dans `deferred`, dans l'ordre. `None` après le délai.
fn wait<T>(
    connection: &Connection,
    deferred: &mut std::collections::VecDeque<*mut xcb_generic_event_t>,
    matches: impl Fn(*mut xcb_generic_event_t) -> Option<T>,
) -> Option<T> {
    let xcb = connection.xcb;
    let deadline = Instant::now() + READ_TIMEOUT;
    loop {
        // SAFETY : connexion valide ; événement possédé, libéré ou gardé ci-dessous.
        let event = unsafe { (xcb.xcb_poll_for_event)(connection.raw) };
        if event.is_null() {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return None;
            }
            let mut fd = libc::pollfd {
                // SAFETY : connexion valide.
                fd: unsafe { (xcb.xcb_get_file_descriptor)(connection.raw) },
                events: libc::POLLIN,
                revents: 0,
            };
            // SAFETY : une structure valide pendant l'appel.
            unsafe { libc::poll(&mut fd, 1, left.as_millis().max(1) as i32) };
            continue;
        }
        match matches(event) {
            Some(value) => {
                // SAFETY : événement possédé, libéré une fois.
                unsafe { libc::free(event.cast()) };
                return Some(value);
            }
            None => deferred.push_back(event),
        }
    }
}
