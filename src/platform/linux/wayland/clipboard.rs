//! Presse-papiers (`wl_data_device`) : offres reçues du compositeur, sélection courante,
//! écriture par une source de l'application (texte servi sur demande, depuis un thread
//! pour ne pas bloquer la boucle) et lecture par un tube, avec un délai borné.
use super::{
    display::{Event, Kind, NEW, Ptr, create, destroy, request},
    protocol::{wl_data_device, wl_data_device_manager, wl_data_offer, wl_data_source},
};
use crate::platform::linux::ffi::wayland::{
    wl_data_device_interface, wl_data_source_interface, wl_display_flush,
};
use std::{
    io::{Read, Write},
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

/// Types de texte, du préféré au plus ancien.
const TEXT: [&std::ffi::CStr; 5] = [
    c"text/plain;charset=utf-8",
    c"UTF8_STRING",
    c"text/plain",
    c"TEXT",
    c"STRING",
];
/// Attente maximale du texte d'une autre application.
const READ_TIMEOUT: Duration = Duration::from_secs(2);
/// Au-delà, le texte d'une autre application n'est pas lu (une source hostile ou un
/// fichier énorme ne font pas allouer sans borne).
const READ_LIMIT: usize = 16 << 20;
/// Écritures du texte de l'application en cours au plus (un thread chacune).
const MAX_SENDERS: usize = 8;

pub(super) struct Clipboard {
    device: Ptr,
    /// Offres annoncées (objet, types), dont l'une peut devenir la sélection.
    offers: Vec<(Ptr, Vec<String>)>,
    selection: Option<Ptr>,
    /// Source de l'application tant qu'elle est la sélection, et son texte.
    source: Ptr,
    text: Arc<str>,
    /// Threads qui servent le texte (`wl_data_source.send`) encore en cours.
    senders: Arc<AtomicUsize>,
    /// Offre d'un glisser-déposer en cours au-dessus d'une fenêtre (non géré) : rendue à
    /// sa sortie, sinon elle s'accumulerait avec ses types jusqu'à la prochaine sélection.
    drag: Ptr,
}
impl Clipboard {
    pub fn new(manager: Ptr, seat: Ptr) -> Self {
        let device = if manager.is_null() || seat.is_null() {
            std::ptr::null_mut()
        } else {
            create!(
                manager,
                wl_data_device_manager::GET_DATA_DEVICE,
                &raw const wl_data_device_interface,
                Kind::DataDevice,
                NEW,
                seat
            )
        };
        Self {
            device,
            offers: Vec::new(),
            selection: None,
            source: std::ptr::null_mut(),
            text: Arc::from(""),
            senders: Arc::new(AtomicUsize::new(0)),
            drag: std::ptr::null_mut(),
        }
    }
    /// Événements du périphérique, des offres et de la source.
    pub fn event(&mut self, mut event: Event) {
        match (event.kind, event.opcode) {
            (Kind::DataDevice, wl_data_device::EV_DATA_OFFER) => {
                self.offers.push((event.object(0), Vec::new()));
            }
            (Kind::DataDevice, wl_data_device::EV_ENTER) => self.drag = event.object(4),
            (Kind::DataDevice, wl_data_device::EV_LEAVE) => {
                // Fin du glisser (après un dépôt aussi) : plus aucun événement pour
                // l'offre, qui est rendue.
                let offer = std::mem::replace(&mut self.drag, std::ptr::null_mut());
                destroy!(offer, wl_data_offer::DESTROY);
                self.offers.retain(|(o, _)| *o != offer);
            }
            (Kind::DataDevice, wl_data_device::EV_SELECTION) => {
                let offer = event.object(0);
                // Les autres offres (glisser-déposer, sélections remplacées) sont rendues.
                for (other, _) in self.offers.iter().filter(|(o, _)| *o != offer) {
                    destroy!(*other, wl_data_offer::DESTROY);
                }
                self.offers.retain(|(o, _)| *o == offer);
                self.selection = (!offer.is_null()).then_some(offer);
            }
            (Kind::DataOffer, wl_data_offer::EV_OFFER) => {
                if let Some((_, types)) = self.offers.iter_mut().find(|(o, _)| *o == event.target) {
                    types.push(event.string(0).to_owned());
                }
            }
            (Kind::DataSource, wl_data_source::EV_SEND) if event.target == self.source => {
                if let Some(fd) = event.take_fd(1) {
                    self.serve(fd);
                }
            }
            (Kind::DataSource, wl_data_source::EV_CANCELLED) => {
                destroy!(event.target, wl_data_source::DESTROY);
                if event.target == self.source {
                    self.source = std::ptr::null_mut();
                }
            }
            _ => {}
        }
    }
    /// Sert le texte de l'application sur `fd` depuis un thread (écriture bloquante : le
    /// lecteur peut être lent). Au-delà de `MAX_SENDERS` écritures en cours, le descripteur
    /// est fermé (fin de fichier pour le lecteur) : un client qui demande sans jamais lire
    /// ne fait pas croître le nombre de threads.
    fn serve(&self, fd: OwnedFd) {
        if self.senders.fetch_add(1, Ordering::AcqRel) >= MAX_SENDERS {
            self.senders.fetch_sub(1, Ordering::AcqRel);
            return;
        }
        let text = Arc::clone(&self.text);
        let senders = Arc::clone(&self.senders);
        let spawned = std::thread::Builder::new()
            .name("karbon-clipboard".into())
            .spawn(move || {
                // Un lecteur parti donne EPIPE (la runtime Rust ignore SIGPIPE).
                let _ = std::fs::File::from(fd).write_all(text.as_bytes());
                senders.fetch_sub(1, Ordering::AcqRel);
            });
        if spawned.is_err() {
            self.senders.fetch_sub(1, Ordering::AcqRel);
        }
    }
    /// Met `text` dans le presse-papiers ; `serial` est celui de l'entrée qui le demande.
    pub fn write(&mut self, manager: Ptr, serial: u32, text: &str) {
        if self.device.is_null() {
            return;
        }
        let source = create!(
            manager,
            wl_data_device_manager::CREATE_DATA_SOURCE,
            &raw const wl_data_source_interface,
            Kind::DataSource,
            NEW
        );
        for mime in TEXT {
            request!(source, wl_data_source::OFFER, mime.as_ptr());
        }
        request!(self.device, wl_data_device::SET_SELECTION, source, serial);
        if !self.source.is_null() {
            destroy!(self.source, wl_data_source::DESTROY);
        }
        self.source = source;
        self.text = Arc::from(text);
    }
    /// Texte du presse-papiers : celui de l'application si elle en est la source, sinon
    /// lu dans l'offre courante.
    pub fn read(&mut self, display: Ptr) -> Option<String> {
        if !self.source.is_null() {
            return Some(self.text.to_string());
        }
        let offer = self.selection?;
        let types = &self.offers.iter().find(|(o, _)| *o == offer)?.1;
        let mime = TEXT
            .iter()
            .find(|m| types.iter().any(|t| t.as_bytes() == m.to_bytes()))?;
        let mut fds = [0; 2];
        // SAFETY : tableau de deux descripteurs valide en écriture.
        if unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) } != 0 {
            return None;
        }
        // SAFETY : descripteurs fraîchement créés, possédés ici.
        let (reader, writer) =
            unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) };
        request!(
            offer,
            wl_data_offer::RECEIVE,
            mime.as_ptr(),
            writer.as_raw_fd()
        );
        drop(writer);
        // SAFETY : connexion valide ; la demande part avant la lecture.
        unsafe { wl_display_flush(display) };
        read_all(reader)
    }
}
/// Lit `fd` jusqu'à sa fin, en `READ_TIMEOUT` et `READ_LIMIT` octets au plus.
fn read_all(fd: OwnedFd) -> Option<String> {
    let deadline = Instant::now() + READ_TIMEOUT;
    let mut file = std::fs::File::from(fd);
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 16384];
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return None;
        }
        let mut poll = libc::pollfd {
            fd: file.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        // SAFETY : une structure valide pendant l'appel.
        let ready =
            unsafe { libc::poll(&mut poll, 1, left.as_millis().min(i32::MAX as u128) as i32) };
        if ready <= 0 {
            return None;
        }
        match file.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) if bytes.len() + n > READ_LIMIT => return None,
            Ok(n) => bytes.extend_from_slice(&chunk[..n]),
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(_) => return None,
        }
    }
    Some(String::from_utf8_lossy(&bytes).into_owned())
}
