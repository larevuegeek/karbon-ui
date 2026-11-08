//! Commandes d'une fenêtre ouverte sous Wayland. Le protocole `xdg_toplevel` laisse le
//! client choisir la taille d'une fenêtre flottante, annoncer ses bornes et demander
//! l'agrandissement ou le plein écran ; il ne donne ni position, ni premier plan, ni
//! masquage, ni activation sans jeton : ces changements sont sans effet
//! (`docs/windows.md`).
use super::{
    super::{display::request, protocol::xdg_toplevel},
    Window,
};
use crate::window::WindowChange;

impl Window {
    pub fn change(&mut self, change: WindowChange) {
        match change {
            WindowChange::Size { width, height } => {
                // Agrandie, collée ou en plein écran : la taille appartient au compositeur.
                if !self.session.tiled {
                    let (width, height) = self.session.bounded(width, height);
                    self.session.width = width;
                    self.session.height = height;
                    self.session.dirty = true;
                }
                if !self.session.facts.resizable {
                    self.announce_limits();
                }
            }
            WindowChange::MinSize(size) => {
                let (width, height) = size.unwrap_or((1, 1));
                self.session.options.min_width = width;
                self.session.options.min_height = height;
                self.announce_limits();
                self.change(WindowChange::Size {
                    width: self.session.width,
                    height: self.session.height,
                });
            }
            WindowChange::MaxSize(size) => {
                self.session.facts.max_size = size;
                self.announce_limits();
                self.change(WindowChange::Size {
                    width: self.session.width,
                    height: self.session.height,
                });
            }
            // Aucun protocole de proportions : gardées pour le prochain état remis.
            WindowChange::AspectRatio(ratio) => self.session.facts.aspect = ratio,
            WindowChange::Resizable(resizable) => {
                self.session.facts.resizable = resizable;
                self.announce_limits();
            }
            WindowChange::Maximized(maximized) => self.set_maximized(maximized),
            WindowChange::Fullscreen(true) => request!(
                self.toplevel,
                xdg_toplevel::SET_FULLSCREEN,
                std::ptr::null_mut::<std::ffi::c_void>()
            ),
            WindowChange::Fullscreen(false) => {
                request!(self.toplevel, xdg_toplevel::UNSET_FULLSCREEN)
            }
            WindowChange::Topmost(_)
            | WindowChange::Position { .. }
            | WindowChange::Center
            | WindowChange::KeepOnScreen
            | WindowChange::Visible(_)
            | WindowChange::Focus => {}
        }
    }
    /// Bornes de taille annoncées au compositeur (fixes pour une fenêtre non
    /// redimensionnable ; `0` : pas de maximum).
    fn announce_limits(&self) {
        let ((min_width, min_height), max) = self.session.size_limits();
        let (max_width, max_height) = max.unwrap_or((0, 0));
        request!(
            self.toplevel,
            xdg_toplevel::SET_MIN_SIZE,
            min_width,
            min_height
        );
        request!(
            self.toplevel,
            xdg_toplevel::SET_MAX_SIZE,
            max_width,
            max_height
        );
    }
}
