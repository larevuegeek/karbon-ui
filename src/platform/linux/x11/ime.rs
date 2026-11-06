//! Méthode d'entrée des fenêtres X11 par IBus (`linux/ime/`) : quand un champ de saisie
//! modifiable a le focus, chaque touche va d'abord au moteur ; ses demandes deviennent les
//! événements de composition du moteur (`CompositionStart`, `CompositionUpdate`,
//! `CompositionCommit`, `CompositionEnd`, comme sous Wayland et macOS) ou du texte. Le
//! caret, en pixels de l'écran, place la fenêtre des candidats. Un clic dans le contenu ou
//! la perte du focus valide la composition en cours.
use super::X11;
use crate::{platform::linux::ime::Request, window::WindowId};
use std::time::Instant;

impl X11 {
    /// Fenêtre qui a le clavier.
    fn keyboard_window(&self) -> Option<WindowId> {
        self.windows
            .iter()
            .find(|w| w.session.focused)
            .map(|w| w.session.id)
    }
    /// Transmet la touche au moteur si un champ de saisie a le focus ; `true` s'il l'a
    /// prise (elle ne va pas à l'application).
    pub(super) fn ime_key(&mut self, keysym: u32, code: u32, state: u32, pressed: bool) -> bool {
        let Some(ime) = self.ime.as_ref().filter(|i| i.focused()) else {
            return false;
        };
        let taken = ime.key(keysym, code, state, pressed);
        self.ime_requests();
        taken
    }
    /// Applique les demandes reçues du moteur.
    pub(super) fn ime_requests(&mut self) {
        let Some(ime) = &self.ime else {
            return;
        };
        let requests = ime.requests();
        let Some(id) = self.keyboard_window() else {
            return;
        };
        for request in requests {
            match request {
                Request::Commit(text) => {
                    for event in self.composition.commit(text) {
                        self.send(id, event);
                    }
                }
                Request::Preedit(text, cursor) => {
                    let cursor_utf16 = text.chars().take(cursor).map(char::len_utf16).sum();
                    for event in self.composition.update(text, cursor_utf16) {
                        self.send(id, event);
                    }
                }
                Request::Forward { keycode, state, .. } => {
                    let pressed = state & 1 << 30 == 0;
                    for event in self.keys.key(keycode, pressed, Instant::now()) {
                        self.send(id, event);
                    }
                }
            }
        }
    }
    /// Valide dans le champ la composition en cours, et remet le moteur à zéro.
    pub(super) fn finish_composition(&mut self) {
        let events = self.composition.finish();
        if events.is_empty() {
            return;
        }
        if let Some(id) = self.keyboard_window() {
            for event in events {
                self.send(id, event);
            }
        }
        if let Some(ime) = &self.ime {
            ime.reset();
        }
    }
    /// Suit le champ de saisie de la fenêtre qui a le clavier : focus du contexte et caret
    /// à l'écran.
    pub(super) fn sync_ime(&mut self) {
        let Some(ime) = &self.ime else {
            return;
        };
        let focused = ime.focused();
        let connection = &self.connection;
        let wanted = self
            .windows
            .iter()
            .find(|w| w.session.focused)
            .and_then(|w| {
                let geometry = w.session.runtime.focused_input_geometry()?;
                if geometry.read_only {
                    return None;
                }
                let (x, y) = w.origin(connection)?;
                let (c, s) = (geometry.caret, w.session.scale);
                Some([
                    x + (c.left * s).floor() as i32,
                    y + (c.top * s).floor() as i32,
                    ((c.right - c.left) * s).ceil().max(1.) as i32,
                    ((c.bottom - c.top) * s).ceil().max(1.) as i32,
                ])
            });
        match (wanted, &mut self.ime) {
            (Some(caret), Some(ime)) => {
                ime.focus(true);
                ime.cursor(caret);
            }
            (None, _) if focused => {
                self.finish_composition();
                if let Some(ime) = &mut self.ime {
                    ime.focus(false);
                }
            }
            _ => {}
        }
    }
}
