//! Connexion au serveur X (XCB) : écran par défaut, visuels (celui de la racine, et un
//! visuel ARGB 32 bits pour les fenêtres transparentes), atomes du gestionnaire de
//! fenêtres (ICCCM, EWMH, Motif) et échelle du bureau (`Xft.dpi` des ressources X, 96 ppp
//! pour 1x, comme GTK et Qt).
use crate::platform::linux::ffi::xcb::*;
use std::ffi::c_void;

/// Visuel d'une fenêtre : identifiant et profondeur.
#[derive(Clone, Copy, Debug)]
pub(super) struct Visual {
    pub id: u32,
    pub depth: u8,
}
/// Atomes employés par le backend.
#[derive(Clone, Copy, Default, Debug)]
pub(super) struct Atoms {
    pub wm_protocols: u32,
    pub wm_delete_window: u32,
    pub net_wm_ping: u32,
    pub net_wm_name: u32,
    pub utf8_string: u32,
    pub net_wm_pid: u32,
    /// Hôte du processus : EWMH ne donne un sens à `_NET_WM_PID` qu'avec lui.
    pub wm_client_machine: u32,
    pub motif_wm_hints: u32,
    pub net_wm_state: u32,
    pub net_wm_state_maximized_vert: u32,
    pub net_wm_state_maximized_horz: u32,
    pub net_wm_state_fullscreen: u32,
    pub net_wm_state_hidden: u32,
    pub wm_change_state: u32,
    pub net_wm_moveresize: u32,
    pub xkb_rules_names: u32,
    pub clipboard: u32,
    pub targets: u32,
    pub text: u32,
    pub incr: u32,
    /// Propriété de la fenêtre du presse-papiers où arrive le texte demandé.
    pub karbon_clipboard: u32,
    /// Premier plan, activation et zone de travail (EWMH, `x11/window/change.rs`).
    pub net_wm_state_above: u32,
    pub net_active_window: u32,
    pub net_workarea: u32,
    /// Menu de la fenêtre du gestionnaire (extension de GTK, mutter et KWin).
    pub gtk_show_window_menu: u32,
}
pub(super) struct Connection {
    pub raw: *mut xcb_connection_t,
    pub xcb: &'static Xcb,
    /// Écran par défaut (description possédée par XCB, valide tant que la connexion).
    pub screen: *const xcb_screen_t,
    pub root: u32,
    /// Visuel de la racine (fenêtres opaques).
    pub opaque: Visual,
    /// Visuel ARGB 32 bits (fenêtres transparentes), s'il existe.
    pub argb: Option<Visual>,
    pub atoms: Atoms,
    /// Échelle du bureau (pixels par unité logique).
    pub scale: f32,
    /// Premier numéro des événements XKB, une fois l'extension initialisée et ses
    /// événements de disposition demandés (`server_keyboard`).
    pub xkb_event: std::cell::Cell<Option<u8>>,
    /// Répétition détectable accordée par le serveur : une touche répétée n'envoie que des
    /// appuis (sinon, relâchement et appui jumeaux au même instant).
    pub detectable_repeat: std::cell::Cell<bool>,
}
fn unsupported(reason: &'static str) -> crate::Error {
    crate::Error::Unsupported { reason }
}
impl Connection {
    /// Connexion au serveur de `DISPLAY`.
    pub fn connect() -> crate::Result<Self> {
        let xcb = xcb().ok_or(unsupported("libxcb.so.1 is not installed"))?;
        let mut screen_number = 0;
        // SAFETY : affichage par défaut (`DISPLAY`) ; sortie locale.
        let raw = unsafe { (xcb.xcb_connect)(std::ptr::null(), &mut screen_number) };
        // SAFETY : `xcb_connect` rend toujours une connexion, éventuellement en erreur.
        if unsafe { (xcb.xcb_connection_has_error)(raw) } != 0 {
            // SAFETY : connexion rendue par `xcb_connect`, libérée une fois.
            unsafe { (xcb.xcb_disconnect)(raw) };
            return Err(unsupported("cannot connect to the X server (DISPLAY)"));
        }
        let mut this = Self {
            raw,
            xcb,
            screen: std::ptr::null(),
            root: 0,
            opaque: Visual { id: 0, depth: 0 },
            argb: None,
            atoms: Atoms::default(),
            scale: 1.,
            xkb_event: std::cell::Cell::new(None),
            detectable_repeat: std::cell::Cell::new(false),
        };
        // SAFETY : connexion valide ; description du serveur possédée par XCB.
        let setup = unsafe { &*(xcb.xcb_get_setup)(raw) };
        if setup.image_byte_order != XCB_IMAGE_ORDER_LSB_FIRST {
            return Err(unsupported("X server with big-endian images"));
        }
        // SAFETY : itérateur sur les écrans de la description, parcourue en lecture.
        let screen = unsafe {
            let mut screens = (xcb.xcb_setup_roots_iterator)(setup);
            for _ in 0..screen_number.max(0) {
                if screens.rem <= 1 {
                    break;
                }
                (xcb.xcb_screen_next)(&mut screens);
            }
            screens
                .data
                .as_ref()
                .ok_or(unsupported("X server without screen"))?
        };
        this.screen = screen;
        this.root = screen.root;
        this.opaque = Visual {
            id: screen.root_visual,
            depth: screen.root_depth,
        };
        this.argb = this.argb_visual(screen);
        if !this.true_color(screen, this.opaque.id) {
            return Err(unsupported("X root visual is not 24-bit TrueColor"));
        }
        this.atoms = this.atoms();
        this.scale = this.desktop_scale();
        Ok(this)
    }
    /// Visuel TrueColor de profondeur 32 aux masques BGRA, s'il existe.
    fn argb_visual(&self, screen: &xcb_screen_t) -> Option<Visual> {
        let xcb = self.xcb;
        // SAFETY : itérateurs sur la description du serveur, parcourue en lecture.
        unsafe {
            let mut depths = (xcb.xcb_screen_allowed_depths_iterator)(screen);
            while depths.rem > 0 {
                let depth = &*depths.data;
                if depth.depth == 32 {
                    let mut visuals = (xcb.xcb_depth_visuals_iterator)(depth);
                    while visuals.rem > 0 {
                        let v = &*visuals.data;
                        if v.class == XCB_VISUAL_CLASS_TRUE_COLOR && v.red_mask == 0xff_0000 {
                            return Some(Visual {
                                id: v.visual_id,
                                depth: 32,
                            });
                        }
                        (xcb.xcb_visualtype_next)(&mut visuals);
                    }
                }
                (xcb.xcb_depth_next)(&mut depths);
            }
        }
        None
    }
    /// `visual` est TrueColor 24 bits aux masques BGRA (pixels du moteur sans conversion).
    fn true_color(&self, screen: &xcb_screen_t, visual: u32) -> bool {
        let xcb = self.xcb;
        // SAFETY : idem.
        unsafe {
            let mut depths = (xcb.xcb_screen_allowed_depths_iterator)(screen);
            while depths.rem > 0 {
                let depth = &*depths.data;
                let mut visuals = (xcb.xcb_depth_visuals_iterator)(depth);
                while visuals.rem > 0 {
                    let v = &*visuals.data;
                    if v.visual_id == visual {
                        return v.class == XCB_VISUAL_CLASS_TRUE_COLOR
                            && depth.depth >= 24
                            && (v.red_mask, v.green_mask, v.blue_mask)
                                == (0xff_0000, 0xff00, 0xff);
                    }
                    (xcb.xcb_visualtype_next)(&mut visuals);
                }
                (xcb.xcb_depth_next)(&mut depths);
            }
        }
        false
    }
    /// Atomes du backend, demandés ensemble puis lus.
    /// Atome de nom `name` (créé s'il n'existe pas).
    #[cfg(test)]
    pub(super) fn intern(&self, name: &str) -> u32 {
        let xcb = self.xcb;
        // SAFETY : nom local lu pendant l'appel ; réponse libérée ici.
        unsafe {
            let cookie =
                (xcb.xcb_intern_atom)(self.raw, 0, name.len() as u16, name.as_ptr().cast());
            let reply = (xcb.xcb_intern_atom_reply)(self.raw, cookie, std::ptr::null_mut());
            let atom = reply.as_ref().map_or(0, |r| r.atom);
            libc::free(reply.cast());
            atom
        }
    }
    fn atoms(&self) -> Atoms {
        let names = [
            "WM_PROTOCOLS",
            "WM_DELETE_WINDOW",
            "_NET_WM_PING",
            "_NET_WM_NAME",
            "UTF8_STRING",
            "_NET_WM_PID",
            "WM_CLIENT_MACHINE",
            "_MOTIF_WM_HINTS",
            "_NET_WM_STATE",
            "_NET_WM_STATE_MAXIMIZED_VERT",
            "_NET_WM_STATE_MAXIMIZED_HORZ",
            "_NET_WM_STATE_FULLSCREEN",
            "_NET_WM_STATE_HIDDEN",
            "WM_CHANGE_STATE",
            "_NET_WM_MOVERESIZE",
            "_XKB_RULES_NAMES",
            "CLIPBOARD",
            "TARGETS",
            "TEXT",
            "INCR",
            "KARBON_CLIPBOARD",
            "_NET_WM_STATE_ABOVE",
            "_NET_ACTIVE_WINDOW",
            "_NET_WORKAREA",
            "_GTK_SHOW_WINDOW_MENU",
        ];
        let xcb = self.xcb;
        // SAFETY : noms locaux lus pendant l'appel.
        let cookies = names.map(|name| unsafe {
            (xcb.xcb_intern_atom)(self.raw, 0, name.len() as u16, name.as_ptr().cast())
        });
        let values = cookies.map(|cookie| {
            // SAFETY : réponse à la requête ci-dessus, libérée ici.
            unsafe {
                let reply = (xcb.xcb_intern_atom_reply)(self.raw, cookie, std::ptr::null_mut());
                let atom = reply.as_ref().map_or(0, |r| r.atom);
                libc::free(reply.cast());
                atom
            }
        });
        let [
            wm_protocols,
            wm_delete_window,
            net_wm_ping,
            net_wm_name,
            utf8_string,
            net_wm_pid,
            wm_client_machine,
            motif_wm_hints,
            net_wm_state,
            net_wm_state_maximized_vert,
            net_wm_state_maximized_horz,
            net_wm_state_fullscreen,
            net_wm_state_hidden,
            wm_change_state,
            net_wm_moveresize,
            xkb_rules_names,
            clipboard,
            targets,
            text,
            incr,
            karbon_clipboard,
            net_wm_state_above,
            net_active_window,
            net_workarea,
            gtk_show_window_menu,
        ] = values;
        Atoms {
            wm_protocols,
            wm_delete_window,
            net_wm_ping,
            net_wm_name,
            utf8_string,
            net_wm_pid,
            wm_client_machine,
            motif_wm_hints,
            net_wm_state,
            net_wm_state_maximized_vert,
            net_wm_state_maximized_horz,
            net_wm_state_fullscreen,
            net_wm_state_hidden,
            wm_change_state,
            net_wm_moveresize,
            xkb_rules_names,
            clipboard,
            targets,
            text,
            incr,
            karbon_clipboard,
            net_wm_state_above,
            net_active_window,
            net_workarea,
            gtk_show_window_menu,
        }
    }
    /// Disposition du clavier du serveur : lue telle quelle par l'extension XKB
    /// (xkbcommon-x11), sinon compilée d'après ses noms (`_XKB_RULES_NAMES` de la racine).
    /// Sous XWayland, ces noms restent ceux du lancement (`us`) quand le compositeur
    /// fournit la disposition : seule la lecture par XKB est juste.
    pub fn keyboard(&self) -> Option<crate::platform::linux::ffi::xkb::Keyboard> {
        let locale = crate::platform::linux::shell::keyboard::locale();
        self.server_keyboard(&locale)
            .or_else(|| self.named_keyboard(&locale))
    }
    /// Disposition du clavier principal, par l'extension XKB 1.0.
    pub(super) fn server_keyboard(
        &self,
        locale: &std::ffi::CStr,
    ) -> Option<crate::platform::linux::ffi::xkb::Keyboard> {
        let xkb = crate::platform::linux::ffi::xcb::xkb_x11()?;
        // SAFETY : connexion valide ; sorties locales ; disposition rendue à xkbcommon
        // par `Keyboard` (contexte fourni par lui, vivant pendant l'appel).
        unsafe {
            let (mut major, mut minor, mut event, mut error) = (0, 0, 0, 0);
            if (xkb.xkb_x11_setup_xkb_extension)(
                self.raw, 1, 0, 0, &mut major, &mut minor, &mut event, &mut error,
            ) != 1
            {
                return None;
            }
            let device = (xkb.xkb_x11_get_core_keyboard_device_id)(self.raw);
            if device < 0 {
                return None;
            }
            // Client XKB : le serveur ne lui envoie plus `MappingNotify` ; les changements
            // de disposition arrivent par ces événements (tous leurs détails, sans liste).
            if self.xkb_event.get().is_none()
                && let Some(events) = xcb_xkb()
            {
                let which = XCB_XKB_EVENT_TYPE_NEW_KEYBOARD_NOTIFY | XCB_XKB_EVENT_TYPE_MAP_NOTIFY;
                (events.xcb_xkb_select_events)(
                    self.raw,
                    device as u16,
                    which,
                    0,
                    which,
                    XCB_XKB_MAP_PART_ALL,
                    XCB_XKB_MAP_PART_ALL,
                    std::ptr::null(),
                );
                self.xkb_event.set(Some(event));
                let flag = XCB_XKB_PER_CLIENT_FLAG_DETECTABLE_AUTO_REPEAT;
                let cookie = (events.xcb_xkb_per_client_flags)(
                    self.raw,
                    XCB_XKB_ID_USE_CORE_KBD,
                    flag,
                    flag,
                    0,
                    0,
                    0,
                );
                let reply =
                    (events.xcb_xkb_per_client_flags_reply)(self.raw, cookie, std::ptr::null_mut());
                self.detectable_repeat
                    .set(reply.as_ref().is_some_and(|r| r.value & flag != 0));
                libc::free(reply.cast());
            }
            crate::platform::linux::ffi::xkb::Keyboard::from_context(locale, |context| {
                (xkb.xkb_x11_keymap_new_from_device)(context, self.raw, device, 0)
            })
            .ok()
        }
    }
    /// Disposition compilée d'après les noms du serveur (règles, modèle, dispositions,
    /// variantes, options) ; disposition par défaut sans cette propriété.
    fn named_keyboard(
        &self,
        locale: &std::ffi::CStr,
    ) -> Option<crate::platform::linux::ffi::xkb::Keyboard> {
        let names = self.property(self.root, self.atoms.xkb_rules_names);
        let mut parts: Vec<std::ffi::CString> = names
            .split(|b| *b == 0)
            .take(5)
            .map(|p| std::ffi::CString::new(p).unwrap_or_default())
            .collect();
        parts.resize(5, std::ffi::CString::default());
        let names = std::array::from_fn(|i| parts[i].as_c_str());
        crate::platform::linux::ffi::xkb::Keyboard::from_names(names, locale).ok()
    }
    /// Valeur de la propriété `property` de `window` (atomes ou octets), au plus 1 Mo.
    pub fn property(&self, window: u32, property: u32) -> Vec<u8> {
        self.read_property(window, property, false).1
    }
    /// Type et valeur de la propriété `property` de `window`, supprimée après lecture.
    pub fn take_property(&self, window: u32, property: u32) -> (u32, Vec<u8>) {
        self.read_property(window, property, true)
    }
    /// Lue par tranches de 1 Mo jusqu'au bout (au plus `MAX_PROPERTY`) ; avec `delete`,
    /// le serveur la supprime à la lecture de la dernière tranche.
    fn read_property(&self, window: u32, property: u32, delete: bool) -> (u32, Vec<u8>) {
        /// Tranche lue par requête, en mots de 32 bits (1 Mo).
        const SLICE: u32 = 1 << 18;
        /// Taille au plus d'une propriété lue (presse-papiers compris).
        const MAX_PROPERTY: usize = 64 << 20;
        let xcb = self.xcb;
        let (mut kind, mut bytes) = (0, Vec::new());
        let mut offset = 0u32;
        loop {
            // SAFETY : requête puis réponse libérée ici ; valeur de `length` octets.
            let after = unsafe {
                let cookie = (xcb.xcb_get_property)(
                    self.raw,
                    u8::from(delete),
                    window,
                    property,
                    0,
                    offset,
                    SLICE,
                );
                let reply = (xcb.xcb_get_property_reply)(self.raw, cookie, std::ptr::null_mut());
                if reply.is_null() {
                    return (kind, bytes);
                }
                kind = (*reply).type_;
                let length = (xcb.xcb_get_property_value_length)(reply).max(0) as usize;
                let value = (xcb.xcb_get_property_value)(reply).cast::<u8>();
                if !value.is_null() {
                    bytes.extend_from_slice(std::slice::from_raw_parts(value, length));
                }
                let after = (*reply).bytes_after;
                libc::free(reply.cast());
                after
            };
            if after == 0 || bytes.len() >= MAX_PROPERTY {
                return (kind, bytes);
            }
            offset += SLICE;
        }
    }
    /// Échelle du bureau : `Xft.dpi` / 96 (ressources X de la racine), au moins 1.
    fn desktop_scale(&self) -> f32 {
        let resources = self.property(self.root, XCB_ATOM_RESOURCE_MANAGER);
        let text = String::from_utf8_lossy(&resources);
        dpi_scale(&text).unwrap_or(1.)
    }
    pub fn flush(&self) {
        // SAFETY : connexion valide.
        unsafe { (self.xcb.xcb_flush)(self.raw) };
    }
    /// Connexion vue par Vulkan (`VkXcbSurfaceCreateInfoKHR`).
    pub fn as_ptr(&self) -> *mut c_void {
        self.raw
    }
}
impl Drop for Connection {
    fn drop(&mut self) {
        // SAFETY : connexion possédée, fermée une fois (après les fenêtres).
        unsafe { (self.xcb.xcb_disconnect)(self.raw) };
    }
}
/// Échelle de la ligne `Xft.dpi` des ressources X (`dpi / 96`, bornée à 1–4).
pub(super) fn dpi_scale(resources: &str) -> Option<f32> {
    resources.lines().find_map(|line| {
        let value = line.strip_prefix("Xft.dpi:")?.trim();
        let dpi = value.parse::<f32>().ok()?;
        (dpi.is_finite() && dpi > 0.).then(|| (dpi / 96.).clamp(1., 4.))
    })
}
