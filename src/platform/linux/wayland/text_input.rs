//! Saisie par méthode d'entrée (`zwp_text_input_v3`, IBus sous GNOME) : activée tant
//! qu'un champ modifiable a le focus dans la fenêtre qui a le clavier, avec le rectangle
//! du caret (fenêtre des candidats) et le texte environnant. Les événements reçus
//! jusqu'à `done` sont appliqués dans l'ordre du protocole : suppression autour du caret,
//! texte validé, puis nouveau texte en composition, traduits dans la séquence du moteur
//! (`CompositionStart`, `CompositionUpdate`…, `CompositionCommit`, `CompositionEnd`).
//!
//! Numéro de `done` : celui des `commit` vus par le compositeur. Un `done` en retard sur
//! les nôtres est appliqué, mais l'état (caret, texte environnant) n'est renvoyé qu'après
//! un `done` à jour, comme le demande le protocole (certaines méthodes d'entrée bouclent
//! sinon), ou au plus tard après `STALE_WAIT`.
use super::{
    Wayland,
    display::{Event, Kind, NEW, Ptr, create, destroy, request},
    protocol::{ZWP_TEXT_INPUT_V3, zwp_text_input_manager_v3, zwp_text_input_v3},
};
use crate::{
    platform::linux::shell::composition::Composition,
    ui::runtime::{Key, UiEvent},
};

/// Texte environnant au plus (limite du protocole, 4000 octets).
const SURROUNDING: usize = 4000;
/// Attente au plus d'un `done` à jour avant de renvoyer l'état malgré tout.
const STALE_WAIT: std::time::Duration = std::time::Duration::from_millis(200);

/// État annoncé à la méthode d'entrée : caret (unités logiques) et texte environnant
/// (texte, caret et ancre en octets).
#[derive(Clone, PartialEq)]
struct Announced {
    caret: [i32; 4],
    surrounding: Option<(String, usize, usize)>,
}
/// Événements reçus depuis le dernier `done`.
#[derive(Default)]
struct Pending {
    preedit: Option<(String, i32)>,
    commit: Option<String>,
    delete: (u32, u32),
}
pub(super) struct TextInput {
    object: Ptr,
    /// Surface où le compositeur dirige la saisie.
    entered: Ptr,
    enabled: Option<Announced>,
    pending: Pending,
    /// Nombre de `commit` envoyés (numéro attendu dans `done`).
    commits: u32,
    /// Dernier `done` en retard sur nos `commit` : l'état attend un `done` à jour.
    stale: Option<std::time::Instant>,
    /// Texte en composition affiché dans le champ.
    pub composition: Composition,
}
impl TextInput {
    pub fn new(manager: Ptr, seat: Ptr) -> Self {
        let object = if manager.is_null() || seat.is_null() {
            std::ptr::null_mut()
        } else {
            create!(
                manager,
                zwp_text_input_manager_v3::GET_TEXT_INPUT,
                &ZWP_TEXT_INPUT_V3,
                Kind::TextInput,
                NEW,
                seat
            )
        };
        Self {
            object,
            entered: std::ptr::null_mut(),
            enabled: None,
            pending: Pending::default(),
            commits: 0,
            stale: None,
            composition: Composition::default(),
        }
    }
    /// Envoie l'état en attente (`commit`), compté pour les numéros de `done`.
    fn commit(&mut self) {
        request!(self.object, zwp_text_input_v3::COMMIT);
        self.commits = self.commits.wrapping_add(1);
    }
    /// L'état peut être renvoyé : dernier `done` à jour, ou attente écoulée.
    fn may_update(&self) -> bool {
        self.stale.is_none_or(|since| since.elapsed() >= STALE_WAIT)
    }
    fn announce(&mut self, state: &Announced) {
        let [x, y, w, h] = state.caret;
        request!(
            self.object,
            zwp_text_input_v3::SET_CURSOR_RECTANGLE,
            x,
            y,
            w,
            h
        );
        if let Some((text, cursor, anchor)) = &state.surrounding {
            let text = super::display::text(text);
            request!(
                self.object,
                zwp_text_input_v3::SET_SURROUNDING_TEXT,
                text.as_ptr(),
                *cursor as i32,
                *anchor as i32
            );
        }
        self.commit();
        self.stale = None;
    }
    pub fn close(&mut self) {
        destroy!(self.object, zwp_text_input_v3::DESTROY);
        self.object = std::ptr::null_mut();
    }
}
impl Drop for TextInput {
    fn drop(&mut self) {
        self.close();
    }
}
impl Wayland {
    pub(super) fn text_input_event(&mut self, event: &Event) {
        let ime = &mut self.text_input;
        match event.opcode {
            zwp_text_input_v3::EV_ENTER => ime.entered = event.object(0),
            zwp_text_input_v3::EV_LEAVE => {
                ime.entered = std::ptr::null_mut();
                ime.enabled = None;
            }
            zwp_text_input_v3::EV_PREEDIT_STRING => {
                ime.pending.preedit = Some((event.string(0).to_owned(), event.int(1)));
            }
            zwp_text_input_v3::EV_COMMIT_STRING => {
                ime.pending.commit = Some(event.string(0).to_owned());
            }
            zwp_text_input_v3::EV_DELETE_SURROUNDING_TEXT => {
                ime.pending.delete = (event.uint(0), event.uint(1));
            }
            zwp_text_input_v3::EV_DONE => {
                ime.stale = (event.uint(0) != ime.commits).then(std::time::Instant::now);
                self.text_input_done();
            }
            _ => {}
        }
    }
    /// `done` : applique les changements reçus au champ focalisé.
    fn text_input_done(&mut self) {
        let Pending {
            preedit,
            commit,
            delete: (before, after),
        } = std::mem::take(&mut self.text_input.pending);
        let Some(id) = self.seat.keyboard_window else {
            return;
        };
        // Ordre du protocole : la composition affichée est retirée, puis la suppression
        // autour du caret, le texte validé, et enfin la nouvelle composition (absente :
        // le champ n'en a plus).
        let deleting = before > 0 || after > 0;
        if deleting || commit.is_some() {
            for event in self.text_input.composition.discard() {
                self.send(id, event);
            }
        }
        if deleting {
            // Suppression en octets autour du caret, rejouée en caractères.
            let snapshot = self
                .windows
                .iter()
                .find(|w| w.session.id == id)
                .and_then(|w| w.session.runtime.focused_input());
            if let Some(s) = snapshot {
                let caret = s.selection.end.min(s.value.len());
                let start = caret.saturating_sub(before as usize);
                let end = (caret + after as usize).min(s.value.len());
                let count = |a: usize, b: usize| s.value.get(a..b).map_or(0, |t| t.chars().count());
                for (key, n) in [
                    (Key::Backspace, count(start, caret)),
                    (Key::Delete, count(caret, end)),
                ] {
                    for _ in 0..n {
                        self.send(
                            id,
                            UiEvent::KeyDown {
                                key,
                                shift: false,
                                repeat: false,
                            },
                        );
                        self.send(id, UiEvent::KeyUp(key));
                    }
                }
            }
        }
        if let Some(text) = commit {
            for event in self.text_input.composition.commit(text) {
                self.send(id, event);
            }
        }
        let events = match preedit {
            Some((text, cursor)) => {
                let byte = (cursor.max(0) as usize).min(text.len());
                let cursor_utf16 = text.get(..byte).map_or(0, |t| t.encode_utf16().count());
                self.text_input.composition.update(text, cursor_utf16)
            }
            None => self.text_input.composition.discard(),
        };
        for event in events {
            self.send(id, event);
        }
    }
    /// Valide la composition en cours dans le champ (clic ailleurs, perte du focus) et
    /// remet la méthode d'entrée à zéro.
    pub(super) fn finish_composition(&mut self) {
        let events = self.text_input.composition.finish();
        if events.is_empty() {
            return;
        }
        if let Some(id) = self.seat.keyboard_window {
            for event in events {
                self.send(id, event);
            }
        }
        let ime = &mut self.text_input;
        if ime.enabled.take().is_some() {
            request!(ime.object, zwp_text_input_v3::DISABLE);
            ime.commit();
        }
    }
    /// Active, met à jour ou désactive la saisie selon le champ focalisé de la fenêtre
    /// qui a le clavier.
    pub(super) fn sync_text_input(&mut self) {
        if self.text_input.object.is_null() {
            return;
        }
        let entered = self.text_input.entered;
        let wanted = self
            .seat
            .keyboard_window
            .and_then(|id| self.windows.iter().find(|w| w.session.id == id))
            .filter(|w| !entered.is_null() && w.surface == entered)
            .and_then(|w| {
                let geometry = w.session.runtime.focused_input_geometry()?;
                if geometry.read_only {
                    return None;
                }
                let c = geometry.caret;
                let caret = [
                    c.left.floor() as i32,
                    c.top.floor() as i32,
                    (c.right - c.left).ceil().max(1.) as i32,
                    (c.bottom - c.top).ceil().max(1.) as i32,
                ];
                let surrounding = w
                    .session
                    .runtime
                    .focused_input()
                    .filter(|s| s.value.len() <= SURROUNDING && !s.value.contains('\0'))
                    .map(|s| (s.value, s.selection.end, s.selection.start));
                Some(Announced { caret, surrounding })
            });
        let ime = &mut self.text_input;
        match (wanted, &ime.enabled) {
            (Some(state), None) => {
                request!(ime.object, zwp_text_input_v3::ENABLE);
                // Texte libre, sans indication (`hint` et `purpose` à 0).
                request!(ime.object, zwp_text_input_v3::SET_CONTENT_TYPE, 0u32, 0u32);
                ime.announce(&state);
                ime.enabled = Some(state);
            }
            (Some(state), Some(previous)) if *previous != state && ime.may_update() => {
                ime.announce(&state);
                ime.enabled = Some(state);
            }
            (None, Some(_)) => {
                ime.enabled = None;
                request!(ime.object, zwp_text_input_v3::DISABLE);
                ime.commit();
            }
            _ => {}
        }
    }
}
