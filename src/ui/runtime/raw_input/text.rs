//! Le texte tapé donné à un élément dessiné par l'application (`Element::on_text`) : un
//! éditeur de code, un champ de recherche d'une toile, un renommage sur place. Il reçoit
//! les caractères (touches mortes et méthode de saisie comprises), pas les touches.
//!
//! Le texte part de l'élément qui a le clavier (`on_key` : le focus, ou le dernier
//! panneau où l'on a appuyé) et remonte à ses ancêtres jusqu'au premier gestionnaire qui
//! le prend. Un champ de texte du runtime qui a le focus garde tout son texte, comme avant.
//! La fenêtre de la méthode de saisie (japonais, chinois) se place au curseur que dit
//! l'élément (`Element::text_caret`).
use super::super::*;
use std::rc::Rc;

/// Text typed while an element with an
/// [`Element::on_text`](crate::ui::element::Element::on_text) handler has the keyboard.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TextEvent {
    /// Characters to insert at the caret: a key, a dead key followed by a letter (« ê »),
    /// or the text an input method has just committed.
    Insert(String),
    /// An input method composes `text`, not yet committed; `cursor` is its caret, a byte
    /// offset in `text`. The element shows it at the caret, underlined.
    Composition {
        /// The text being composed.
        text: String,
        /// Caret within `text`, in bytes.
        cursor: usize,
    },
    /// The composition ended (committed by an `Insert` just before, or cancelled).
    CompositionEnd,
}

/// Gestionnaire `on_text` : `None` laisse passer le texte aux ancêtres.
#[derive(Clone)]
pub(crate) struct TextHandler(Rc<TextCall>);

/// Ce qu'appelle un gestionnaire de texte.
type TextCall = dyn Fn(&TextEvent) -> Option<Message>;

impl TextHandler {
    /// Le gestionnaire qui rend le message de `handler`, s'il en rend un.
    pub(crate) fn new<M: 'static>(handler: impl Fn(&TextEvent) -> Option<M> + 'static) -> Self {
        Self(Rc::new(move |event| handler(event).map(Message::new)))
    }
}

/// Le décalage en octets de `utf16` unités UTF-16 dans `text`.
fn byte_offset(text: &str, utf16: usize) -> usize {
    let mut units = 0;
    for (offset, c) in text.char_indices() {
        if units >= utf16 {
            return offset;
        }
        units += c.len_utf16();
    }
    text.len()
}

/// L'événement de texte d'un événement de la fenêtre, s'il en est un.
fn text_event(event: &UiEvent) -> Option<TextEvent> {
    match event {
        UiEvent::Text(text) | UiEvent::CompositionCommit(text) => {
            Some(TextEvent::Insert(text.to_string()))
        }
        UiEvent::CompositionUpdate {
            value,
            cursor_utf16,
        } => Some(TextEvent::Composition {
            text: value.to_string(),
            cursor: byte_offset(value, *cursor_utf16),
        }),
        UiEvent::CompositionEnd => Some(TextEvent::CompositionEnd),
        _ => None,
    }
}

/// Ce qui prend le texte : le gestionnaire, le rectangle de l'élément, son curseur.
type Owner = (TextHandler, RectF, RectF);

/// L'élément de `start` ou de ses ancêtres qui a un gestionnaire `on_text`, le plus
/// proche d'abord.
fn text_owner(node: &Element, start: ElementId) -> Option<Option<Owner>> {
    let own = || {
        let listeners = node.listeners.as_ref()?;
        let handler = listeners.text.clone()?;
        Some((
            handler,
            node.rect,
            listeners.text_caret.unwrap_or(RectF {
                left: 0.,
                top: 0.,
                right: 0.,
                bottom: 0.,
            }),
        ))
    };
    if node.id == start {
        return Some(own());
    }
    let inner = node.children.iter().find_map(|c| text_owner(c, start))?;
    Some(inner.or_else(own))
}

impl UiRuntime {
    /// L'élément qui prend le texte tapé, s'il y en a un et qu'aucun champ du runtime n'a
    /// le focus.
    fn text_target(&self) -> Option<Owner> {
        if self.focused_input_geometry().is_some() {
            return None;
        }
        let (root, start) = (self.root.as_ref()?, self.key_focus_start()?);
        text_owner(root, start).flatten()
    }

    /// Donne `event` au gestionnaire `on_text` qui a le clavier, s'il en est un et que
    /// c'est du texte ; rend ce que le backend doit faire, `None` quand rien ne l'a pris.
    pub(in crate::ui::runtime) fn typed_text(&mut self, event: &UiEvent) -> Option<EventResult> {
        if matches!(event, UiEvent::CompositionStart) {
            self.text_target()?;
            self.key_focus.composing = true;
            return Some(EventResult {
                handled: true,
                ..EventResult::default()
            });
        }
        let typed = text_event(event)?;
        let (handler, _, _) = self.text_target()?;
        self.key_focus.composing = matches!(typed, TextEvent::Composition { .. });
        let message = (handler.0)(&typed)?;
        self.send(message);
        self.invalidation.repaint = true;
        Some(EventResult {
            handled: true,
            repaint: true,
            ..EventResult::default()
        })
    }

    /// Where the caret of the element that takes the typed text is, for the window of the
    /// input method: the caret given by
    /// [`Element::text_caret`](crate::ui::element::Element::text_caret) in window
    /// coordinates, and the element. `None` when no such element has the keyboard, or a
    /// text field of the runtime has the focus.
    pub fn text_target_geometry(&self) -> Option<crate::ui::text_input::InputGeometry> {
        let (_, rect, caret) = self.text_target()?;
        let caret = RectF {
            left: rect.left + caret.left,
            top: rect.top + caret.top,
            right: rect.left + caret.right.max(caret.left + 1.),
            bottom: rect.top + caret.bottom.max(caret.top + 1.),
        };
        Some(crate::ui::text_input::InputGeometry {
            composing: self.key_focus.composing,
            read_only: false,
            caret,
            bounds: rect,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_composition_caret_in_utf16_units_becomes_a_byte_offset() {
        assert_eq!(byte_offset("にほんご", 2), "にほ".len());
        assert_eq!(byte_offset("a😃b", 3), "a😃".len());
        assert_eq!(byte_offset("abc", 9), 3);
    }
}
