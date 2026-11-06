//! Texte en composition (méthode d'entrée) affiché dans le champ focalisé, commun à
//! `zwp_text_input_v3` (Wayland) et IBus (X11) : les demandes de la méthode d'entrée
//! deviennent la séquence du moteur (`CompositionStart`, `CompositionUpdate`…,
//! `CompositionCommit`, `CompositionEnd`), la même que sous Windows et macOS. Le moteur
//! n'insère pas la composition dans le texte du champ : la retirer (`discard`) ne change
//! pas le texte, la valider (`commit`, `finish`) le remplace.
use crate::ui::runtime::UiEvent;

#[derive(Default)]
pub(in crate::platform::linux) struct Composition {
    /// Texte en composition affiché dans le champ.
    preedit: Option<String>,
}
impl Composition {
    #[cfg(test)]
    pub fn active(&self) -> bool {
        self.preedit.is_some()
    }
    /// Nouveau texte en composition, caret en unités UTF-16 dans ce texte ; vide : la
    /// composition est retirée du champ.
    pub fn update(&mut self, text: String, cursor_utf16: usize) -> Vec<UiEvent> {
        if text.is_empty() {
            return self.discard();
        }
        let mut events = Vec::with_capacity(2);
        if self.preedit.is_none() {
            events.push(UiEvent::CompositionStart);
        }
        events.push(UiEvent::CompositionUpdate {
            value: text.as_str().into(),
            cursor_utf16,
        });
        self.preedit = Some(text);
        events
    }
    /// Texte validé par la méthode d'entrée : remplace la composition en cours, sinon
    /// s'insère comme une frappe.
    pub fn commit(&mut self, text: String) -> Vec<UiEvent> {
        if self.preedit.take().is_some() {
            vec![
                UiEvent::CompositionCommit(text.into()),
                UiEvent::CompositionEnd,
            ]
        } else if text.is_empty() {
            Vec::new()
        } else {
            vec![UiEvent::Text(text.into())]
        }
    }
    /// Composition retirée du champ sans texte validé.
    pub fn discard(&mut self) -> Vec<UiEvent> {
        match self.preedit.take() {
            Some(_) => vec![UiEvent::CompositionEnd],
            None => Vec::new(),
        }
    }
    /// Composition validée telle quelle (clic ailleurs, perte du focus).
    pub fn finish(&mut self) -> Vec<UiEvent> {
        match self.preedit.take() {
            Some(text) => vec![
                UiEvent::CompositionCommit(text.into()),
                UiEvent::CompositionEnd,
            ],
            None => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Composition, UiEvent};

    #[test]
    fn a_composition_starts_once_then_is_committed_or_discarded() {
        let mut c = Composition::default();
        assert!(matches!(
            c.update("n".into(), 1).as_slice(),
            [UiEvent::CompositionStart, UiEvent::CompositionUpdate { .. }]
        ));
        assert!(matches!(
            c.update("ni".into(), 2).as_slice(),
            [UiEvent::CompositionUpdate {
                cursor_utf16: 2,
                ..
            }]
        ));
        assert!(c.active());
        assert!(matches!(
            c.commit("你".into()).as_slice(),
            [UiEvent::CompositionCommit(_), UiEvent::CompositionEnd]
        ));
        assert!(!c.active());
        // Sans composition, un texte validé est une frappe ; un texte vide, rien.
        assert!(matches!(
            c.commit("a".into()).as_slice(),
            [UiEvent::Text(_)]
        ));
        assert!(c.commit(String::new()).is_empty());
        assert!(c.discard().is_empty());
        c.update("x".into(), 1);
        assert!(matches!(
            c.finish().as_slice(),
            [UiEvent::CompositionCommit(_), UiEvent::CompositionEnd]
        ));
        c.update("y".into(), 1);
        assert!(matches!(
            c.update(String::new(), 0).as_slice(),
            [UiEvent::CompositionEnd]
        ));
    }
}
