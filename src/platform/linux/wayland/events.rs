//! Événements du compositeur : registre, configuration et fermeture des fenêtres, rappels
//! de trame, tampons rendus, siège, sorties, presse-papiers, échelles et décorations.
use super::{
    Wayland,
    display::{self, Event, Kind, request},
    protocol,
    window::Window,
};

/// Échelle entière la plus grande acceptée du compositeur : un facteur absurde
/// déborderait les tailles de tampon et de curseur.
const MAX_SCALE: i32 = 8;

impl Wayland {
    /// Traite les événements mis en file par le répartiteur.
    pub(super) fn process(&mut self) {
        loop {
            let events = display::take_events();
            if events.is_empty() {
                break;
            }
            for event in events {
                self.handle(event);
            }
        }
    }
    fn window_by(&mut self, f: impl Fn(&Window) -> bool) -> Option<&mut Window> {
        self.windows.iter_mut().find(|w| f(w))
    }
    fn handle(&mut self, event: Event) {
        use protocol::*;
        let target = event.target;
        match event.kind {
            Kind::Registry => self.display.global(&event),
            Kind::WmBase if event.opcode == xdg_wm_base::EV_PING => {
                request!(
                    self.display.globals.wm_base,
                    xdg_wm_base::PONG,
                    event.uint(0)
                );
            }
            Kind::XdgSurface if event.opcode == xdg_surface::EV_CONFIGURE => {
                if let Some(w) = self.window_by(|w| w.xdg == target) {
                    w.configure(event.uint(0));
                }
            }
            Kind::Toplevel => match event.opcode {
                xdg_toplevel::EV_CONFIGURE => {
                    let (width, height) = (event.int(0), event.int(1));
                    if let Some(w) = self.window_by(|w| w.toplevel == target) {
                        w.propose(width, height, event.array(2));
                    }
                }
                xdg_toplevel::EV_CLOSE => {
                    if let Some(id) = self
                        .windows
                        .iter()
                        .find(|w| w.toplevel == target)
                        .map(|w| w.session.id)
                    {
                        self.closed.push(id);
                    }
                }
                _ => {}
            },
            Kind::Surface => self.surface_event(&event),
            Kind::Callback => {
                for w in &mut self.windows {
                    if w.frame_done(target) {
                        break;
                    }
                }
            }
            Kind::Buffer => {
                for w in &mut self.windows {
                    if w.released(target) {
                        break;
                    }
                }
            }
            Kind::Seat if event.opcode == wl_seat::EV_CAPABILITIES => {
                self.seat.capabilities(event.uint(0));
            }
            Kind::Pointer => self.pointer_event(&event),
            Kind::Keyboard => self.keyboard_event(event),
            Kind::Output if event.opcode == wl_output::EV_SCALE => {
                let factor = event.int(0).clamp(1, MAX_SCALE);
                if let Some(output) = self
                    .display
                    .globals
                    .outputs
                    .iter_mut()
                    .find(|(_, o, _)| *o == target)
                {
                    output.2 = factor;
                }
                self.refresh_scales();
            }
            Kind::DataDevice | Kind::DataOffer | Kind::DataSource => self.clipboard.event(event),
            Kind::TextInput => self.text_input_event(&event),
            Kind::Fractional if event.opcode == wp_fractional_scale_v1::EV_PREFERRED_SCALE => {
                let scale = event.uint(0);
                if let Some(w) = self.window_by(|w| w.fractional == target) {
                    w.scale120 = Some(scale.clamp(1, MAX_SCALE as u32 * 120));
                    w.rescale();
                    w.session.dirty = true;
                }
            }
            Kind::Exported if event.opcode == protocol::zxdg_exported::EV_HANDLE => {
                let handle = event.string(0).to_owned();
                if let Some(w) = self.window_by(|w| w.exported == target) {
                    w.foreign_handle = Some(handle);
                }
            }
            Kind::Decoration if event.opcode == zxdg_toplevel_decoration_v1::EV_CONFIGURE => {
                let mode = event.uint(0);
                if let Some(w) = self.window_by(|w| w.decoration == target) {
                    w.decorated(mode);
                }
            }
            _ => {}
        }
    }
    /// Entrée et sortie d'une sortie, échelle entière préférée.
    fn surface_event(&mut self, event: &Event) {
        use protocol::wl_surface;
        let target = event.target;
        let Some(w) = self.window_by(|w| w.surface == target) else {
            return;
        };
        match event.opcode {
            wl_surface::EV_ENTER => w.outputs.push(event.object(0)),
            wl_surface::EV_LEAVE => {
                let output = event.object(0);
                w.outputs.retain(|o| *o != output);
            }
            wl_surface::EV_PREFERRED_BUFFER_SCALE => {
                w.preferred_scale = Some(event.int(0).clamp(1, MAX_SCALE));
            }
            _ => return,
        }
        self.refresh_scales();
    }
    /// Échelle entière de chaque fenêtre : préférée par le compositeur, sinon la plus
    /// grande des sorties où elle est affichée.
    fn refresh_scales(&mut self) {
        let outputs = &self.display.globals.outputs;
        for w in &mut self.windows {
            let scale = w.preferred_scale.unwrap_or_else(|| {
                outputs
                    .iter()
                    .filter(|(_, o, _)| w.outputs.contains(o))
                    .map(|(.., s)| *s)
                    .max()
                    .unwrap_or(1)
            });
            if scale != w.integer_scale {
                w.integer_scale = scale;
                w.rescale();
                w.session.dirty = true;
            }
        }
    }
}
