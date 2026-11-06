//! Presse-papiers X11 (sélection `CLIPBOARD`, ICCCM) : une fenêtre invisible du backend
//! possède la sélection quand l'application copie (elle survit aux fenêtres de
//! l'application) et sert les demandes des autres clients (`TARGETS`, `UTF8_STRING`,
//! `STRING`, `TEXT`) ; une lecture demande la conversion en `UTF8_STRING` et attend la
//! réponse (une seconde au plus), les autres événements reçus entre-temps étant gardés
//! pour la boucle. Les transferts par morceaux (`INCR`) ne sont pas lus.
use super::connection::Connection;
use crate::platform::linux::ffi::xcb::*;
use std::time::{Duration, Instant};

/// Attente au plus d'une réponse du propriétaire de la sélection.
const READ_TIMEOUT: Duration = Duration::from_secs(1);

pub(super) struct Clipboard {
    /// Fenêtre invisible qui possède la sélection et reçoit les conversions.
    window: u32,
    /// Texte copié par l'application, tant qu'elle possède la sélection.
    text: Option<String>,
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
        Self { window, text: None }
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
    pub fn request(&self, connection: &Connection, event: &xcb_selection_request_event_t) {
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
                // Au plus ce qu'une requête peut porter (pas de transfert par morceaux),
                // sans couper un caractère.
                // SAFETY : connexion valide.
                let limit = unsafe { (xcb.xcb_get_maximum_request_length)(connection.raw) };
                let mut end = text.len().min((limit as usize * 4).saturating_sub(64));
                while !text.is_char_boundary(end) {
                    end -= 1;
                }
                // `STRING` est du Latin-1 : les caractères hors de ce jeu deviennent « ? ».
                let latin1: Vec<u8>;
                let bytes: &[u8] = if kind == XCB_ATOM_STRING {
                    latin1 = text[..end]
                        .chars()
                        .map(|c| u8::try_from(u32::from(c)).unwrap_or(b'?'))
                        .collect();
                    &latin1
                } else {
                    &text.as_bytes()[..end]
                };
                // SAFETY : idem.
                unsafe {
                    (xcb.xcb_change_property)(
                        connection.raw,
                        XCB_PROP_MODE_REPLACE,
                        event.requestor,
                        property,
                        kind,
                        8,
                        bytes.len() as u32,
                        bytes.as_ptr().cast(),
                    )
                };
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
    /// Texte du presse-papiers : celui de l'application si elle le possède, sinon celui
    /// du propriétaire, converti en `UTF8_STRING`. Les autres événements reçus pendant
    /// l'attente sont rangés dans `deferred`.
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
            // SAFETY : événement valide d'au moins 32 octets.
            let notify = unsafe {
                ((*event).response_type & 0x7f == XCB_SELECTION_NOTIFY)
                    .then(|| &*event.cast::<xcb_selection_notify_event_t>())
                    .filter(|n| n.requestor == self.window)
                    .map(|n| n.property)
            };
            let Some(property) = notify else {
                deferred.push_back(event);
                continue;
            };
            // SAFETY : événement possédé, libéré une fois.
            unsafe { libc::free(event.cast()) };
            if property == 0 {
                // Le propriétaire ne peut pas convertir en texte.
                return None;
            }
            let (kind, bytes) = connection.take_property(self.window, property);
            if kind == atoms.incr {
                return None;
            }
            return Some(String::from_utf8_lossy(&bytes).into_owned());
        }
    }
    pub fn close(&self, connection: &Connection) {
        // SAFETY : fenêtre créée par `new`, détruite une fois.
        unsafe { (connection.xcb.xcb_destroy_window)(connection.raw, self.window) };
    }
}
