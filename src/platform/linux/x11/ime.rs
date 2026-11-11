//! Méthode d'entrée des fenêtres X11 par IBus (`linux/ime/`) : quand un champ de saisie
//! modifiable a le focus, chaque touche va d'abord au moteur ; ses demandes deviennent les
//! événements de composition du moteur (`CompositionStart`, `CompositionUpdate`,
//! `CompositionCommit`, `CompositionEnd`, comme sous Wayland et macOS) ou du texte. Le
//! caret, en pixels de l'écran, place la fenêtre des candidats. Un clic dans le contenu ou
//! la perte du focus valide la composition en cours.
//!
//! Les touches partent au moteur sans attendre sa réponse (comme GTK) : elles attendent
//! dans `held`, avec leur état, et vont à l'application dans l'ordre d'arrivée quand le
//! moteur les refuse (rien n'est doublé ni perdu, même avec un moteur lent). Une touche
//! frappée pendant l'attente passe derrière elles ; sans réponse après `KEY_WAIT`, une
//! touche est rendue à l'application.
use super::X11;
use crate::{platform::linux::ime::Request, window::WindowId};
use std::time::{Duration, Instant};

/// Attente au plus de la réponse du moteur à une touche.
pub(super) const KEY_WAIT: Duration = Duration::from_secs(1);

/// Touche en attente : envoyée au moteur (`serial`), ou derrière une touche qui l'est.
pub(super) struct HeldKey {
    serial: Option<u32>,
    /// Réponse reçue : prise par le moteur ou non.
    taken: Option<bool>,
    code: u32,
    /// Modificateurs et groupe de l'événement X, rétablis avant de la traiter.
    state: u32,
    pressed: bool,
    repeat: bool,
    window: WindowId,
    sent: Instant,
}
impl HeldKey {
    /// Échéance après laquelle la touche est rendue à l'application.
    pub(super) fn deadline(&self) -> Instant {
        self.sent + KEY_WAIT
    }
}

impl X11 {
    /// Fenêtre qui a le clavier.
    fn keyboard_window(&self) -> Option<WindowId> {
        self.windows
            .iter()
            .find(|w| w.session.focused)
            .map(|w| w.session.id)
    }
    /// Touche de la fenêtre `window` (`state` : état de l'événement X) : envoyée au moteur
    /// si un champ de saisie a le focus, puis gardée jusqu'à sa réponse ; gardée aussi
    /// derrière des touches en attente. `false` : à traiter tout de suite.
    pub(super) fn ime_key(
        &mut self,
        window: WindowId,
        (keysym, code, state): (u32, u32, u32),
        pressed: bool,
        repeat: bool,
    ) -> bool {
        let serial = self
            .ime
            .as_ref()
            .filter(|i| i.focused())
            .and_then(|ime| ime.key(keysym, code, state & 0xffff, pressed || repeat));
        if serial.is_none() && self.held.is_empty() {
            return false;
        }
        self.held.push_back(HeldKey {
            serial,
            taken: None,
            code,
            state,
            pressed,
            repeat,
            window,
            sent: Instant::now(),
        });
        self.ime_requests();
        true
    }
    /// Touches en tête de file dont le sort est connu : rendues à l'application dans
    /// l'ordre (refusées, sans moteur, ou sans réponse à temps), oubliées si prises.
    fn release_held(&mut self) {
        let now = Instant::now();
        while let Some(front) = self.held.front() {
            let late = front.taken.is_none() && front.serial.is_some() && now >= front.deadline();
            if front.serial.is_some() && front.taken.is_none() && !late {
                return;
            }
            let key = self.held.pop_front().expect("front checked above");
            if key.taken == Some(true) {
                continue;
            }
            self.keys
                .modifiers(key.state & 0xff, 0, 0, (key.state >> 13) & 3);
            let events = if key.repeat {
                self.keys.repeated(key.code)
            } else {
                self.keys.key(key.code, key.pressed, now)
            };
            for event in events {
                self.send(key.window, event);
            }
        }
    }
    /// Applique les demandes reçues du moteur.
    pub(super) fn ime_requests(&mut self) {
        let Some(ime) = &self.ime else {
            return;
        };
        let requests = ime.requests();
        for request in &requests {
            if let Request::Answered { serial, taken } = *request
                && let Some(key) = self.held.iter_mut().find(|k| k.serial == Some(serial))
            {
                key.taken = Some(taken);
            }
        }
        // Les touches refusées avant ces demandes passent d'abord, dans l'ordre.
        self.release_held();
        let Some(id) = self.keyboard_window() else {
            return;
        };
        for request in requests {
            match request {
                Request::Answered { .. } => {}
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
