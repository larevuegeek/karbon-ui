//! Fenêtre Wayland : surface `xdg_toplevel`, taille et états reçus du compositeur,
//! échelle (fractionnaire par `wp_fractional_scale` ou entière), runtime de
//! l'application, renderers (Vulkan par défaut, Cairo en repli) et tâches de fond.
mod access;
mod change;
mod paint;

use super::{
    display::{Globals, Kind, NEW, Ptr, create, destroy, request, text},
    protocol::{
        WP_FRACTIONAL_SCALE_V1, WP_VIEWPORT, XDG_SURFACE, XDG_TOPLEVEL,
        ZXDG_TOPLEVEL_DECORATION_V1, wl_compositor, wl_surface, wp_fractional_scale_manager_v1,
        wp_fractional_scale_v1, wp_viewport, wp_viewporter, xdg_surface, xdg_toplevel, xdg_wm_base,
        zxdg_decoration_manager_v1, zxdg_toplevel_decoration_v1,
    },
    shm::Buffer,
};
use crate::platform::linux::shell::session::Session;
use crate::{
    platform::linux::{
        bitmap::Bitmap, ffi::wayland::wl_surface_interface, graphics::Renderer, imaging::Imaging,
        vulkan,
    },
    ui::runtime::UiRuntime,
    window::WindowDescriptor,
};

pub(super) use paint::shared_gpu;

/// États `xdg_toplevel` utiles.
const STATE_MAXIMIZED: u32 = 1;
const STATE_FULLSCREEN: u32 = 2;
const STATE_RESIZING: u32 = 3;
const STATE_TILED_LEFT: u32 = 5;
const STATE_TILED_BOTTOM: u32 = 8;
/// Fenêtre réduite ou entièrement masquée (xdg-shell 6).
const STATE_SUSPENDED: u32 = 9;
/// Modes de `zxdg_toplevel_decoration_v1`.
const DECORATION_CLIENT: u32 = 1;
const DECORATION_SERVER: u32 = 2;

pub(super) struct Window {
    pub surface: Ptr,
    pub xdg: Ptr,
    pub toplevel: Ptr,
    viewport: Ptr,
    pub fractional: Ptr,
    pub decoration: Ptr,
    /// Rappel de la dernière image envoyée, pas encore affichée.
    pub frame: Ptr,
    /// Application, renderers, taille, tâches et réveils (communs avec X11).
    pub session: Session,
    /// Configuration annoncée par `xdg_toplevel.configure`, appliquée au `configure`
    /// de la surface qui la confirme.
    pending: Option<(i32, i32, Vec<u32>)>,
    /// Une configuration a été reçue (la surface peut recevoir une image).
    configured: bool,
    /// Configuration confirmée sans image envoyée depuis.
    acked: bool,
    /// Réduite ou masquée (état `suspended` de xdg-shell 6) : aucune image n'est
    /// dessinée jusqu'à la configuration qui la rend visible.
    suspended: bool,
    /// Échelle fractionnaire préférée (×120), si le compositeur la donne.
    pub scale120: Option<u32>,
    /// Échelle entière préférée par le compositeur (`wl_surface.preferred_buffer_scale`).
    pub preferred_scale: Option<i32>,
    /// Échelle entière retenue : préférée, sinon celle des sorties où est la surface.
    pub integer_scale: i32,
    /// Sorties où la surface est affichée.
    pub outputs: Vec<Ptr>,
    surface_pixels: Option<Bitmap>,
    buffers: Vec<Buffer>,
    /// Taille logique et transparence des régions déjà déclarées.
    regions: Option<(i32, i32)>,
}
impl Window {
    /// Charge l'application, puis crée sa surface (affichée à la première configuration).
    pub fn open(
        descriptor: WindowDescriptor,
        globals: &Globals,
        (display, gpu): (Ptr, Option<std::rc::Rc<vulkan::Gpu>>),
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
        let surface = create!(
            globals.compositor,
            wl_compositor::CREATE_SURFACE,
            &raw const wl_surface_interface,
            Kind::Surface,
            NEW
        );
        let xdg = create!(
            globals.wm_base,
            xdg_wm_base::GET_XDG_SURFACE,
            &XDG_SURFACE,
            Kind::XdgSurface,
            NEW,
            surface
        );
        let toplevel = create!(
            xdg,
            xdg_surface::GET_TOPLEVEL,
            &XDG_TOPLEVEL,
            Kind::Toplevel,
            NEW
        );
        request!(
            toplevel,
            xdg_toplevel::SET_TITLE,
            text(options.title).as_ptr()
        );
        request!(
            toplevel,
            xdg_toplevel::SET_APP_ID,
            text(&crate::platform::linux::shell::app_id()).as_ptr()
        );
        request!(
            toplevel,
            xdg_toplevel::SET_MIN_SIZE,
            options.min_width.max(1),
            options.min_height.max(1)
        );
        let mut decoration = std::ptr::null_mut();
        if !globals.decoration.is_null() {
            decoration = create!(
                globals.decoration,
                zxdg_decoration_manager_v1::GET_TOPLEVEL_DECORATION,
                &ZXDG_TOPLEVEL_DECORATION_V1,
                Kind::Decoration,
                NEW,
                toplevel
            );
            // Barre de titre demandée : celle du compositeur s'il en dessine une.
            let mode = if options.title_bar {
                DECORATION_SERVER
            } else {
                DECORATION_CLIENT
            };
            request!(decoration, zxdg_toplevel_decoration_v1::SET_MODE, mode);
        }
        let (mut fractional, mut viewport) = (std::ptr::null_mut(), std::ptr::null_mut());
        if !globals.fractional.is_null() && !globals.viewporter.is_null() {
            fractional = create!(
                globals.fractional,
                wp_fractional_scale_manager_v1::GET_FRACTIONAL_SCALE,
                &WP_FRACTIONAL_SCALE_V1,
                Kind::Fractional,
                NEW,
                surface
            );
            viewport = create!(
                globals.viewporter,
                wp_viewporter::GET_VIEWPORT,
                &WP_VIEWPORT,
                Kind::Silent,
                NEW,
                surface
            );
        }
        let gpu = paint::gpu(&options, gpu.as_ref(), display, surface);
        // Premier envoi sans image : le compositeur répond par la configuration initiale.
        request!(surface, wl_surface::COMMIT);
        let mut session = Session::new(id, options, (runtime, renderer), wake);
        session.gpu = gpu;
        Ok(Self {
            session,
            surface,
            xdg,
            toplevel,
            viewport,
            fractional,
            decoration,
            frame: std::ptr::null_mut(),
            pending: None,
            configured: false,
            acked: false,
            suspended: false,
            scale120: None,
            preferred_scale: None,
            integer_scale: 1,
            outputs: Vec::new(),
            surface_pixels: None,
            buffers: Vec::new(),
            regions: None,
        })
    }
    /// `xdg_toplevel.configure` : taille proposée (0 : au choix de l'application) et états.
    pub fn propose(&mut self, width: i32, height: i32, states: &[u8]) {
        let states = states
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| u32::from_ne_bytes(*c))
            .collect();
        self.pending = Some((width, height, states));
    }
    /// `xdg_surface.configure` : la configuration annoncée est confirmée et appliquée.
    pub fn configure(&mut self, serial: u32) {
        request!(self.xdg, xdg_surface::ACK_CONFIGURE, serial);
        if let Some((width, height, states)) = self.pending.take() {
            if width > 0 && height > 0 {
                self.session.width = width.max(self.session.options.min_width.max(1));
                self.session.height = height.max(self.session.options.min_height.max(1));
            }
            self.session.maximized = states.contains(&STATE_MAXIMIZED);
            self.session.tiled = self.session.maximized
                || states.contains(&STATE_FULLSCREEN)
                || states
                    .iter()
                    .any(|s| (STATE_TILED_LEFT..=STATE_TILED_BOTTOM).contains(s));
            self.session
                .runtime
                .set_live_resize(states.contains(&STATE_RESIZING));
            self.session.facts.fullscreen = states.contains(&STATE_FULLSCREEN);
            self.session.facts.dragging = states.contains(&STATE_RESIZING);
            self.suspended = states.contains(&STATE_SUSPENDED);
            self.session.facts.minimized = self.suspended;
            if self.suspended {
                self.hidden();
            }
        }
        self.configured = true;
        self.acked = true;
        self.session.dirty = true;
    }
    /// Mode de décoration choisi par le compositeur.
    pub fn decorated(&mut self, mode: u32) {
        self.session.server_decorated = mode == DECORATION_SERVER;
        self.session.dirty = true;
    }
    /// Échelle de la surface : fractionnaire si le compositeur la donne, sinon entière.
    pub fn scale(&self) -> f32 {
        match self.scale120 {
            Some(s) if !self.viewport.is_null() => s as f32 / 120.,
            _ => self.integer_scale.max(1) as f32,
        }
    }
    /// L'échelle de la surface a pu changer : celle de la session est tenue à jour.
    pub fn rescale(&mut self) {
        self.session.scale = self.scale();
    }
    /// Tampon rendu par le compositeur.
    pub fn released(&mut self, buffer: Ptr) -> bool {
        match self.buffers.iter_mut().find(|b| b.raw == buffer) {
            Some(b) => {
                b.busy = false;
                true
            }
            None => false,
        }
    }
    /// L'image envoyée est affichée : la suivante peut être dessinée.
    pub fn frame_done(&mut self, callback: Ptr) -> bool {
        if self.frame != callback {
            return false;
        }
        // SAFETY : rappel terminé, sans requête destructrice dans le protocole.
        unsafe { crate::platform::linux::ffi::wayland::wl_proxy_destroy(callback) };
        self.frame = std::ptr::null_mut();
        true
    }
    pub fn set_maximized(&self, maximized: bool) {
        request!(
            self.toplevel,
            if maximized {
                xdg_toplevel::SET_MAXIMIZED
            } else {
                xdg_toplevel::UNSET_MAXIMIZED
            }
        );
    }
    pub fn minimize(&self) {
        request!(self.toplevel, xdg_toplevel::SET_MINIMIZED);
    }
}
impl Drop for Window {
    fn drop(&mut self) {
        // La surface Vulkan disparaît avant la `wl_surface`.
        self.session.gpu = None;
        if !self.frame.is_null() {
            // SAFETY : rappel en attente, oublié avec la fenêtre.
            unsafe { crate::platform::linux::ffi::wayland::wl_proxy_destroy(self.frame) };
        }
        destroy!(self.decoration, zxdg_toplevel_decoration_v1::DESTROY);
        destroy!(self.fractional, wp_fractional_scale_v1::DESTROY);
        destroy!(self.viewport, wp_viewport::DESTROY);
        destroy!(self.toplevel, xdg_toplevel::DESTROY);
        destroy!(self.xdg, xdg_surface::DESTROY);
        self.buffers.clear();
        destroy!(self.surface, wl_surface::DESTROY);
    }
}
