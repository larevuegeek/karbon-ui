//! Siège : pointeur et clavier selon les capacités annoncées, fenêtres qui ont le pointeur
//! et le clavier, dernier numéro d'entrée, défilement accumulé jusqu'à la fin d'une trame
//! du pointeur, et curseurs du thème (libwayland-cursor, taille 24 × échelle).
use super::{
    display::{Kind, NEW, Ptr, create, destroy, request},
    protocol::{wl_compositor, wl_keyboard, wl_pointer, wl_seat, wl_surface},
};
use crate::platform::linux::shell::keyboard::Keys;
use crate::{
    platform::linux::ffi::wayland::{
        wl_cursor_image_get_buffer, wl_cursor_theme_destroy, wl_cursor_theme_get_cursor,
        wl_cursor_theme_load, wl_keyboard_interface, wl_pointer_interface, wl_proxy_destroy,
        wl_proxy_get_version, wl_surface_interface,
    },
    ui::scene::PointF,
    window::WindowId,
};

const CAPABILITY_POINTER: u32 = 1;
const CAPABILITY_KEYBOARD: u32 = 2;

/// Défilement reçu pendant une trame du pointeur.
#[derive(Default)]
pub(super) struct Scroll {
    /// Valeurs continues (pixels logiques) et crans (×120) : vertical, horizontal.
    pub value: [f64; 2],
    pub value120: [i32; 2],
}
pub(super) struct Seat {
    pub seat: Ptr,
    pub pointer: Ptr,
    pub keyboard: Ptr,
    /// Fenêtre sous le pointeur et position logique.
    pub pointer_window: Option<WindowId>,
    pub position: PointF,
    /// Numéro de l'entrée du pointeur (changement de curseur).
    enter_serial: u32,
    /// Numéro du dernier appui (bouton ou touche) : déplacement, menu, presse-papiers.
    pub serial: u32,
    pub keyboard_window: Option<WindowId>,
    pub keys: Keys,
    pub scroll: Scroll,
    /// Thèmes chargés par taille en pixels, surface du curseur, curseur affiché.
    themes: Vec<(i32, Ptr)>,
    cursor_surface: Ptr,
    shown: Option<(&'static str, i32)>,
}
impl Seat {
    pub fn new(seat: Ptr) -> Self {
        Self {
            seat,
            pointer: std::ptr::null_mut(),
            keyboard: std::ptr::null_mut(),
            pointer_window: None,
            position: PointF { x: 0., y: 0. },
            enter_serial: 0,
            serial: 0,
            keyboard_window: None,
            keys: Keys::new(),
            scroll: Scroll::default(),
            themes: Vec::new(),
            cursor_surface: std::ptr::null_mut(),
            shown: None,
        }
    }
    /// `wl_seat.capabilities` : pointeur et clavier créés ou rendus.
    pub fn capabilities(&mut self, capabilities: u32) {
        let pointer = capabilities & CAPABILITY_POINTER != 0;
        if pointer && self.pointer.is_null() {
            self.pointer = create!(
                self.seat,
                wl_seat::GET_POINTER,
                &raw const wl_pointer_interface,
                Kind::Pointer,
                NEW
            );
        } else if !pointer && !self.pointer.is_null() {
            release(self.pointer, wl_pointer::RELEASE);
            self.pointer = std::ptr::null_mut();
            self.pointer_window = None;
        }
        let keyboard = capabilities & CAPABILITY_KEYBOARD != 0;
        if keyboard && self.keyboard.is_null() {
            self.keyboard = create!(
                self.seat,
                wl_seat::GET_KEYBOARD,
                &raw const wl_keyboard_interface,
                Kind::Keyboard,
                NEW
            );
        } else if !keyboard && !self.keyboard.is_null() {
            release(self.keyboard, wl_keyboard::RELEASE);
            self.keyboard = std::ptr::null_mut();
            self.keyboard_window = None;
            self.keys.reset();
        }
    }
    /// Le pointeur entre dans une surface : le curseur doit être redonné.
    pub fn entered(&mut self, serial: u32) {
        self.enter_serial = serial;
        self.shown = None;
    }
    /// Affiche le premier curseur de `names` présent dans le thème, à l'échelle entière
    /// `scale`.
    pub fn show_cursor(&mut self, compositor: Ptr, shm: Ptr, names: &[&'static str], scale: i32) {
        if self.pointer.is_null() || self.pointer_window.is_none() {
            return;
        }
        let scale = scale.max(1);
        let Some(&first) = names.first() else {
            return;
        };
        if self.shown == Some((first, scale)) {
            return;
        }
        let size = cursor_size() * scale;
        let theme = match self.themes.iter().find(|(s, _)| *s == size) {
            Some((_, theme)) => *theme,
            None => {
                // SAFETY : nom nul (thème par défaut ou `XCURSOR_THEME`) ; wl_shm lié.
                let name = std::env::var("XCURSOR_THEME")
                    .ok()
                    .and_then(|n| std::ffi::CString::new(n).ok());
                let theme = unsafe {
                    wl_cursor_theme_load(
                        name.as_ref().map_or(std::ptr::null(), |n| n.as_ptr()),
                        size,
                        shm,
                    )
                };
                // Un échec (thème absent) est mémorisé aussi : le disque n'est pas
                // parcouru de nouveau à chaque mouvement du pointeur.
                self.themes.push((size, theme));
                theme
            }
        };
        if theme.is_null() {
            return;
        }
        let cursor = names.iter().find_map(|name| {
            let name = std::ffi::CString::new(*name).ok()?;
            // SAFETY : thème valide ; curseur possédé par le thème.
            let cursor = unsafe { wl_cursor_theme_get_cursor(theme, name.as_ptr()) };
            // SAFETY : curseur valide ; au moins une image d'après `image_count`.
            (!cursor.is_null() && unsafe { (*cursor).image_count } > 0)
                .then(|| unsafe { *(*cursor).images })
        });
        let Some(image) = cursor else {
            return;
        };
        if self.cursor_surface.is_null() {
            self.cursor_surface = create!(
                compositor,
                wl_compositor::CREATE_SURFACE,
                &raw const wl_surface_interface,
                Kind::Silent,
                NEW
            );
        }
        // SAFETY : image du thème ; son tampon appartient au thème.
        let (buffer, image) = unsafe { (wl_cursor_image_get_buffer(image), &*image) };
        if buffer.is_null() {
            return;
        }
        let surface = self.cursor_surface;
        request!(surface, wl_surface::SET_BUFFER_SCALE, scale);
        request!(surface, wl_surface::ATTACH, buffer, 0i32, 0i32);
        request!(
            surface,
            wl_surface::DAMAGE_BUFFER,
            0i32,
            0i32,
            image.width as i32,
            image.height as i32
        );
        request!(surface, wl_surface::COMMIT);
        request!(
            self.pointer,
            wl_pointer::SET_CURSOR,
            self.enter_serial,
            surface,
            image.hotspot_x as i32 / scale,
            image.hotspot_y as i32 / scale
        );
        self.shown = Some((first, scale));
    }
    /// Libère les objets du siège (fin de la boucle).
    pub fn close(&mut self) {
        self.capabilities(0);
        destroy!(self.cursor_surface, wl_surface::DESTROY);
        self.cursor_surface = std::ptr::null_mut();
        for (_, theme) in self.themes.drain(..).filter(|(_, t)| !t.is_null()) {
            // SAFETY : thème possédé, détruit une fois.
            unsafe { wl_cursor_theme_destroy(theme) };
        }
    }
}
/// Rend un pointeur ou un clavier : requête `release` (version 3) ou simple oubli.
fn release(proxy: Ptr, opcode: u32) {
    // SAFETY : objet vivant.
    if unsafe { wl_proxy_get_version(proxy) } >= 3 {
        destroy!(proxy, opcode);
    } else {
        // SAFETY : objet vivant, oublié par l'appelant.
        unsafe { wl_proxy_destroy(proxy) };
    }
}
/// Taille des curseurs en pixels logiques (`XCURSOR_SIZE`, sinon 24).
fn cursor_size() -> i32 {
    std::env::var("XCURSOR_SIZE")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|v| (8..=256).contains(v))
        .unwrap_or(24)
}
