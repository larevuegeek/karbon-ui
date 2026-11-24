//! La zone de saisie d'un champ numérique cliqué.
use super::{FieldId, FieldMsg, Inner, field_element};
use crate::ui::{
    components::{InputStyle, TextInput},
    editor::EditorPalette,
    element::{Element, Length},
};
use std::rc::Rc;

/// La zone de saisie qui remplace un champ cliqué : elle prend le clavier avec tout son
/// texte choisi, valide par Entrée, Tab ou la perte du clavier, abandonne par Échap.
pub(super) fn typing<M: 'static>(
    field: FieldId,
    text: &str,
    palette: EditorPalette,
    map: Rc<dyn Fn(FieldMsg) -> M>,
) -> Element {
    let (on_text, on_key, on_focus) = (Rc::clone(&map), Rc::clone(&map), map);
    let input = TextInput::new(text, move |text| on_text(FieldMsg(Inner::Text(text))))
        .on_key(move |key| on_key(FieldMsg(Inner::Key(key))))
        .autofocus()
        .label("Value")
        .style(InputStyle {
            font_size: palette.font_size,
            padding: 3.,
            radius: 3.,
            background: Some(palette.field),
            foreground: Some(palette.text),
            border: Some(palette.accent),
            focus_border: Some(palette.accent),
        });
    Element::custom(field_element(field).child("input").0, input)
        .height(Length::Px(palette.row_height))
        .on_focus(move |focused| on_focus(FieldMsg(Inner::Focus(focused))))
}
