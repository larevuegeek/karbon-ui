//! Commandes d'une fenêtre ouverte sous X11 : géométrie par `ConfigureWindow`, bornes et
//! proportions par `WM_NORMAL_HINTS` (ICCCM), premier plan, plein écran et activation par
//! les messages EWMH au gestionnaire de fenêtres, zone de travail lue dans
//! `_NET_WORKAREA` (l'écran entier sans elle).
use super::{super::connection::Connection, Window};
use crate::{
    platform::linux::ffi::xcb::*,
    window::{
        WindowChange,
        change::{ScreenRect, centered, keep_on_screen},
    },
};

/// Champs de `xcb_configure_window`.
const CONFIG_X: u16 = 1;
const CONFIG_Y: u16 = 2;
const CONFIG_WIDTH: u16 = 4;
const CONFIG_HEIGHT: u16 = 8;
/// Drapeaux de `XSizeHints`.
const P_MIN_SIZE: u32 = 1 << 4;
const P_MAX_SIZE: u32 = 1 << 5;
const P_ASPECT: u32 = 1 << 7;

impl Window {
    pub fn change(&mut self, connection: &Connection, change: WindowChange) {
        let scale = self.session.scale;
        let pixels = |v: i32| (v as f32 * scale).round() as i32;
        match change {
            WindowChange::Size { width, height } => {
                let (width, height) = self.session.bounded(width, height);
                if !self.session.facts.resizable {
                    // La taille fixe suit la nouvelle taille.
                    self.session.width = width;
                    self.session.height = height;
                    self.size_hints(connection);
                }
                self.configure(
                    connection,
                    CONFIG_WIDTH | CONFIG_HEIGHT,
                    &[pixels(width) as u32, pixels(height) as u32],
                );
            }
            WindowChange::MinSize(size) => {
                let (width, height) = size.unwrap_or((1, 1));
                self.session.options.min_width = width;
                self.session.options.min_height = height;
                self.size_hints(connection);
            }
            WindowChange::MaxSize(size) => {
                self.session.facts.max_size = size;
                self.size_hints(connection);
            }
            WindowChange::AspectRatio(ratio) => {
                self.session.facts.aspect = ratio;
                self.size_hints(connection);
            }
            WindowChange::Resizable(resizable) => {
                self.session.facts.resizable = resizable;
                self.size_hints(connection);
            }
            WindowChange::Topmost(topmost) => {
                self.session.options.topmost = topmost;
                let atoms = connection.atoms;
                self.message(
                    connection,
                    atoms.net_wm_state,
                    [u32::from(topmost), atoms.net_wm_state_above, 0, 1, 0],
                );
            }
            WindowChange::Position { x, y } => {
                self.configure(
                    connection,
                    CONFIG_X | CONFIG_Y,
                    &[pixels(x) as u32, pixels(y) as u32],
                );
            }
            WindowChange::Center | WindowChange::KeepOnScreen => {
                let Some((x, y)) = self.origin(connection) else {
                    return;
                };
                let (width, height) = self.session.physical.map_or(
                    (pixels(self.session.width), pixels(self.session.height)),
                    |(w, h)| (w as i32, h as i32),
                );
                let window = ScreenRect {
                    x,
                    y,
                    width,
                    height,
                };
                let area = work_area(connection);
                let target = if change == WindowChange::Center {
                    centered(width, height, area)
                } else {
                    keep_on_screen(window, area)
                };
                if target != (x, y) {
                    self.configure(
                        connection,
                        CONFIG_X | CONFIG_Y,
                        &[target.0 as u32, target.1 as u32],
                    );
                }
            }
            WindowChange::Maximized(maximized) => self.set_maximized(connection, maximized),
            WindowChange::Fullscreen(fullscreen) => {
                let atoms = connection.atoms;
                self.message(
                    connection,
                    atoms.net_wm_state,
                    [
                        u32::from(fullscreen),
                        atoms.net_wm_state_fullscreen,
                        0,
                        1,
                        0,
                    ],
                );
            }
            WindowChange::Visible(visible) => {
                let xcb = connection.xcb;
                // SAFETY : fenêtre valide de cette connexion.
                unsafe {
                    if visible {
                        (xcb.xcb_map_window)(connection.raw, self.window);
                    } else {
                        (xcb.xcb_unmap_window)(connection.raw, self.window);
                    }
                }
            }
            WindowChange::Focus => {
                let xcb = connection.xcb;
                // SAFETY : fenêtre valide de cette connexion.
                unsafe { (xcb.xcb_map_window)(connection.raw, self.window) };
                // Source 1 : une application ; horodatage inconnu.
                self.message(
                    connection,
                    connection.atoms.net_active_window,
                    [1, 0, 0, 0, 0],
                );
            }
        }
        connection.flush();
    }
    /// Lit la position de la fenêtre pour l'état remis à l'application.
    pub fn locate(&mut self, connection: &Connection) {
        if let Some((x, y)) = self.origin(connection) {
            let scale = self.session.scale.max(0.01);
            self.session.facts.x = (x as f32 / scale).round() as i32;
            self.session.facts.y = (y as f32 / scale).round() as i32;
        }
    }
    fn configure(&self, connection: &Connection, mask: u16, values: &[u32]) {
        // SAFETY : valeurs locales lues pendant l'appel, dans l'ordre des bits du masque.
        unsafe {
            (connection.xcb.xcb_configure_window)(
                connection.raw,
                self.window,
                mask,
                values.as_ptr().cast(),
            )
        };
    }
    /// `WM_NORMAL_HINTS` (ICCCM `XSizeHints`) : bornes et proportions en pixels.
    pub(super) fn size_hints(&self, connection: &Connection) {
        let scale = self.session.scale;
        let pixels = |v: i32| (v.max(1) as f32 * scale).round() as u32;
        let ((min_width, min_height), max) = self.session.size_limits();
        let mut hints = [0u32; 18];
        hints[0] = P_MIN_SIZE;
        hints[5] = pixels(min_width);
        hints[6] = pixels(min_height);
        if let Some((width, height)) = max {
            hints[0] |= P_MAX_SIZE;
            hints[7] = pixels(width);
            hints[8] = pixels(height);
        }
        if let Some((width, height)) = self.session.facts.aspect {
            hints[0] |= P_ASPECT;
            hints[11] = width;
            hints[12] = height;
            hints[13] = width;
            hints[14] = height;
        }
        let bytes: Vec<u8> = hints.iter().flat_map(|v| v.to_ne_bytes()).collect();
        // SAFETY : données locales lues pendant l'appel ; fenêtre valide.
        unsafe {
            (connection.xcb.xcb_change_property)(
                connection.raw,
                XCB_PROP_MODE_REPLACE,
                self.window,
                XCB_ATOM_WM_NORMAL_HINTS,
                XCB_ATOM_WM_SIZE_HINTS,
                32,
                hints.len() as u32,
                bytes.as_ptr().cast(),
            )
        };
    }
}

/// Zone de travail du bureau courant (`_NET_WORKAREA`), sinon l'écran entier, en pixels.
fn work_area(connection: &Connection) -> ScreenRect {
    let values: Vec<i32> = connection
        .property(connection.root, connection.atoms.net_workarea)
        .as_chunks::<4>()
        .0
        .iter()
        .map(|c| i32::from_ne_bytes(*c))
        .collect();
    if let [x, y, width, height, ..] = values[..]
        && width > 0
        && height > 0
    {
        return ScreenRect {
            x,
            y,
            width,
            height,
        };
    }
    // SAFETY : écran de la connexion, valide tant qu'elle vit.
    let screen = unsafe { &*connection.screen };
    ScreenRect {
        x: 0,
        y: 0,
        width: i32::from(screen.width_in_pixels),
        height: i32::from(screen.height_in_pixels),
    }
}
