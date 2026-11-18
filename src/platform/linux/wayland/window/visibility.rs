//! Masquer et réafficher une fenêtre sous Wayland. `xdg_toplevel` n'a pas de requête
//! « masquer » : la surface est démappée par un envoi sans tampon, qui garde son rôle, son
//! titre, son identifiant d'application et sa décoration. Elle est remappée par un nouvel
//! envoi initial sans tampon ; le compositeur y répond par une configuration, suivie d'une
//! image entière. Une fenêtre remappée est une nouvelle fenêtre pour le compositeur, qui
//! la met au premier plan (GNOME, KDE) : c'est le seul moyen de la ramener sans jeton
//! d'activation (clic sur l'icône de la zone de notification).
use super::{Ptr, Window, request, wl_surface, xdg_toplevel};

impl Window {
    /// Masque (`false`) ou réaffiche (`true`) la fenêtre ; sans effet si elle y est déjà.
    pub(super) fn set_visible(&mut self, visible: bool) {
        if visible != self.hidden {
            return;
        }
        if visible {
            self.hidden = false;
            // L'agrandissement est un état demandé avant le mappage, comme à l'ouverture.
            if self.session.maximized {
                request!(self.toplevel, xdg_toplevel::SET_MAXIMIZED);
            }
            request!(self.surface, wl_surface::COMMIT);
            return;
        }
        self.hidden = true;
        // Ressources de l'image (tampons, renderers) libérées comme pour une fenêtre
        // réduite ; la prochaine image est dessinée entière.
        self.hidden();
        if !self.frame.is_null() {
            // SAFETY : rappel en attente, qui ne viendra plus pour une surface démappée.
            unsafe { crate::platform::linux::ffi::wayland::wl_proxy_destroy(self.frame) };
            self.frame = std::ptr::null_mut();
        }
        request!(
            self.surface,
            wl_surface::ATTACH,
            std::ptr::null_mut::<std::ffi::c_void>() as Ptr,
            0i32,
            0i32
        );
        request!(self.surface, wl_surface::COMMIT);
        // Aucune image avant la configuration qui suivra le remappage ; géométrie et
        // régions redéclarées à ce moment-là.
        self.configured = false;
        self.acked = false;
        self.pending = None;
        self.geometry = None;
        self.regions = None;
        self.session.presented = None;
    }
}
