//! Événements AT-SPI émis après chaque passage de la boucle, seulement si un client
//! écoute : fenêtre activée ou désactivée, élément focalisé, texte inséré ou supprimé et
//! caret déplacé dans le champ focalisé, valeur d'un curseur focalisé.
use super::{Bridge, Host, tree::Object};
use crate::{platform::linux::ffi::dbus::Message, window::WindowId};
use std::sync::Arc;

/// État déjà annoncé aux clients.
#[derive(Default)]
pub(super) struct Announced {
    active: Option<WindowId>,
    focus: Option<Object>,
    /// Texte et caret (octets) du champ focalisé.
    text: Option<(Object, Arc<str>, usize)>,
    value: Option<(Object, f64)>,
}
/// Contenu de l'argument `any_data` d'un événement.
enum Data<'a> {
    Int(i32),
    Str(&'a str),
    F64(f64),
}
impl Bridge {
    /// Émet l'événement `(interface, membre)` sur `object`, avec son détail
    /// `(texte, nombre 1, nombre 2)` et ses données.
    fn emit(
        &self,
        object: Object,
        (interface, member): (&str, &str),
        (detail, d1, d2): (&str, i32, i32),
        data: Data<'_>,
    ) {
        let Some(mut signal) = Message::signal(&object.path(), interface, member) else {
            return;
        };
        {
            let mut w = signal.writer();
            w.string(detail).i32(d1).i32(d2);
            match data {
                Data::Int(v) => w.open(b'v', Some("i")).i32(v).close(),
                Data::Str(v) => w.open(b'v', Some("s")).string(v).close(),
                Data::F64(v) => w.open(b'v', Some("d")).f64(v).close(),
            };
            w.open(b'a', Some("{sv}")).close();
        }
        self.bus.send(&signal);
    }
    fn state(&self, object: Object, state: &str, on: bool) {
        self.emit(
            object,
            ("org.a11y.atspi.Event.Object", "StateChanged"),
            (state, i32::from(on), 0),
            Data::Int(0),
        );
    }
    /// Annonce les changements depuis le dernier appel.
    pub fn announce<H: Host>(&mut self, windows: &[H]) {
        if !self.listened() {
            // Plus personne n'écoute : l'état sera annoncé à nouveau au prochain client.
            self.announced = Announced::default();
            return;
        }
        self.refresh(windows);
        let active = windows.iter().find(|w| w.active()).map(|w| w.id());
        if active != self.announced.active {
            if let Some(old) = self.announced.active {
                let title = self
                    .trees
                    .get(&old)
                    .map_or(String::new(), |t| t.title.clone());
                self.emit(
                    Object::Window(old),
                    ("org.a11y.atspi.Event.Window", "Deactivate"),
                    ("", 0, 0),
                    Data::Str(&title),
                );
                self.state(Object::Window(old), "active", false);
            }
            if let Some(new) = active {
                let title = self
                    .trees
                    .get(&new)
                    .map_or(String::new(), |t| t.title.clone());
                self.emit(
                    Object::Window(new),
                    ("org.a11y.atspi.Event.Window", "Activate"),
                    ("", 0, 0),
                    Data::Str(&title),
                );
                self.state(Object::Window(new), "active", true);
            }
            self.announced.active = active;
        }
        let focus = active.and_then(|w| {
            let tree = self.trees.get(&w)?;
            tree.focused().map(|e| Object::Element(w, e))
        });
        if focus != self.announced.focus {
            // Un élément sorti de l'arbre (couche modale, vue reconstruite) n'est plus
            // résolu par les clients : sa perte du focus n'est pas annoncée.
            let exists = |o: Object| match o {
                Object::Element(w, e) => self.trees.get(&w).is_some_and(|t| t.get(e).is_some()),
                _ => false,
            };
            if let Some(old) = self.announced.focus.filter(|o| exists(*o)) {
                self.state(old, "focused", false);
            }
            if let Some(new) = focus {
                self.state(new, "focused", true);
            }
            self.announced.focus = focus;
            self.announced.text = None;
            self.announced.value = None;
        }
        let element = focus.and_then(|o| match o {
            Object::Element(w, e) => self.trees.get(&w)?.get(e),
            _ => None,
        });
        let text = element.and_then(|e| e.text.as_ref()).zip(focus);
        if let Some((t, object)) = text {
            let caret = t.selection.end;
            if let Some((_, old, old_caret)) =
                self.announced.text.take().filter(|(o, ..)| *o == object)
            {
                self.text_changed(object, &old, &t.value);
                if old_caret != caret || *old != *t.value {
                    let offset = t.value.get(..caret).map_or(0, |s| s.chars().count() as i32);
                    self.emit(
                        object,
                        ("org.a11y.atspi.Event.Object", "TextCaretMoved"),
                        ("", offset, 0),
                        Data::Int(0),
                    );
                }
            }
            self.announced.text = Some((object, Arc::clone(&t.value), caret));
        }
        let value = element.and_then(|e| e.range.as_ref()).zip(focus);
        if let Some((range, object)) = value {
            if self
                .announced
                .value
                .is_some_and(|(o, v)| o == object && v != range.value)
            {
                self.emit(
                    object,
                    ("org.a11y.atspi.Event.Object", "PropertyChange"),
                    ("accessible-value", 0, 0),
                    Data::F64(range.value),
                );
            }
            self.announced.value = Some((object, range.value));
        }
        self.bus.flush();
    }
    /// Texte supprimé puis inséré entre `old` et `new` (préfixe et suffixe communs
    /// exclus), en caractères.
    fn text_changed(&self, object: Object, old: &str, new: &str) {
        if old == new {
            return;
        }
        let prefix = old
            .char_indices()
            .zip(new.chars())
            .take_while(|((_, a), b)| a == b)
            .last()
            .map_or(0, |((i, c), _)| i + c.len_utf8());
        let (old_rest, new_rest) = (&old[prefix..], &new[prefix..]);
        let suffix = old_rest
            .chars()
            .rev()
            .zip(new_rest.chars().rev())
            .take_while(|(a, b)| a == b)
            .map(|(c, _)| c.len_utf8())
            .sum::<usize>();
        let deleted = &old_rest[..old_rest.len() - suffix];
        let inserted = &new_rest[..new_rest.len() - suffix];
        let start = old[..prefix].chars().count() as i32;
        let event = "org.a11y.atspi.Event.Object";
        if !deleted.is_empty() {
            self.emit(
                object,
                (event, "TextChanged"),
                ("delete", start, deleted.chars().count() as i32),
                Data::Str(deleted),
            );
        }
        if !inserted.is_empty() {
            self.emit(
                object,
                (event, "TextChanged"),
                ("insert", start, inserted.chars().count() as i32),
                Data::Str(inserted),
            );
        }
    }
}
