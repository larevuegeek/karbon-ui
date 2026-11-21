//! Petits éléments de texte partagés par les composants d'éditeur : un libellé serré et
//! une bulle d'aide avec titre et raccourci.
use super::EditorPalette;
use crate::ui::{
    components::Text,
    element::{Element, ElementId},
    tooltip::Tooltip,
};
use std::sync::Arc;

/// A single line of text in the size and colour of `palette`, the label of a tree row or
/// of a property.
///
/// ```
/// use karbon_ui::ui::editor::{EditorPalette, label};
///
/// let name = label("name", "Caméra principale", &EditorPalette::DARK);
/// # let _ = name;
/// ```
pub fn label(
    id: impl Into<ElementId>,
    text: impl Into<Arc<str>>,
    palette: &EditorPalette,
) -> Element {
    Element::custom(
        id.into().0,
        Text::new(text).size(palette.font_size).color(palette.text),
    )
}

/// A tooltip that names a command, says what it does and gives its shortcut, in the
/// look of `palette`: « Enregistrer (Ctrl+S) », then the description on its own line.
/// `shortcut` may be empty.
///
/// ```
/// use karbon_ui::ui::editor::{EditorPalette, tip};
///
/// let help = tip("Enregistrer", "Écrit le niveau sur le disque.", "Ctrl+S", &EditorPalette::DARK);
/// assert_eq!(&*help.text, "Enregistrer (Ctrl+S)\nÉcrit le niveau sur le disque.");
/// ```
pub fn tip(title: &str, description: &str, shortcut: &str, palette: &EditorPalette) -> Tooltip {
    let mut text = String::from(title);
    if !shortcut.is_empty() {
        text.push_str(&format!(" ({shortcut})"));
    }
    if !description.is_empty() {
        text.push('\n');
        text.push_str(description);
    }
    Tooltip::new(text).style(palette.tooltip())
}
