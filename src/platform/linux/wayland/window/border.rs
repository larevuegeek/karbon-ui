//! Bordure de redimensionnement hors du contenu, comme les fenêtres GTK sous GNOME : une
//! sous-surface transparente, placée sous la fenêtre et plus grande qu'elle de
//! [`MARGIN`] de chaque côté, reçoit les appuis du pourtour. Le contenu garde tous ses
//! pixels, jusqu'au bord : une barre de défilement collée au bord droit s'attrape à la
//! souris, ce que la bande intérieure de `ui::title_bar::hit_test` empêchait. La
//! géométrie de la fenêtre (`xdg_surface.set_window_geometry`) reste celle du contenu :
//! le compositeur place, colle et agrandit la fenêtre visible, sans la bordure.
//!
//! Masquée quand la fenêtre ne se redimensionne pas par ses bords (agrandie, collée, plein
//! écran, fixe, décorée par le compositeur). Sans `wl_subcompositor` ni `wp_viewporter`,
//! la bande intérieure reste.
use super::surface::{
    Buffer, Globals, Kind, NEW, Ptr, WP_VIEWPORT, create, destroy, request, wl_compositor,
    wl_surface, wp_viewport, wp_viewporter,
};
use crate::platform::linux::{
    ffi::wayland::{wl_subsurface_interface, wl_surface_interface},
    wayland::protocol::{wl_subcompositor, wl_subsurface},
};

/// Largeur de la bordure, en unités logiques (celle de la bande intérieure).
pub(crate) const MARGIN: i32 = crate::ui::title_bar::RESIZE_BORDER;

pub(crate) struct Border {
    pub surface: Ptr,
    subsurface: Ptr,
    viewport: Ptr,
    /// Un pixel transparent, étiré par le viewport à la taille de la bordure.
    buffer: Buffer,
    /// Taille logique du contenu entouré ; `None` : bordure masquée.
    shown: Option<(i32, i32)>,
}
impl Border {
    /// Bordure de la surface `parent`, masquée jusqu'au premier [`update`](Self::update).
    pub fn new(globals: &Globals, parent: Ptr) -> Option<Self> {
        if globals.subcompositor.is_null() || globals.viewporter.is_null() {
            return None;
        }
        // Tampon neuf rempli de zéros : un pixel entièrement transparent.
        let buffer = Buffer::new(globals.shm, 1, 1).ok()?;
        let surface = create!(
            globals.compositor,
            wl_compositor::CREATE_SURFACE,
            &raw const wl_surface_interface,
            Kind::Silent,
            NEW
        );
        let subsurface = create!(
            globals.subcompositor,
            wl_subcompositor::GET_SUBSURFACE,
            &raw const wl_subsurface_interface,
            Kind::Silent,
            NEW,
            surface,
            parent
        );
        request!(subsurface, wl_subsurface::PLACE_BELOW, parent);
        request!(subsurface, wl_subsurface::SET_POSITION, -MARGIN, -MARGIN);
        let viewport = create!(
            globals.viewporter,
            wp_viewporter::GET_VIEWPORT,
            &WP_VIEWPORT,
            Kind::Silent,
            NEW,
            surface
        );
        Some(Self {
            surface,
            subsurface,
            viewport,
            buffer,
            shown: None,
        })
    }
    /// Entoure un contenu de `size` (unités logiques), ou masque la bordure (`None`).
    /// Sous-surface synchronisée : appliqué avec le prochain envoi de la fenêtre.
    pub fn update(&mut self, size: Option<(i32, i32)>) {
        if self.shown == size {
            return;
        }
        self.shown = size;
        match size {
            Some((width, height)) => {
                request!(
                    self.surface,
                    wl_surface::ATTACH,
                    self.buffer.raw,
                    0i32,
                    0i32
                );
                request!(
                    self.viewport,
                    wp_viewport::SET_DESTINATION,
                    width + 2 * MARGIN,
                    height + 2 * MARGIN
                );
                request!(
                    self.surface,
                    wl_surface::DAMAGE_BUFFER,
                    0i32,
                    0i32,
                    1i32,
                    1i32
                );
            }
            None => request!(
                self.surface,
                wl_surface::ATTACH,
                std::ptr::null_mut::<std::ffi::c_void>(),
                0i32,
                0i32
            ),
        }
        request!(self.surface, wl_surface::COMMIT);
    }
    /// La bordure est affichée.
    pub fn shown(&self) -> bool {
        self.shown.is_some()
    }
}
impl Drop for Border {
    fn drop(&mut self) {
        destroy!(self.viewport, wp_viewport::DESTROY);
        destroy!(self.subsurface, wl_subsurface::DESTROY);
        destroy!(self.surface, wl_surface::DESTROY);
    }
}
