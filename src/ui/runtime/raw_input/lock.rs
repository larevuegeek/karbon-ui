//! Pointeur verrouillé (`Element::pointer_lock`) et mouvement relatif
//! (`PointerEvent::delta`) : pendant le geste d'un élément qui le demande, le backend
//! cache le pointeur, le garde à sa place et donne le mouvement propre de la souris, qui
//! ne bute jamais au bord de l'écran (caméra libre, champ qu'on fait glisser sans fin).
use super::super::*;
use super::{PointerButton, PointerEvent, PointerInput, PointerPhase, pointer::bit};

impl super::RawPointerState {
    /// Le mouvement depuis l'événement précédent : celui de la souris quand le pointeur
    /// est verrouillé, sinon l'écart des positions ; rien pour un appui ou un relâchement.
    pub(crate) fn delta_of(&mut self, input: &PointerInput) -> (f32, f32) {
        let delta = match (input.phase, input.motion) {
            (PointerPhase::Move, Some(motion)) => motion,
            (PointerPhase::Move, None) => self.last_position.map_or((0., 0.), |last| {
                (input.position.x - last.x, input.position.y - last.y)
            }),
            _ => (0., 0.),
        };
        self.last_position = Some(input.position);
        delta
    }
}

impl UiRuntime {
    /// The zoom of the interface chosen by the application
    /// ([`Application::interface_zoom`](crate::application::Application::interface_zoom)),
    /// kept between 0.5 and 3 (`1.0` when it is not a finite number).
    pub fn interface_zoom(&self) -> f32 {
        let zoom = self.app.interface_zoom();
        if zoom.is_finite() {
            zoom.clamp(0.5, 3.)
        } else {
            1.
        }
    }

    /// L'élément qui tient le geste en cours a demandé le verrouillage du pointeur : le
    /// backend le cache et le garde à sa place jusqu'à la fin du geste.
    pub(crate) fn pointer_lock_wanted(&self) -> bool {
        let Some(id) = self.raw_pointer.target else {
            return false;
        };
        self.root
            .as_ref()
            .and_then(|root| root.find(id))
            .and_then(|node| node.listeners.as_ref())
            .is_some_and(|listeners| listeners.pointer_lock)
    }

    /// Le verrouillage est perdu sans relâchement (la fenêtre a perdu le clavier, une
    /// autre a pris la souris) : l'élément reçoit le relâchement de chaque bouton encore
    /// tenu, à la dernière position, et le geste finit. Rien ne se passe sans geste
    /// verrouillé : les autres gestes gardent leur comportement.
    // Seul le backend Windows verrouille le pointeur pour l'instant.
    #[cfg_attr(not(windows), allow(dead_code))]
    pub(crate) fn pointer_lock_lost(&mut self) -> EventResult {
        let mut result = EventResult::default();
        if !self.pointer_lock_wanted() {
            return result;
        }
        let state = &mut self.raw_pointer;
        let (Some(id), held) = (state.target.take(), std::mem::take(&mut state.held)) else {
            return result;
        };
        let at = state.last_position.unwrap_or(PointF { x: 0., y: 0. });
        let held_element = self
            .root
            .as_ref()
            .and_then(|root| root.find(id))
            .and_then(|node| {
                let handler = node.listeners.as_ref()?.pointer.clone()?;
                Some((node.rect, handler))
            });
        let Some((rect, handler)) = held_element else {
            return result;
        };
        let buttons = [
            PointerButton::Primary,
            PointerButton::Secondary,
            PointerButton::Middle,
            PointerButton::Back,
            PointerButton::Forward,
        ];
        for button in buttons.into_iter().filter(|&b| held & bit(b) != 0) {
            let event = PointerEvent {
                phase: PointerPhase::Up,
                button: Some(button),
                x: at.x,
                y: at.y,
                local_x: at.x - rect.left,
                local_y: at.y - rect.top,
                shift: false,
                control: false,
                alt: false,
                clicks: 0,
                wheel: (0., 0.),
                meta: false,
                delta: (0., 0.),
                pressure: 1.,
                tilt: (0., 0.),
                screen: self.raw_pointer.extras.on_screen(at),
            };
            if let Some(message) = (handler.0)(event) {
                self.send(message);
                result.handled = true;
                result.repaint = true;
                self.invalidation.repaint = true;
            }
        }
        result
    }
}
