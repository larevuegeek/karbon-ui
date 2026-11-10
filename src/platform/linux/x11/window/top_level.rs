pub(super) use super::super::connection::Connection;
use super::*;
pub(super) use crate::{
    platform::linux::{
        bitmap::Bitmap,
        ffi::xcb::*,
        graphics::Renderer,
        imaging::Imaging,
        shell::{app_id, session::Session},
        vulkan,
    },
    ui::runtime::UiRuntime,
    window::WindowDescriptor,
};
pub(super) use std::rc::Rc;

/// Nom d'hôte du processus (`WM_CLIENT_MACHINE`), sans octet nul.
pub(super) fn hostname() -> Vec<u8> {
    let mut name = [0u8; 256];
    // SAFETY : tampon local ; `gethostname` écrit au plus sa taille.
    if unsafe { libc::gethostname(name.as_mut_ptr().cast(), name.len()) } != 0 {
        return b"localhost".to_vec();
    }
    let end = name.iter().position(|b| *b == 0).unwrap_or(name.len());
    name[..end].to_vec()
}

pub(crate) struct Window {
    /// Application, renderers, taille, tâches et réveils (communs avec Wayland).
    pub session: Session,
    pub window: u32,
    pub(super) colormap: u32,
    pub(super) gc: u32,
    pub(super) depth: u8,
    /// Affichée par le gestionnaire (`MapNotify`), et exposée au moins une fois.
    pub mapped: bool,
    pub exposed: bool,
    /// Image Cairo de la fenêtre (chemin logiciel).
    pub(super) surface_pixels: Option<Bitmap>,
    /// Coin de la fenêtre sur l'écran, lu au besoin et oublié à chaque `ConfigureNotify`.
    pub(super) origin: std::cell::Cell<Option<(i32, i32)>>,
}
impl Window {
    /// Charge l'application, puis crée et affiche sa fenêtre.
    pub fn open(
        descriptor: WindowDescriptor,
        connection: &Connection,
        gpu: Option<Rc<vulkan::Gpu>>,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> crate::Result<Self> {
        let WindowDescriptor {
            id,
            options,
            application,
        } = descriptor;
        if let Some(reason) = crate::window::validation::invalid_options(&options) {
            return Err(crate::Error::InvalidWindowOptions {
                reason: reason.into(),
            });
        }
        let mut renderer = Renderer::new()?;
        renderer.follow_desktop();
        renderer.load_fonts(application.fonts())?;
        let mut runtime = UiRuntime::from_dyn(application, &Imaging)?;
        runtime.set_reduced_motion(options.reduced_motion);
        let xcb = connection.xcb;
        let c = connection.raw;
        let scale = connection.scale;
        let visual = match connection.argb {
            Some(argb) if options.transparent => argb,
            _ => connection.opaque,
        };
        let pixels = |v: i32| (v.max(1) as f32 * scale).round().clamp(1., 16384.) as u16;
        let (width, height) = (pixels(options.width), pixels(options.height));
        // SAFETY : connexion valide ; identifiants neufs ; listes de valeurs dans l'ordre
        // croissant des bits de leur masque, lues pendant l'appel.
        let (window, colormap, gc) = unsafe {
            let window = (xcb.xcb_generate_id)(c);
            let colormap = (xcb.xcb_generate_id)(c);
            (xcb.xcb_create_colormap)(
                c,
                XCB_COLORMAP_ALLOC_NONE,
                colormap,
                connection.root,
                visual.id,
            );
            let events = XCB_EVENT_MASK_EXPOSURE
                | XCB_EVENT_MASK_STRUCTURE_NOTIFY
                | XCB_EVENT_MASK_PROPERTY_CHANGE
                | XCB_EVENT_MASK_FOCUS_CHANGE
                | super::super::input::EVENTS;
            // Sans fond : rien n'est effacé avant la première image (pas d'éclair).
            let values = [0u32, 0, XCB_GRAVITY_NORTH_WEST, events, colormap];
            (xcb.xcb_create_window)(
                c,
                visual.depth,
                window,
                connection.root,
                0,
                0,
                width,
                height,
                0,
                XCB_WINDOW_CLASS_INPUT_OUTPUT,
                visual.id,
                XCB_CW_BACK_PIXMAP
                    | XCB_CW_BORDER_PIXEL
                    | XCB_CW_BIT_GRAVITY
                    | XCB_CW_EVENT_MASK
                    | XCB_CW_COLORMAP,
                values.as_ptr().cast(),
            );
            let gc = (xcb.xcb_generate_id)(c);
            (xcb.xcb_create_gc)(c, gc, window, 0, std::ptr::null());
            (window, colormap, gc)
        };
        let mut this = Self {
            session: Session::new(id, options, (runtime, renderer), wake),
            window,
            colormap,
            gc,
            depth: visual.depth,
            mapped: false,
            exposed: false,
            surface_pixels: None,
            origin: std::cell::Cell::new(None),
        };
        this.session.scale = scale;
        this.session.server_decorated = this.session.options.title_bar;
        this.session.physical = Some((width as usize, height as usize));
        this.properties(connection);
        this.session.gpu = paint::gpu(&this.session.options, gpu.as_ref(), connection, window);
        // SAFETY : fenêtre créée ci-dessus.
        unsafe { (xcb.xcb_map_window)(c, window) };
        connection.flush();
        Ok(this)
    }
    /// Titre, classe, PID, taille minimale, protocoles et décorations.
    pub(super) fn properties(&self, connection: &Connection) {
        let atoms = connection.atoms;
        let options = &self.session.options;
        let set = |property: u32, kind: u32, format: u8, data: &[u8]| {
            let count = data.len() as u32 / (u32::from(format) / 8);
            // SAFETY : données locales lues pendant l'appel ; fenêtre valide.
            unsafe {
                (connection.xcb.xcb_change_property)(
                    connection.raw,
                    XCB_PROP_MODE_REPLACE,
                    self.window,
                    property,
                    kind,
                    format,
                    count,
                    data.as_ptr().cast(),
                )
            };
        };
        let words = |values: &[u32]| {
            values
                .iter()
                .flat_map(|v| v.to_ne_bytes())
                .collect::<Vec<_>>()
        };
        set(
            XCB_ATOM_WM_NAME,
            XCB_ATOM_STRING,
            8,
            options.title.as_bytes(),
        );
        set(
            atoms.net_wm_name,
            atoms.utf8_string,
            8,
            options.title.as_bytes(),
        );
        let class = app_id();
        let class = format!("{class}\0{class}\0");
        set(XCB_ATOM_WM_CLASS, XCB_ATOM_STRING, 8, class.as_bytes());
        set(
            atoms.net_wm_pid,
            XCB_ATOM_CARDINAL,
            32,
            &words(&[std::process::id()]),
        );
        // Sans `WM_CLIENT_MACHINE`, le gestionnaire ignore le PID (dialogue « ne répond
        // pas » sans bouton pour forcer la fermeture).
        set(atoms.wm_client_machine, XCB_ATOM_STRING, 8, &hostname());
        set(
            atoms.wm_protocols,
            XCB_ATOM_ATOM,
            32,
            &words(&[atoms.wm_delete_window, atoms.net_wm_ping]),
        );
        self.size_hints(connection);
        if !options.title_bar {
            // `MWM_HINTS_DECORATIONS` seul, aucune décoration.
            set(
                atoms.motif_wm_hints,
                atoms.motif_wm_hints,
                32,
                &words(&[2, 0, 0, 0, 0]),
            );
        }
    }
    /// `ConfigureNotify` : taille en pixels de la fenêtre.
    pub fn resized(&mut self, width: u16, height: u16) {
        // La fenêtre a pu bouger aussi.
        self.origin.set(None);
        let size = (usize::from(width.max(1)), usize::from(height.max(1)));
        if self.session.physical == Some(size) {
            return;
        }
        let scale = self.session.scale;
        self.session.physical = Some(size);
        self.session.width = ((size.0 as f32 / scale).round() as i32).max(1);
        self.session.height = ((size.1 as f32 / scale).round() as i32).max(1);
        self.session.dirty = true;
    }
    /// Coin de la fenêtre en pixels de l'écran (fenêtre des candidats de la méthode
    /// d'entrée).
    pub fn origin(&self, connection: &Connection) -> Option<(i32, i32)> {
        if let Some(origin) = self.origin.get() {
            return Some(origin);
        }
        let xcb = connection.xcb;
        // SAFETY : fenêtres valides ; réponse libérée ici.
        let origin = unsafe {
            let cookie =
                (xcb.xcb_translate_coordinates)(connection.raw, self.window, connection.root, 0, 0);
            let reply =
                (xcb.xcb_translate_coordinates_reply)(connection.raw, cookie, std::ptr::null_mut());
            let origin = reply
                .as_ref()
                .map(|r| (i32::from(r.dst_x), i32::from(r.dst_y)));
            libc::free(reply.cast());
            origin
        };
        self.origin.set(origin);
        origin
    }
    /// `Expose` (dernier d'une série) : le contenu est à redessiner en entier.
    pub fn exposed(&mut self) {
        self.exposed = true;
        self.session.presented = None;
        self.session.renderer.forget();
        self.session.dirty = true;
    }
    /// `_NET_WM_STATE` changé : agrandie, plein écran, réduite.
    pub fn states(&mut self, connection: &Connection) {
        let atoms = connection.atoms;
        let states: Vec<u32> = connection
            .property(self.window, atoms.net_wm_state)
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| u32::from_ne_bytes(*c))
            .collect();
        let maximized = states.contains(&atoms.net_wm_state_maximized_vert)
            && states.contains(&atoms.net_wm_state_maximized_horz);
        let fullscreen = states.contains(&atoms.net_wm_state_fullscreen);
        if maximized != self.session.maximized {
            self.session.maximized = maximized;
            self.session.dirty = true;
        }
        self.session.tiled = maximized || fullscreen;
        self.session.facts.fullscreen = fullscreen;
        self.session.facts.minimized = states.contains(&atoms.net_wm_state_hidden);
        if states.contains(&atoms.net_wm_state_hidden) {
            self.hidden();
        }
    }
    /// Demande au gestionnaire d'agrandir ou de rétablir la fenêtre (EWMH).
    pub fn set_maximized(&self, connection: &Connection, maximized: bool) {
        let atoms = connection.atoms;
        self.message(
            connection,
            atoms.net_wm_state,
            [
                u32::from(maximized),
                atoms.net_wm_state_maximized_vert,
                atoms.net_wm_state_maximized_horz,
                1,
                0,
            ],
        );
    }
    /// Demande au gestionnaire de réduire la fenêtre (ICCCM, état iconique).
    pub fn minimize(&self, connection: &Connection) {
        const ICONIC_STATE: u32 = 3;
        self.message(
            connection,
            connection.atoms.wm_change_state,
            [ICONIC_STATE, 0, 0, 0, 0],
        );
    }
    /// Message `kind` à la racine, au sujet de cette fenêtre.
    pub(super) fn message(&self, connection: &Connection, kind: u32, data: [u32; 5]) {
        let event = xcb_client_message_event_t {
            response_type: XCB_CLIENT_MESSAGE,
            format: 32,
            sequence: 0,
            window: self.window,
            type_: kind,
            data,
        };
        send_to_root(connection, &event);
    }
}
/// Envoie `event` à la racine, pour le gestionnaire de fenêtres.
pub(crate) fn send_to_root(connection: &Connection, event: &xcb_client_message_event_t) {
    // SAFETY : événement de 32 octets lu pendant l'appel.
    unsafe {
        (connection.xcb.xcb_send_event)(
            connection.raw,
            0,
            connection.root,
            XCB_EVENT_MASK_SUBSTRUCTURE_NOTIFY | XCB_EVENT_MASK_SUBSTRUCTURE_REDIRECT,
            (event as *const xcb_client_message_event_t).cast(),
        )
    };
}
impl Window {
    /// Détruit la fenêtre et ses ressources X (après la surface Vulkan).
    pub fn close(mut self, connection: &Connection) {
        self.session.gpu = None;
        let xcb = connection.xcb;
        // SAFETY : objets créés par `open`, détruits une fois.
        unsafe {
            (xcb.xcb_free_gc)(connection.raw, self.gc);
            (xcb.xcb_destroy_window)(connection.raw, self.window);
            (xcb.xcb_free_colormap)(connection.raw, self.colormap);
        }
        connection.flush();
    }
}
impl crate::platform::linux::accessibility::Host for Window {
    fn id(&self) -> crate::window::WindowId {
        self.session.id
    }
    fn title(&self) -> &str {
        crate::platform::linux::accessibility::Host::title(&self.session)
    }
    fn size(&self) -> (i32, i32) {
        crate::platform::linux::accessibility::Host::size(&self.session)
    }
    fn runtime(&self) -> &UiRuntime {
        &self.session.runtime
    }
    fn active(&self) -> bool {
        crate::platform::linux::accessibility::Host::active(&self.session)
    }
    fn act(&mut self, action: crate::ui::runtime::AccessibilityAction) -> crate::Result<bool> {
        crate::platform::linux::accessibility::Host::act(&mut self.session, action)
    }
}
