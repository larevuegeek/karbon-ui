//! Entrées X11 vers les fenêtres : pointeur (barre de titre et bords du moteur, zones de
//! déplacement de l'application, clics, menu contextuel, molette), clavier et curseurs.
//!
//! - Clavier : disposition compilée par xkbcommon d'après les noms du serveur
//!   (`Connection::keyboard`), modificateurs et disposition active lus dans chaque
//!   événement (`state` : modificateurs réels dans les huit bits bas, groupe XKB dans les
//!   bits 13 et 14), répétitions faites par le serveur (relâchement et appui au même
//!   instant : une répétition).
//! - Déplacement et redimensionnement par le gestionnaire (`_NET_WM_MOVERESIZE`, après
//!   avoir rendu la saisie implicite du pointeur).
//! - Curseurs du thème par `libxcb-cursor` (curseur par défaut du serveur sans elle).
use super::{X11, connection::Connection};
use crate::{
    platform::linux::{
        ffi::xcb::*,
        shell::chrome::{Edge, Press, cursor_names},
    },
    ui::{runtime::UiEvent, scene::PointF},
};
use std::{collections::HashMap, ffi::c_void, time::Instant};

/// Événements d'entrée demandés pour chaque fenêtre.
pub(super) const EVENTS: u32 = XCB_EVENT_MASK_KEY_PRESS
    | XCB_EVENT_MASK_KEY_RELEASE
    | XCB_EVENT_MASK_BUTTON_PRESS
    | XCB_EVENT_MASK_BUTTON_RELEASE
    | XCB_EVENT_MASK_ENTER_WINDOW
    | XCB_EVENT_MASK_LEAVE_WINDOW
    | XCB_EVENT_MASK_POINTER_MOTION;
/// Défilement d'un cran de molette, commun aux boucles (`shell/scroll.rs`).
pub(super) use crate::platform::linux::shell::scroll::NOTCH;
/// Différence entre un code de touche X (XKB) et un code evdev.
const KEYCODE_OFFSET: u32 = 8;
/// `_NET_WM_MOVERESIZE` : déplacement.
const MOVE: u32 = 8;

/// Direction `_NET_WM_MOVERESIZE` d'un bord.
fn direction(edge: Edge) -> u32 {
    match edge {
        Edge::TopLeft => 0,
        Edge::Top => 1,
        Edge::TopRight => 2,
        Edge::Right => 3,
        Edge::BottomRight => 4,
        Edge::Bottom => 5,
        Edge::BottomLeft => 6,
        Edge::Left => 7,
    }
}
/// Curseurs du thème chargés, et celui affiché par chaque fenêtre.
pub(super) struct Cursors {
    context: *mut c_void,
    loaded: HashMap<&'static str, u32>,
    shown: HashMap<u32, u32>,
}
impl Cursors {
    pub fn new(connection: &Connection) -> Self {
        let mut context = std::ptr::null_mut();
        if let Some(lib) = xcb_cursor() {
            // SAFETY : connexion et écran valides ; contexte libéré par `close`.
            let failed = unsafe {
                (lib.xcb_cursor_context_new)(connection.raw, connection.screen, &mut context)
            };
            if failed < 0 {
                context = std::ptr::null_mut();
            }
        }
        Self {
            context,
            loaded: HashMap::new(),
            shown: HashMap::new(),
        }
    }
    /// Premier curseur du thème trouvé parmi `names` (0 : curseur par défaut).
    fn load(&mut self, names: &[&'static str]) -> u32 {
        let Some(lib) = xcb_cursor().filter(|_| !self.context.is_null()) else {
            return 0;
        };
        for name in names {
            let cursor = *self.loaded.entry(name).or_insert_with(|| {
                let text = std::ffi::CString::new(*name).unwrap_or_default();
                // SAFETY : contexte valide ; nom terminé par un octet nul.
                unsafe { (lib.xcb_cursor_load_cursor)(self.context, text.as_ptr()) }
            });
            if cursor != 0 {
                return cursor;
            }
        }
        0
    }
    /// Affiche sur `window` le premier curseur trouvé parmi `names`.
    pub fn show(&mut self, connection: &Connection, window: u32, names: &[&'static str]) {
        let cursor = self.load(names);
        if self.shown.get(&window) == Some(&cursor) {
            return;
        }
        self.shown.insert(window, cursor);
        // SAFETY : fenêtre valide ; valeur locale lue pendant l'appel.
        unsafe {
            (connection.xcb.xcb_change_window_attributes)(
                connection.raw,
                window,
                XCB_CW_CURSOR,
                (&raw const cursor).cast(),
            )
        };
    }
    /// Fenêtre fermée : son curseur est oublié.
    pub fn forget(&mut self, window: u32) {
        self.shown.remove(&window);
    }
    pub fn close(&mut self, connection: &Connection) {
        for (_, cursor) in self.loaded.drain() {
            if cursor != 0 {
                // SAFETY : curseur chargé par ce contexte, libéré une fois.
                unsafe { (connection.xcb.xcb_free_cursor)(connection.raw, cursor) };
            }
        }
        if let Some(lib) = xcb_cursor().filter(|_| !self.context.is_null()) {
            // SAFETY : contexte créé par `new`, libéré une fois.
            unsafe { (lib.xcb_cursor_context_free)(self.context) };
            self.context = std::ptr::null_mut();
        }
    }
}
impl X11 {
    /// Position logique d'un événement de la fenêtre `window`.
    fn logical(&self, window: u32, x: i16, y: i16) -> PointF {
        let scale = self
            .windows
            .iter()
            .find(|w| w.window == window)
            .map_or(1., |w| w.session.scale);
        PointF {
            x: f32::from(x) / scale,
            y: f32::from(y) / scale,
        }
    }
    pub(super) fn entered(&mut self, event: &xcb_enter_notify_event_t) {
        if let Some(xinput) = &mut self.xinput {
            xinput.reset();
        }
        let window = self.windows.iter().find(|w| w.window == event.event);
        self.pointer_window = window.map(|w| w.session.id);
        self.position = self.logical(event.event, event.event_x, event.event_y);
        self.moved();
    }
    pub(super) fn left(&mut self, event: &xcb_enter_notify_event_t) {
        let Some(id) = self.pointer_window else {
            return;
        };
        if self
            .windows
            .iter()
            .any(|w| w.window == event.event && w.session.id == id)
        {
            self.pointer_window = None;
            if let Some(w) = self.windows.iter_mut().find(|w| w.session.id == id) {
                w.session.leave();
            }
            self.send(id, UiEvent::PointerLeave);
        }
    }
    pub(super) fn motion(&mut self, event: &xcb_button_press_event_t) {
        self.time = event.time;
        self.position = self.logical(event.event, event.event_x, event.event_y);
        if self.pointer_window.is_none() {
            self.pointer_window = self
                .windows
                .iter()
                .find(|w| w.window == event.event)
                .map(|w| w.session.id);
        }
        self.moved();
    }
    /// Le pointeur a bougé : survol des boutons et des bords, puis le contenu.
    fn moved(&mut self) {
        let Some(id) = self.pointer_window else {
            return;
        };
        let position = self.position;
        if let Some(w) = self.windows.iter_mut().find(|w| w.session.id == id) {
            w.session.hover(position);
        }
        self.send(id, UiEvent::PointerMove(position));
    }
    pub(super) fn button(&mut self, event: &xcb_button_press_event_t, pressed: bool) {
        let Some(id) = self
            .windows
            .iter()
            .find(|w| w.window == event.event)
            .map(|w| w.session.id)
        else {
            return;
        };
        let position = self.logical(event.event, event.event_x, event.event_y);
        self.position = position;
        self.time = event.time;
        let shift = event.state & XCB_MOD_MASK_SHIFT != 0;
        match (event.detail, pressed) {
            (1, true) => {
                let now = Instant::now();
                let Some(press) = self.with(id, |w, _| w.session.press(position, now)) else {
                    return;
                };
                match press {
                    Press::Resize(edge) => self.moveresize(event, direction(edge)),
                    Press::Move => self.moveresize(event, MOVE),
                    Press::Maximize => {
                        if let Some(w) = self.windows.iter().find(|w| w.session.id == id) {
                            w.set_maximized(&self.connection, !w.session.maximized);
                        }
                    }
                    Press::Button => {}
                    Press::Content => {
                        // Un clic valide la composition en cours, comme sous Wayland.
                        self.finish_composition();
                        // Position à jour avant l'appui (un menu a pu apparaître sous un
                        // pointeur immobile).
                        self.send(id, UiEvent::PointerMove(position));
                        let down = if shift {
                            UiEvent::PointerDownExtended(position)
                        } else {
                            UiEvent::PointerDown(position)
                        };
                        self.send(id, down);
                    }
                }
            }
            (1, false) => {
                let released = self
                    .windows
                    .iter_mut()
                    .find(|w| w.session.id == id)
                    .and_then(|w| w.session.release(position));
                match released {
                    Some(Some(action)) => self.action(id, action),
                    Some(None) => {}
                    None => self.send(id, UiEvent::PointerUp(position)),
                }
            }
            (3, true) => {
                let caption = self
                    .windows
                    .iter()
                    .any(|w| w.session.id == id && w.session.caption_at(position));
                if caption {
                    self.window_menu(event);
                } else {
                    self.send(id, UiEvent::ContextMenu(Some(position)));
                }
            }
            // Molette : un cran par appui (4 et 5 vertical, 6 et 7 horizontal) ; Maj fait
            // défiler horizontalement.
            (4..=7, true) => {
                let sign = if event.detail.is_multiple_of(2) {
                    -1.
                } else {
                    1.
                };
                let (x, y) = if event.detail >= 6 {
                    (sign * NOTCH[1], 0.)
                } else {
                    (0., sign * NOTCH[0])
                };
                if let Some((x, y)) = crate::platform::linux::shell::scroll::delta(x, y, shift) {
                    // Boutons 4 à 7 : crans de molette, glissement doux.
                    self.send(
                        id,
                        UiEvent::Wheel {
                            position,
                            delta_x: x,
                            delta_y: y,
                        },
                    );
                }
            }
            _ => {}
        }
    }
    /// Confie au gestionnaire le déplacement (`MOVE`) ou le redimensionnement de la
    /// fenêtre de `event`, en cours avec le bouton principal.
    fn moveresize(&mut self, event: &xcb_button_press_event_t, direction: u32) {
        let connection = &self.connection;
        // SAFETY : connexion valide ; la saisie implicite du bouton est rendue au
        // gestionnaire.
        unsafe { (connection.xcb.xcb_ungrab_pointer)(connection.raw, XCB_CURRENT_TIME) };
        let message = xcb_client_message_event_t {
            response_type: XCB_CLIENT_MESSAGE,
            format: 32,
            sequence: 0,
            window: event.event,
            type_: connection.atoms.net_wm_moveresize,
            data: [event.root_x as u32, event.root_y as u32, direction, 1, 1],
        };
        super::window::send_to_root(connection, &message);
        connection.flush();
    }
    /// Menu de la fenêtre du gestionnaire sous le pointeur (clic droit sur la barre de
    /// titre du moteur), comme GTK : sans effet si le gestionnaire ne le connaît pas.
    fn window_menu(&mut self, event: &xcb_button_press_event_t) {
        let connection = &self.connection;
        // SAFETY : connexion valide ; la saisie implicite du bouton est rendue au
        // gestionnaire, qui saisit le pointeur pour son menu.
        unsafe { (connection.xcb.xcb_ungrab_pointer)(connection.raw, XCB_CURRENT_TIME) };
        let message = xcb_client_message_event_t {
            response_type: XCB_CLIENT_MESSAGE,
            format: 32,
            sequence: 0,
            window: event.event,
            type_: connection.atoms.gtk_show_window_menu,
            data: [0, event.root_x as u32, event.root_y as u32, 0, 0],
        };
        super::window::send_to_root(connection, &message);
        connection.flush();
    }
    /// Touche enfoncée ou relâchée ; `repeat` : répétition du serveur.
    pub(super) fn key(&mut self, event: &xcb_button_press_event_t, pressed: bool, repeat: bool) {
        let Some(id) = self
            .windows
            .iter()
            .find(|w| w.window == event.event)
            .map(|w| w.session.id)
        else {
            return;
        };
        self.time = event.time;
        let state = u32::from(event.state);
        self.keys.modifiers(state & 0xff, 0, 0, (state >> 13) & 3);
        let code = u32::from(event.detail).saturating_sub(KEYCODE_OFFSET);
        // Champ de saisie : la méthode d'entrée voit la touche d'abord.
        let keysym = self.keys.keysym(code);
        if self.ime_key(id, (keysym, code, state), pressed, repeat) {
            return;
        }
        let events = if repeat {
            self.keys.repeated(code)
        } else {
            self.keys.key(code, pressed, Instant::now())
        };
        for ui in events {
            self.send(id, ui);
        }
    }
    /// Curseur de la fenêtre sous le pointeur : bord de redimensionnement, sinon celui
    /// demandé par le runtime.
    pub(super) fn update_cursor(&mut self) {
        let Some(id) = self.pointer_window else {
            return;
        };
        let position = self.position;
        let Some(w) = self.windows.iter_mut().find(|w| w.session.id == id) else {
            return;
        };
        let names = w
            .session
            .hover(position)
            .unwrap_or_else(|| cursor_names(w.session.runtime.cursor()));
        let window = w.window;
        self.cursors.show(&self.connection, window, names);
    }
}
