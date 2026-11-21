//! Menu contextuel (clic droit) aux entrées serrées de la barre de menus : cases, choix
//! exclusifs, raccourcis, entrées grisées (KUI-034).
use super::{
    model::MenuEntry,
    widgets::{Entry, Separator},
};
use crate::{
    application::Action,
    ui::{
        editor::EditorPalette,
        element::{Element, ElementId, Length},
        popover::{Anchor, Popover},
        scene::PointF,
    },
};
use std::cell::Cell;

/// A context menu (right click) at `at`, made of menu entries with their check marks,
/// dots, shortcuts and greyed-out entries. Choosing an entry sends its message, then
/// `on_close`; Escape or a click elsewhere sends `on_close`. A submenu does not open from
/// a context menu: its entry is shown greyed. Add the element to the overlays of the
/// root `layers` while the menu is open.
///
/// ```
/// use karbon_ui::ui::{
///     editor::{EditorPalette, MenuEntry, context_menu},
///     scene::PointF,
/// };
///
/// #[derive(Clone)]
/// enum Msg {
///     Rename,
///     Delete,
///     CloseMenu,
/// }
/// let menu = context_menu(
///     "object-menu",
///     PointF { x: 120., y: 80. },
///     vec![
///         MenuEntry::new("Renommer", Msg::Rename).shortcut("F2"),
///         MenuEntry::separator(),
///         MenuEntry::new("Supprimer", Msg::Delete).shortcut("Suppr"),
///     ],
///     Msg::CloseMenu,
///     EditorPalette::DARK,
/// );
/// # let _ = menu;
/// ```
pub fn context_menu<M: Clone + 'static>(
    id: impl Into<ElementId>,
    at: PointF,
    entries: Vec<MenuEntry<M>>,
    on_close: M,
    palette: EditorPalette,
) -> Element {
    let id = id.into();
    let children = entries
        .into_iter()
        .enumerate()
        .map(|(index, entry)| {
            let entry_id = id.child("entry").item(index as u64).0;
            if entry.separator {
                return Element::custom(entry_id, Separator { palette });
            }
            let action = entry
                .message
                .filter(|_| entry.enabled && entry.children.is_empty())
                .map(Action::new);
            let widget = Entry {
                label: entry.label,
                shortcut: entry.shortcut,
                check: entry.check,
                submenu: !entry.children.is_empty(),
                action,
                shortcut_width: Cell::new(0.),
                palette,
            };
            Element::custom(entry_id, widget).height(Length::Px(palette.row_height))
        })
        .collect();
    let popover = Popover::new("Menu", Anchor::Point(at), on_close).menu();
    Element::popover(
        id.0,
        popover,
        Element::column(id.child("entries").0, children),
    )
    .width(Length::Auto)
    .min_width(180.)
    .padding(3.)
    .surface(palette.field, 4.)
}
