//! Entrées du siège vers les fenêtres : pointeur (barre de titre et bords du moteur,
//! zones de déplacement, clics, menu contextuel, défilement par trame), clavier (focus,
//! touches et répétitions) et curseur affiché.
use super::{
    Wayland,
    display::{Event, request},
    protocol::{wl_keyboard, wl_pointer, xdg_toplevel},
    seat::Scroll,
    window::MARGIN,
};
use crate::platform::linux::shell::{
    chrome::{Edge, Press, cursor_names},
    scroll::{self, NOTCH},
};
use crate::ui::{runtime::UiEvent, scene::PointF};
use std::time::Instant;

const BUTTON_LEFT: u32 = 0x110;
const BUTTON_RIGHT: u32 = 0x111;
const PRESSED: u32 = 1;

impl Wayland {
    fn window_at_surface(&self, surface: super::display::Ptr) -> Option<crate::window::WindowId> {
        self.windows
            .iter()
            .find(|w| w.surface == surface)
            .map(|w| w.session.id)
    }
    /// Fenêtre dont `surface` est la bordure de redimensionnement.
    fn window_at_border(&self, surface: super::display::Ptr) -> Option<crate::window::WindowId> {
        self.windows
            .iter()
            .find(|w| w.is_border(surface))
            .map(|w| w.session.id)
    }
    /// Position logique dans la fenêtre d'un point de la surface sous le pointeur (la
    /// bordure commence `MARGIN` unités avant le contenu).
    fn window_point(&self, x: f64, y: f64) -> PointF {
        let shift = if self.seat.pointer_border {
            MARGIN as f32
        } else {
            0.
        };
        PointF {
            x: x as f32 - shift,
            y: y as f32 - shift,
        }
    }
    pub(super) fn pointer_event(&mut self, event: &Event) {
        match event.opcode {
            wl_pointer::EV_ENTER => {
                self.seat.entered(event.uint(0));
                let surface = event.object(1);
                let border = self.window_at_border(surface);
                self.seat.pointer_border = border.is_some();
                self.seat.pointer_window = border.or_else(|| self.window_at_surface(surface));
                self.seat.position = self.window_point(event.fixed(2), event.fixed(3));
                self.moved();
            }
            wl_pointer::EV_LEAVE => {
                // Le contenu a déjà reçu sa sortie en passant sur la bordure.
                if std::mem::take(&mut self.seat.pointer_border) {
                    self.seat.pointer_window = None;
                } else if let Some(id) = self.seat.pointer_window.take() {
                    if let Some(w) = self.windows.iter_mut().find(|w| w.session.id == id) {
                        w.session.leave();
                    }
                    self.send(id, UiEvent::PointerLeave);
                }
            }
            wl_pointer::EV_MOTION => {
                self.seat.position = self.window_point(event.fixed(1), event.fixed(2));
                self.moved();
            }
            wl_pointer::EV_BUTTON => {
                self.seat.serial = event.uint(0);
                let (button, state) = (event.uint(2), event.uint(3));
                self.button(button, state == PRESSED);
            }
            wl_pointer::EV_AXIS => {
                let axis = (event.uint(1) as usize).min(1);
                self.seat.scroll.value[axis] += event.fixed(2);
            }
            wl_pointer::EV_AXIS_VALUE120 => {
                let axis = (event.uint(0) as usize).min(1);
                self.seat.scroll.value120[axis] += event.int(1);
            }
            wl_pointer::EV_AXIS_DISCRETE => {
                // Compositeurs sans `wl_pointer` 8 : un cran entier par unité, sinon la
                // molette défilerait des pixels (`axis`) au lieu de crans.
                let axis = (event.uint(0) as usize).min(1);
                self.seat.scroll.value120[axis] += event.int(1).saturating_mul(120);
            }
            wl_pointer::EV_FRAME => self.scrolled(),
            _ => {}
        }
    }
    /// Le pointeur a bougé : survol des boutons et des bords, puis le contenu.
    fn moved(&mut self) {
        let Some(id) = self.seat.pointer_window else {
            return;
        };
        if self.seat.pointer_border {
            return;
        }
        let position = self.seat.position;
        if let Some(w) = self.windows.iter_mut().find(|w| w.session.id == id) {
            w.session.hover(position);
        }
        self.send(id, UiEvent::PointerMove(position));
    }
    fn button(&mut self, button: u32, pressed: bool) {
        let Some(id) = self.seat.pointer_window else {
            return;
        };
        let position = self.seat.position;
        let (seat, serial) = (self.seat.seat, self.seat.serial);
        if self.seat.pointer_border {
            // Bordure : seul le bouton principal sert, à redimensionner.
            let edge = self
                .windows
                .iter()
                .find(|w| w.session.id == id)
                .and_then(|w| Some((w.toplevel, w.session.outer_edge(position)?.0)));
            if let (BUTTON_LEFT, true, Some((toplevel, edge))) = (button, pressed, edge) {
                request!(toplevel, xdg_toplevel::RESIZE, seat, serial, xdg_edge(edge));
            }
            return;
        }
        match (button, pressed) {
            (BUTTON_LEFT, true) => {
                let now = Instant::now();
                let Some(press) = self.with(id, |w| w.session.press(position, now)) else {
                    return;
                };
                let Some(w) = self.windows.iter().find(|w| w.session.id == id) else {
                    return;
                };
                match press {
                    Press::Resize(edge) => {
                        request!(
                            w.toplevel,
                            xdg_toplevel::RESIZE,
                            seat,
                            serial,
                            xdg_edge(edge)
                        )
                    }
                    Press::Move => request!(w.toplevel, xdg_toplevel::MOVE, seat, serial),
                    Press::Maximize => w.set_maximized(!w.session.maximized),
                    Press::Button => {}
                    Press::Content => {
                        // Un clic valide la composition en cours, comme sous macOS.
                        self.finish_composition();
                        // Position à jour avant l'appui (un menu a pu apparaître sous un
                        // pointeur immobile), comme `HeadlessApp::click_at`.
                        self.send(id, UiEvent::PointerMove(position));
                        let down = if self.seat.keys.shift() {
                            UiEvent::PointerDownExtended(position)
                        } else {
                            UiEvent::PointerDown(position)
                        };
                        self.send(id, down);
                    }
                }
            }
            (BUTTON_LEFT, false) => {
                let released = self
                    .windows
                    .iter_mut()
                    .find(|w| w.session.id == id)
                    .and_then(|w| w.session.release(position));
                match released {
                    Some(Some(action)) => self.action(id, action),
                    Some(None) => {}
                    None => self.send(id, UiEvent::PointerUp(position)),
                }
            }
            (BUTTON_RIGHT, true) => {
                let caption = self
                    .windows
                    .iter()
                    .find(|w| w.session.id == id)
                    .filter(|w| w.session.caption_at(position));
                match caption {
                    Some(w) => request!(
                        w.toplevel,
                        xdg_toplevel::SHOW_WINDOW_MENU,
                        seat,
                        serial,
                        position.x as i32,
                        position.y as i32
                    ),
                    None => self.send(id, UiEvent::ContextMenu(Some(position))),
                }
            }
            _ => {}
        }
    }
    /// Fin d'une trame du pointeur : défilement accumulé (crans de molette, sinon pixels
    /// d'un pavé tactile). Maj fait défiler horizontalement, comme sous Windows.
    fn scrolled(&mut self) {
        let Scroll { value, value120 } = std::mem::take(&mut self.seat.scroll);
        let Some(id) = self.seat.pointer_window else {
            return;
        };
        let delta = |axis: usize| {
            if value120[axis] != 0 {
                value120[axis] as f32 / 120. * NOTCH[axis]
            } else {
                value[axis] as f32
            }
        };
        let Some((x, y)) = scroll::delta(delta(1), delta(0), self.seat.keys.shift()) else {
            return;
        };
        let position = self.seat.position;
        // Crans de molette : glissement doux ; pixels d'un pavé tactile : immédiats.
        let event = if value120 != [0, 0] {
            UiEvent::Wheel {
                position,
                delta_x: x,
                delta_y: y,
            }
        } else {
            UiEvent::Scroll {
                position,
                delta_x: x,
                delta_y: y,
            }
        };
        self.send(id, event);
    }
    pub(super) fn keyboard_event(&mut self, mut event: Event) {
        match event.opcode {
            wl_keyboard::EV_KEYMAP => {
                let (format, size) = (event.uint(0), event.uint(2));
                let fd = event.take_fd(1);
                self.seat.keys.keymap(format, fd, size);
            }
            wl_keyboard::EV_ENTER => {
                crate::platform::linux::desktop::refresh();
                self.seat.serial = event.uint(0);
                self.seat.keyboard_window = self.window_at_surface(event.object(1));
                if let Some(w) = self
                    .windows
                    .iter_mut()
                    .find(|w| Some(w.session.id) == self.seat.keyboard_window)
                {
                    w.session.focused = true;
                }
                if let Some(id) = self.seat.keyboard_window {
                    self.send(id, UiEvent::FocusChanged(true));
                }
            }
            wl_keyboard::EV_LEAVE => {
                self.finish_composition();
                self.seat.keys.reset();
                if let Some(id) = self.seat.keyboard_window.take() {
                    if let Some(w) = self.windows.iter_mut().find(|w| w.session.id == id) {
                        w.session.focused = false;
                    }
                    self.send(id, UiEvent::Cancel);
                    self.send(id, UiEvent::FocusChanged(false));
                }
            }
            wl_keyboard::EV_KEY => {
                self.seat.serial = event.uint(0);
                let (code, state) = (event.uint(2), event.uint(3));
                let events = self.seat.keys.key(code, state == PRESSED, Instant::now());
                if let Some(id) = self.seat.keyboard_window {
                    for ui in events {
                        self.send(id, ui);
                    }
                }
            }
            wl_keyboard::EV_MODIFIERS => {
                let m = (event.uint(1), event.uint(2), event.uint(3), event.uint(4));
                self.seat.keys.modifiers(m.0, m.1, m.2, m.3);
            }
            wl_keyboard::EV_REPEAT_INFO => self.seat.keys.repeat_info(event.int(0), event.int(1)),
            _ => {}
        }
    }
    /// Curseur de la fenêtre sous le pointeur : bord de redimensionnement, sinon celui
    /// demandé par le runtime.
    pub(super) fn update_cursor(&mut self) {
        let Some(id) = self.seat.pointer_window else {
            return;
        };
        let position = self.seat.position;
        let Some(w) = self.windows.iter_mut().find(|w| w.session.id == id) else {
            return;
        };
        let names = if self.seat.pointer_border {
            w.session
                .outer_edge(position)
                .map_or(&["default"][..], |(_, c)| c)
        } else {
            w.session
                .hover(position)
                .unwrap_or_else(|| cursor_names(w.session.runtime.cursor()))
        };
        let scale = w.scale().ceil() as i32;
        let (compositor, shm) = (self.display.globals.compositor, self.display.globals.shm);
        self.seat.show_cursor(compositor, shm, names, scale);
    }
}
/// Valeur de `xdg_toplevel.resize_edge` d'un bord.
fn xdg_edge(edge: Edge) -> u32 {
    match edge {
        Edge::Top => 1,
        Edge::Bottom => 2,
        Edge::Left => 4,
        Edge::TopLeft => 5,
        Edge::BottomLeft => 6,
        Edge::Right => 8,
        Edge::TopRight => 9,
        Edge::BottomRight => 10,
    }
}
