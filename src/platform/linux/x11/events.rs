//! Répartition des événements du serveur X : exposition, taille, affichage, états EWMH,
//! focus, protocoles du gestionnaire, sélections, pointeur et clavier.
use super::{Window, X11};
use crate::{platform::linux::ffi::xcb::*, ui::runtime::UiEvent};

impl X11 {
    /// Événements reçus du serveur, traités jusqu'au dernier. Ceux gardés pendant une
    /// lecture du presse-papiers (`deferred`) passent d'abord, puis ceux que le serveur a
    /// envoyés ensuite : une lecture déclenchée au milieu d'un tour ne réordonne rien.
    pub(super) fn process(&mut self) {
        while let Some(event) = self.next_event() {
            // SAFETY : événement valide (XCB ou gardé par `Clipboard::read`), d'au moins
            // 32 octets avec le type en tête ; libéré une fois.
            unsafe {
                self.event(event);
                libc::free(event.cast());
            }
        }
    }
    /// Prochain événement à traiter : différé, sinon lu de la connexion ; nul s'il n'y en
    /// a plus.
    fn next_event(&mut self) -> Option<*mut xcb_generic_event_t> {
        let event = match self.deferred.pop_front() {
            Some(event) => event,
            // SAFETY : connexion valide ; événement possédé par l'appelant.
            None => unsafe { (self.connection.xcb.xcb_poll_for_event)(self.connection.raw) },
        };
        (!event.is_null()).then_some(event)
    }
    /// Traite `event` selon son type.
    ///
    /// # Safety
    ///
    /// `event` est un événement valide rendu par XCB.
    unsafe fn event(&mut self, event: *mut xcb_generic_event_t) {
        // SAFETY : chaque type d'événement a la disposition de sa structure.
        unsafe {
            // Événement XKB (nouveau clavier, disposition changée) : disposition relue.
            if self.connection.xkb_event.get() == Some((*event).response_type & 0x7f) {
                self.reload_keyboard();
                return;
            }
            match (*event).response_type & 0x7f {
                XCB_EXPOSE => {
                    let e = &*event.cast::<xcb_expose_event_t>();
                    if e.count == 0
                        && let Some(w) = self.window(e.window)
                    {
                        w.exposed();
                    }
                }
                XCB_CONFIGURE_NOTIFY => {
                    let e = &*event.cast::<xcb_configure_notify_event_t>();
                    if let Some(w) = self.window(e.window) {
                        w.resized(e.width, e.height);
                    }
                }
                XCB_MAP_NOTIFY => {
                    let e = &*event.cast::<xcb_map_notify_event_t>();
                    if let Some(w) = self.window(e.window) {
                        w.mapped = true;
                        w.session.dirty = true;
                    }
                }
                XCB_UNMAP_NOTIFY => {
                    let e = &*event.cast::<xcb_map_notify_event_t>();
                    if let Some(w) = self.window(e.window) {
                        w.mapped = false;
                        w.hidden();
                    }
                }
                XCB_PROPERTY_NOTIFY => {
                    let e = &*event.cast::<xcb_property_notify_event_t>();
                    // Morceau suivant d'un texte servi par morceaux.
                    self.clipboard.property_changed(&self.connection, e);
                    if e.atom == self.connection.atoms.net_wm_state {
                        let connection = &self.connection;
                        if let Some(w) = self.windows.iter_mut().find(|w| w.window == e.window) {
                            w.states(connection);
                        }
                    }
                }
                XCB_FOCUS_IN | XCB_FOCUS_OUT => {
                    let e = &*event.cast::<xcb_focus_in_event_t>();
                    // Saisie du clavier par un autre client (Alt+Tab, capture d'écran…) :
                    // le focus n'a pas vraiment changé, comme pour GDK.
                    if matches!(e.mode, XCB_NOTIFY_MODE_GRAB | XCB_NOTIFY_MODE_UNGRAB)
                        || e.detail == XCB_NOTIFY_DETAIL_POINTER
                    {
                        return;
                    }
                    let focused = (*event).response_type & 0x7f == XCB_FOCUS_IN;
                    let Some(w) = self.window(e.event) else {
                        return;
                    };
                    if w.session.focused == focused {
                        return;
                    }
                    let id = w.session.id;
                    if focused {
                        w.session.focused = true;
                        crate::platform::linux::desktop::refresh();
                    } else {
                        // Composition validée dans le champ tant qu'il a encore le clavier.
                        self.finish_composition();
                        if let Some(w) = self.window(e.event) {
                            w.session.focused = false;
                        }
                        self.keys.reset();
                        self.send(id, UiEvent::Cancel);
                    }
                    self.send(id, UiEvent::FocusChanged(focused));
                }
                XCB_CLIENT_MESSAGE => {
                    let e = *event.cast::<xcb_client_message_event_t>();
                    self.client_message(e);
                }
                XCB_SELECTION_REQUEST => {
                    let e = &*event.cast::<xcb_selection_request_event_t>();
                    self.clipboard.request(&self.connection, e);
                }
                XCB_SELECTION_CLEAR => self.clipboard.cleared(&*event.cast()),
                // Disposition changée (réglages du bureau, `setxkbmap`) : relue du serveur,
                // la répétition en cours est gardée.
                XCB_MAPPING_NOTIFY => self.reload_keyboard(),
                XCB_GE_GENERIC => self.generic(event),
                XCB_MOTION_NOTIFY => self.motion(&*event.cast()),
                XCB_BUTTON_PRESS => self.button(&*event.cast(), true),
                XCB_BUTTON_RELEASE => self.button(&*event.cast(), false),
                XCB_ENTER_NOTIFY => self.entered(&*event.cast()),
                XCB_LEAVE_NOTIFY => self.left(&*event.cast()),
                XCB_KEY_PRESS => self.key(&*event.cast(), true, false),
                XCB_KEY_RELEASE => self.released(&*event.cast()),
                _ => {}
            }
        }
    }
    /// Disposition changée (réglages du bureau, `setxkbmap`) : relue du serveur, la
    /// répétition en cours gardée.
    fn reload_keyboard(&mut self) {
        let keyboard = self.connection.keyboard();
        self.keys.set_keyboard(keyboard, true);
    }
    /// Touche relâchée : suivie d'un appui de la même touche au même instant, c'est une
    /// répétition du serveur (l'appui est consommé ici).
    fn released(&mut self, event: &xcb_key_press_event_t) {
        // L'appui jumeau est l'événement suivant : d'abord ceux gardés par une lecture du
        // presse-papiers, sinon ceux déjà lus de la connexion (sans attendre).
        let next = match self.deferred.pop_front() {
            Some(next) => next,
            // SAFETY : connexion valide ; événement possédé, libéré ci-dessous.
            None => unsafe { (self.connection.xcb.xcb_poll_for_queued_event)(self.connection.raw) },
        };
        if next.is_null() {
            self.key(event, false, false);
            return;
        }
        // SAFETY : événement valide d'au moins 32 octets ; disposition d'une touche.
        unsafe {
            let press = &*next.cast::<xcb_key_press_event_t>();
            if press.response_type & 0x7f == XCB_KEY_PRESS
                && press.detail == event.detail
                && press.time == event.time
            {
                self.key(press, true, true);
            } else {
                self.key(event, false, false);
                self.event(next);
            }
            libc::free(next.cast());
        }
    }
    fn window(&mut self, window: u32) -> Option<&mut Window> {
        self.windows.iter_mut().find(|w| w.window == window)
    }
    /// Protocoles du gestionnaire de fenêtres : fermeture demandée, `_NET_WM_PING`.
    fn client_message(&mut self, mut event: xcb_client_message_event_t) {
        let atoms = self.connection.atoms;
        if event.type_ != atoms.wm_protocols {
            return;
        }
        if event.data[0] == atoms.wm_delete_window {
            if let Some(w) = self.window(event.window) {
                let id = w.session.id;
                self.closed.push(id);
            }
        } else if event.data[0] == atoms.net_wm_ping {
            // La réponse est le même message, renvoyé à la racine.
            event.window = self.connection.root;
            super::window::send_to_root(&self.connection, &event);
        }
    }
}
