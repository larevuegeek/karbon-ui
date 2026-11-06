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
        }
    }
    /// Disposition du clavier du serveur, compilée par xkbcommon d'après ses noms
    /// (`_XKB_RULES_NAMES` de la racine : règles, modèle, dispositions, variantes,
    /// options) ; disposition par défaut sans cette propriété.
    pub fn keyboard(&self) -> Option<crate::platform::linux::ffi::xkb::Keyboard> {
        let names = self.property(self.root, self.atoms.xkb_rules_names);
        let mut parts: Vec<std::ffi::CString> = names
            .split(|b| *b == 0)
            .take(5)
            .map(|p| std::ffi::CString::new(p).unwrap_or_default())
            .collect();
        parts.resize(5, std::ffi::CString::default());
        let names = std::array::from_fn(|i| parts[i].as_c_str());
        let locale = crate::platform::linux::shell::keyboard::locale();
        crate::platform::linux::ffi::xkb::Keyboard::from_names(names, &locale).ok()
    }
    /// Valeur de la propriété `property` de `window` (atomes ou octets), au plus 1 Mo.
    pub fn property(&self, window: u32, property: u32) -> Vec<u8> {
        self.read_property(window, property, false).1
    }
    /// Type et valeur de la propriété `property` de `window`, supprimée après lecture.
    pub fn take_property(&self, window: u32, property: u32) -> (u32, Vec<u8>) {
        self.read_property(window, property, true)
    }
    fn read_property(&self, window: u32, property: u32, delete: bool) -> (u32, Vec<u8>) {
        let xcb = self.xcb;
        // SAFETY : requête puis réponse libérée ici ; valeur de `length` octets.
        unsafe {
            let cookie =
                (xcb.xcb_get_property)(self.raw, u8::from(delete), window, property, 0, 0, 1 << 18);
            let reply = (xcb.xcb_get_property_reply)(self.raw, cookie, std::ptr::null_mut());
            if reply.is_null() {
                return (0, Vec::new());
            }
            let kind = (*reply).type_;
            let length = (xcb.xcb_get_property_value_length)(reply).max(0) as usize;
            let value = (xcb.xcb_get_property_value)(reply).cast::<u8>();
            let bytes = if value.is_null() {
                Vec::new()
            } else {
                std::slice::from_raw_parts(value, length).to_vec()
            };
            libc::free(reply.cast());
            (kind, bytes)
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
