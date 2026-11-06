//! Pointeur et défilement fin par XInput 2.1 (pavés tactiles, molettes à haute résolution),
//! comme GTK et Qt : chaque fenêtre demande les déplacements et les boutons des pointeurs
//! maîtres en XI2 (le serveur ne lui envoie plus les événements X classiques
//! équivalents) ; les axes de défilement d'un périphérique (classes `Scroll` de
//! `XIQueryDevice` : numéro de l'axe, sens, incrément d'un cran) sont lus à sa première
//! rencontre, et la variation de leur valeur devient un défilement. Les crans de molette
//! émulés pour les anciens clients (`XIPointerEmulated`) sont ignorés ; un périphérique
//! sans axe de défilement garde ses boutons 4 à 7. Sans XInput 2.1, les événements X
//! classiques restent (`input.rs`).
//!
//! Les événements arrivent comme événements génériques d'extension ; XCB insère
//! `full_sequence` après les 32 premiers octets, d'où les positions ci-dessous.
use super::{X11, connection::Connection, input::NOTCH};
use crate::{
    platform::linux::ffi::xcb::*,
    ui::{runtime::UiEvent, scene::PointF},
};
use std::collections::HashMap;

const XI_DEVICE_CHANGED: u16 = 1;
const XI_BUTTON_PRESS: u16 = 4;
const XI_BUTTON_RELEASE: u16 = 5;
const XI_MOTION: u16 = 6;
const XI_ALL_MASTER_DEVICES: u16 = 1;
const XI_ALL_DEVICES: u16 = 0;
const XI_POINTER_EMULATED: u32 = 1 << 16;
const SCROLL_CLASS: u16 = 3;
const SCROLL_VERTICAL: u16 = 1;

/// Axe de défilement : sens, incrément d'un cran, dernière valeur reçue.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) struct Axis {
    pub vertical: bool,
    pub increment: f64,
    pub last: Option<f64>,
}
/// Événement XI2 d'un pointeur (déplacement, bouton), décodé.
#[derive(PartialEq, Debug)]
pub(super) struct Device {
    pub kind: u16,
    pub device: u16,
    pub time: u32,
    pub detail: u32,
    pub window: u32,
    /// Positions en pixels (`FP1616`) dans la racine et la fenêtre.
    pub root: (f64, f64),
    pub position: (f64, f64),
    pub flags: u32,
    /// Modificateurs effectifs et groupe XKB effectif.
    pub mods: u32,
    pub group: u8,
    /// Valeurs des axes présents : numéro, valeur.
    pub valuators: Vec<(u16, f64)>,
}
fn u16_at(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_ne_bytes(bytes.get(at..at + 2)?.try_into().ok()?))
}
fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_ne_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}
fn fp1616(bytes: &[u8], at: usize) -> Option<f64> {
    Some(f64::from(u32_at(bytes, at)? as i32) / 65536.)
}
fn fp3232(bytes: &[u8], at: usize) -> Option<f64> {
    let integral = u32_at(bytes, at)? as i32;
    let fraction = u32_at(bytes, at + 4)?;
    Some(f64::from(integral) + f64::from(fraction) / 4_294_967_296.)
}
/// Décode un événement de pointeur XI2 (`xcb_input_button_press_event_t` et ses tableaux :
/// boutons, masque des axes, valeurs `FP3232`).
pub(super) fn device_event(bytes: &[u8]) -> Option<Device> {
    let buttons = usize::from(u16_at(bytes, 52)?);
    let masks = usize::from(u16_at(bytes, 54)?);
    let mask_at = 84 + buttons * 4;
    let mut values_at = mask_at + masks * 4;
    let mut valuators = Vec::new();
    for word in 0..masks {
        let mask = u32_at(bytes, mask_at + word * 4)?;
        for bit in 0..32 {
            if mask & (1 << bit) != 0 {
                valuators.push(((word * 32 + bit) as u16, fp3232(bytes, values_at)?));
                values_at += 8;
            }
        }
    }
    Some(Device {
        kind: u16_at(bytes, 8)?,
        device: u16_at(bytes, 10)?,
        time: u32_at(bytes, 12)?,
        detail: u32_at(bytes, 16)?,
        window: u32_at(bytes, 24)?,
        root: (fp1616(bytes, 36)?, fp1616(bytes, 40)?),
        position: (fp1616(bytes, 44)?, fp1616(bytes, 48)?),
        flags: u32_at(bytes, 60)?,
        mods: u32_at(bytes, 76)?,
        group: *bytes.get(83)?,
        valuators,
    })
}
/// Axes de défilement de chaque périphérique d'une réponse `XIQueryDevice` (en-tête de
/// 32 octets, puis `XIDeviceInfo` : nom aligné sur 4 octets, classes de `len` mots).
pub(super) fn scroll_axes(reply: &[u8]) -> HashMap<u16, HashMap<u16, Axis>> {
    let mut devices = HashMap::new();
    let count = u16_at(reply, 8).unwrap_or(0);
    let mut at = 32;
    for _ in 0..count {
        let (Some(id), Some(classes), Some(name)) = (
            u16_at(reply, at),
            u16_at(reply, at + 6),
            u16_at(reply, at + 8),
        ) else {
            break;
        };
        at += 12 + usize::from(name).next_multiple_of(4);
        let mut axes = HashMap::new();
        for _ in 0..classes {
            let (Some(kind), Some(len)) = (u16_at(reply, at), u16_at(reply, at + 2)) else {
                return devices;
            };
            if kind == SCROLL_CLASS
                && let (Some(number), Some(direction), Some(increment)) = (
                    u16_at(reply, at + 6),
                    u16_at(reply, at + 8),
                    fp3232(reply, at + 16),
                )
                && increment != 0.
            {
                axes.insert(
                    number,
                    Axis {
                        vertical: direction == SCROLL_VERTICAL,
                        increment,
                        last: None,
                    },
                );
            }
            at += usize::from(len.max(1)) * 4;
        }
        devices.insert(id, axes);
    }
    devices
}
/// État XInput 2 de la boucle : code de l'extension et axes de défilement.
pub(super) struct XInput {
    opcode: u8,
    /// Axes de chaque périphérique, lus à sa première rencontre.
    devices: HashMap<u16, HashMap<u16, Axis>>,
}
impl XInput {
    /// XInput 2.1 au moins (défilement fin) ; `None` sinon, ou avec `KARBON_XINPUT=0`
    /// (événements classiques, pour un serveur qui gère mal XInput 2).
    pub fn new(connection: &Connection) -> Option<Self> {
        if std::env::var("KARBON_XINPUT").is_ok_and(|v| v == "0") {
            return None;
        }
        let lib = xinput()?;
        let xcb = connection.xcb;
        // SAFETY : connexion valide ; clé de l'extension ; réponse libérée ici.
        unsafe {
            let data = (xcb.xcb_get_extension_data)(connection.raw, lib.id).as_ref()?;
            if data.present == 0 {
                return None;
            }
            let cookie = (lib.fns.xcb_input_xi_query_version)(connection.raw, 2, 2);
            let reply = (lib.fns.xcb_input_xi_query_version_reply)(
                connection.raw,
                cookie,
                std::ptr::null_mut(),
            );
            let version = reply.as_ref().map(|r| (r.major_version, r.minor_version));
            libc::free(reply.cast());
            let (major, minor) = version?;
            (major > 2 || (major == 2 && minor >= 1)).then_some(Self {
                opcode: data.major_opcode,
                devices: HashMap::new(),
            })
        }
    }
    /// Demande en XI2 les déplacements et les boutons des pointeurs maîtres sur `window`.
    pub fn select(&self, connection: &Connection, window: u32) {
        let Some(lib) = xinput() else {
            return;
        };
        let mask: u32 =
            1 << XI_DEVICE_CHANGED | 1 << XI_BUTTON_PRESS | 1 << XI_BUTTON_RELEASE | 1 << XI_MOTION;
        // `xcb_input_event_mask_t` (périphérique, longueur en mots) suivi du masque.
        let mut bytes = [0u8; 8];
        bytes[..2].copy_from_slice(&XI_ALL_MASTER_DEVICES.to_ne_bytes());
        bytes[2..4].copy_from_slice(&1u16.to_ne_bytes());
        bytes[4..].copy_from_slice(&mask.to_ne_bytes());
        // SAFETY : un masque local de 8 octets lu pendant l'appel ; fenêtre valide.
        unsafe {
            (lib.fns.xcb_input_xi_select_events)(connection.raw, window, 1, bytes.as_ptr().cast())
        };
    }
    /// Axes de défilement de `device` (demandés au serveur s'ils sont inconnus).
    fn axes(&mut self, connection: &Connection, device: u16) -> Option<&mut HashMap<u16, Axis>> {
        if !self.devices.contains_key(&device) {
            let lib = xinput()?;
            // SAFETY : connexion valide ; réponse de `32 + 4 × length` octets, libérée ici.
            let known = unsafe {
                let cookie = (lib.fns.xcb_input_xi_query_device)(connection.raw, XI_ALL_DEVICES);
                let reply = (lib.fns.xcb_input_xi_query_device_reply)(
                    connection.raw,
                    cookie,
                    std::ptr::null_mut(),
                );
                if reply.is_null() {
                    HashMap::new()
                } else {
                    let length = u32::from_ne_bytes(
                        std::slice::from_raw_parts(reply.add(4), 4)
                            .try_into()
                            .unwrap_or([0; 4]),
                    );
                    let bytes = std::slice::from_raw_parts(reply, 32 + length as usize * 4);
                    let axes = scroll_axes(bytes);
                    libc::free(reply.cast());
                    axes
                }
            };
            self.devices.extend(known);
            self.devices.entry(device).or_default();
        }
        self.devices.get_mut(&device)
    }
    /// Pointeur entré dans une fenêtre : les valeurs des axes repartent de la suivante.
    pub fn reset(&mut self) {
        for axis in self.devices.values_mut().flat_map(HashMap::values_mut) {
            axis.last = None;
        }
    }
}
impl X11 {
    /// Événement générique d'extension : XInput 2 s'il vient d'elle.
    ///
    /// # Safety
    ///
    /// `event` est un événement générique valide rendu par XCB (`length` mots après les
    /// 32 premiers octets et `full_sequence`).
    pub(super) unsafe fn generic(&mut self, event: *mut xcb_generic_event_t) {
        let Some(opcode) = self.xinput.as_ref().map(|x| x.opcode) else {
            return;
        };
        // SAFETY : événement d'au moins 36 octets, et `length` mots de plus.
        let bytes = unsafe {
            let header = std::slice::from_raw_parts(event.cast::<u8>(), 36);
            if header[1] != opcode {
                return;
            }
            let length = u32_at(header, 4).unwrap_or(0) as usize;
            std::slice::from_raw_parts(event.cast::<u8>(), 36 + length * 4)
        };
        let kind = u16_at(bytes, 8).unwrap_or(0);
        if kind == XI_DEVICE_CHANGED {
            if let (Some(xinput), Some(device)) = (&mut self.xinput, u16_at(bytes, 10)) {
                xinput.devices.remove(&device);
            }
            return;
        }
        let Some(e) = device_event(bytes) else {
            return;
        };
        // Événement classique équivalent, pour les chemins communs (`input.rs`).
        let pixel = |v: f64| v.round().clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16;
        let core = xcb_button_press_event_t {
            response_type: 0,
            detail: e.detail.min(255) as u8,
            sequence: 0,
            time: e.time,
            root: self.connection.root,
            event: e.window,
            child: 0,
            root_x: pixel(e.root.0),
            root_y: pixel(e.root.1),
            event_x: pixel(e.position.0),
            event_y: pixel(e.position.1),
            state: (e.mods & 0xff) as u16 | (u16::from(e.group & 3) << 13),
            same_screen: 1,
            pad0: 0,
        };
        match kind {
            XI_BUTTON_PRESS | XI_BUTTON_RELEASE => {
                // Crans émulés d'un défilement fin, déjà rendu par les axes.
                if (4..=7).contains(&e.detail) && e.flags & XI_POINTER_EMULATED != 0 {
                    return;
                }
                self.button(&core, kind == XI_BUTTON_PRESS);
            }
            XI_MOTION => {
                self.motion(&core);
                self.smooth_scroll(&e, core.state);
            }
            _ => {}
        }
    }
    /// Variations des axes de défilement de `e`, en pixels logiques.
    fn smooth_scroll(&mut self, e: &Device, state: u16) {
        let connection = &self.connection;
        let Some(axes) = self
            .xinput
            .as_mut()
            .and_then(|x| x.axes(connection, e.device))
        else {
            return;
        };
        let (mut x, mut y) = (0f32, 0f32);
        for &(number, value) in &e.valuators {
            let Some(axis) = axes.get_mut(&number) else {
                continue;
            };
            if let Some(last) = axis.last {
                let notches = ((value - last) / axis.increment) as f32;
                if axis.vertical {
                    y += notches * NOTCH[0];
                } else {
                    x += notches * NOTCH[1];
                }
            }
            axis.last = Some(value);
        }
        let shift = state & XCB_MOD_MASK_SHIFT != 0;
        let Some((x, y)) = crate::platform::linux::shell::scroll::delta(x, y, shift) else {
            return;
        };
        let Some(id) = self
            .windows
            .iter()
            .find(|w| w.window == e.window)
            .map(|w| w.session.id)
        else {
            return;
        };
        let position: PointF = self.position;
        self.send(
            id,
            UiEvent::Scroll {
                position,
                delta_x: x,
                delta_y: y,
            },
        );
    }
}
