//! État de la fenêtre vu par le système (position, plein écran, bornes de taille), remis
//! à l'application par `Application::window_changed` ; commun aux boucles Wayland et X11.
use super::session::Session;
use crate::window::{ResizeEdge, WindowState};

/// Ce que le moteur sait de la fenêtre en plus de sa taille et de son échelle.
pub(in crate::platform::linux) struct Facts {
    /// Coin de la fenêtre en unités logiques de l'écran (X11 ; inconnu sous Wayland).
    pub x: i32,
    pub y: i32,
    pub minimized: bool,
    pub fullscreen: bool,
    /// Bords à tirer (`WindowChange::Resizable`).
    pub resizable: bool,
    /// Redimensionnement à la souris en cours.
    pub dragging: bool,
    /// Taille maximale et proportions imposées (unités logiques).
    pub max_size: Option<(i32, i32)>,
    pub aspect: Option<(u32, u32)>,
    /// Dernier état remis à l'application.
    reported: Option<WindowState>,
}
impl Default for Facts {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            minimized: false,
            fullscreen: false,
            resizable: true,
            dragging: false,
            max_size: None,
            aspect: None,
            reported: None,
        }
    }
}
impl Session {
    /// Taille logique bornée par la taille minimale et maximale.
    pub fn bounded(&self, width: i32, height: i32) -> (i32, i32) {
        let (max_width, max_height) = self.facts.max_size.unwrap_or((i32::MAX, i32::MAX));
        (
            width.min(max_width).max(self.options.min_width.max(1)),
            height.min(max_height).max(self.options.min_height.max(1)),
        )
    }
    /// Bornes de taille annoncées au système : fixes (`min = max = taille`) quand la
    /// fenêtre n'est pas redimensionnable.
    pub fn size_limits(&self) -> ((i32, i32), Option<(i32, i32)>) {
        if self.facts.resizable {
            (
                (
                    self.options.min_width.max(1),
                    self.options.min_height.max(1),
                ),
                self.facts.max_size,
            )
        } else {
            let size = (self.width, self.height);
            (size, Some(size))
        }
    }
    /// État actuel de la fenêtre.
    pub fn snapshot(&self) -> WindowState {
        WindowState {
            x: self.facts.x,
            y: self.facts.y,
            width: self.width,
            height: self.height,
            scale: self.scale,
            maximized: self.maximized,
            minimized: self.facts.minimized,
            fullscreen: self.facts.fullscreen,
            topmost: self.options.topmost,
            resizable: self.facts.resizable,
            dragging: self.facts.dragging,
        }
    }
    /// Remet l'état à l'application s'il a changé depuis le dernier envoi.
    pub fn report(&mut self) {
        let current = self.snapshot();
        if self.facts.reported == Some(current) {
            return;
        }
        self.facts.reported = Some(current);
        if self.runtime.window_changed(current) {
            self.dirty = true;
        }
    }
    /// Poignée de redimensionnement de l'application sous `point`.
    pub fn resize_region_at(&self, point: crate::ui::scene::PointF) -> Option<ResizeEdge> {
        self.runtime.resize_region_at(point)
    }
}
