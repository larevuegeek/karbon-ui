//! La barre de menus à l'écran : la rangée des titres, et le menu ouvert (puis son
//! sous-menu) dans des popovers en mode menu.
use super::{
    model::{Menu, MenuEntry},
    state::{Inner, MenuBarMsg, MenuBarState},
    widgets::{Entry, Separator, Title},
};
use crate::{
    application::Action,
    ui::{
        editor::EditorPalette,
        element::{Element, ElementId, Length},
        popover::{Anchor, Popover, PopoverPlacement, Side, SideAlign},
    },
};
use std::{cell::Cell, rc::Rc, sync::Arc};

/// A menu bar inside the window: « Fichier », « Édition », « Affichage »… A click on a
/// title opens its menu below it; the entries show their check marks, their shortcut on
/// the right and a greyed look when disabled; an entry with a submenu opens it on its
/// right. Keys (forwarded to [`MenuBarState::key`]): F10, Alt with the letter of a menu,
/// Left and Right between menus; in a menu, the arrows, Enter and Escape as in any menu.
///
/// [`build`](Self::build) gives the bar and the overlays to add to the root `layers`
/// of the view.
///
/// ```
/// use karbon_ui::ui::{
///     editor::{Menu, MenuBar, MenuBarMsg, MenuBarState, MenuEntry},
///     prelude::*,
/// };
///
/// #[derive(Clone)]
/// enum Msg {
///     Menus(MenuBarMsg<Msg>),
///     Save,
///     Grid,
/// }
/// struct Editor {
///     menus: MenuBarState,
///     grid: bool,
/// }
/// impl Editor {
///     fn menus(&self) -> Vec<Menu<Msg>> {
///         vec![
///             Menu::new("&Fichier").entry(MenuEntry::new("Enregistrer", Msg::Save).shortcut("Ctrl+S")),
///             Menu::new("&Affichage").entry(MenuEntry::new("Grille", Msg::Grid).checked(self.grid)),
///         ]
///     }
/// }
/// impl Application for Editor {
///     type Message = Msg;
///     fn view(&self, _: &ViewContext) -> Element {
///         let (bar, overlays) = MenuBar::new("menus", &self.menus, Msg::Menus, self.menus()).build();
///         layers("root", column("page", [bar]), overlays)
///     }
///     fn update(&mut self, message: Msg) -> bool {
///         match message {
///             Msg::Menus(message) => {
///                 let reply = self.menus.update(message);
///                 return match reply.command {
///                     Some(command) => self.update(command),
///                     None => reply.rebuild,
///                 };
///             }
///             Msg::Save => {}
///             Msg::Grid => self.grid = !self.grid,
///         }
///         true
///     }
///     fn key(&mut self, press: karbon_ui::ui::runtime::KeyPress) -> bool {
///         let menus = self.menus();
///         self.menus.key(press, &menus)
///     }
/// }
/// ```
pub struct MenuBar<M> {
    id: ElementId,
    open: Option<usize>,
    submenu: Option<usize>,
    map: Rc<dyn Fn(MenuBarMsg<M>) -> M>,
    menus: Vec<Menu<M>>,
    palette: EditorPalette,
}

impl<M: Clone + 'static> MenuBar<M> {
    /// The bar showing `menus` in the state `state`; `map` wraps its gestures in the
    /// application's message.
    pub fn new(
        id: impl Into<ElementId>,
        state: &MenuBarState,
        map: impl Fn(MenuBarMsg<M>) -> M + 'static,
        menus: Vec<Menu<M>>,
    ) -> Self {
        Self {
            id: id.into(),
            open: state.open.filter(|&index| index < menus.len()),
            submenu: state.submenu,
            map: Rc::new(map),
            menus,
            palette: EditorPalette::DARK,
        }
    }

    /// Colours of the bar and its menus ([`EditorPalette::DARK`] by default).
    pub fn palette(mut self, palette: EditorPalette) -> Self {
        self.palette = palette;
        self
    }

    /// The bar, one row high and as wide as its parent, and the open menus, to add to
    /// the overlays of the root `layers` of the view.
    pub fn build(self) -> (Element, Vec<Element>) {
        let Self {
            id,
            open,
            submenu,
            map,
            mut menus,
            palette,
        } = self;
        let titles = menus
            .iter()
            .enumerate()
            .map(|(index, menu)| {
                let hover = Rc::clone(&map);
                let title = Title {
                    text: Arc::clone(&menu.title),
                    open: open == Some(index),
                    action: Action::new(map(MenuBarMsg(Inner::Title(index)))),
                    palette,
                };
                Element::custom(title_id(id, index).0, title)
                    .width(Length::Auto)
                    .on_hover(move |inside| {
                        let message = if inside {
                            Inner::HoverTitle(index)
                        } else {
                            Inner::Ignore
                        };
                        hover(MenuBarMsg(message))
                    })
            })
            .collect();
        let bar = Element::row(id.0, titles)
            .height(Length::Px(palette.row_height + 4.))
            .padding(2.)
            .surface(palette.panel, 0.);
        let mut overlays = Vec::new();
        if let Some(index) = open {
            let entries = std::mem::take(&mut menus[index].entries);
            let title = Arc::clone(&menus[index].title);
            let anchor = title_id(id, index);
            let (menu, sub) = menu_popover(id, anchor, title, entries, submenu, &map, palette);
            overlays.push(menu);
            overlays.extend(sub);
        }
        (bar, overlays)
    }
}

fn title_id(bar: ElementId, index: usize) -> ElementId {
    bar.child("title").item(index as u64)
}

/// Le popover du menu ouvert, et celui de son sous-menu ouvert.
fn menu_popover<M: Clone + 'static>(
    bar: ElementId,
    anchor: ElementId,
    title: Arc<str>,
    entries: Vec<MenuEntry<M>>,
    submenu: Option<usize>,
    map: &Rc<dyn Fn(MenuBarMsg<M>) -> M>,
    palette: EditorPalette,
) -> (Element, Option<Element>) {
    let mut children = Vec::new();
    let mut sub = None;
    for (index, mut entry) in entries.into_iter().enumerate() {
        let entry_id = bar.child("entry").item(index as u64);
        if submenu == Some(index) && !entry.children.is_empty() {
            let items = std::mem::take(&mut entry.children);
            let content = items
                .into_iter()
                .enumerate()
                .map(|(k, item)| {
                    entry_element(
                        bar.child("subentry").item(k as u64),
                        item,
                        Place::Submenu,
                        map,
                        palette,
                    )
                })
                .collect();
            let placement = PopoverPlacement {
                side: Side::Right,
                align: SideAlign::Start,
                offset: 2.,
                ..PopoverPlacement::default()
            };
            let popover = Popover::new(
                Arc::clone(&entry.label),
                Anchor::Element(entry_id),
                map(MenuBarMsg(Inner::CloseSubmenu)),
            )
            .menu()
            .placement(placement);
            sub = Some(dress(
                Element::popover(
                    bar.child("submenu").0,
                    popover,
                    Element::column(bar.child("subentries").0, content),
                ),
                palette,
            ));
        }
        let place = Place::Menu {
            index,
            opens: sub.is_some() && submenu == Some(index),
        };
        children.push(entry_element(entry_id, entry, place, map, palette));
    }
    let placement = PopoverPlacement {
        offset: 0.,
        ..PopoverPlacement::default()
    };
    let popover = Popover::new(
        title,
        Anchor::Element(anchor),
        map(MenuBarMsg(Inner::Close)),
    )
    .menu()
    .placement(placement);
    let menu = Element::popover(
        bar.child("menu").0,
        popover,
        Element::column(bar.child("entries").0, children),
    );
    (dress(menu, palette), sub)
}

/// Apparence serrée d'un menu : largeur du contenu, petit rayon, fond de la palette.
fn dress(menu: Element, palette: EditorPalette) -> Element {
    menu.width(Length::Auto)
        .min_width(180.)
        .padding(3.)
        .surface(palette.field, 4.)
}

/// Où se trouve une entrée.
#[derive(Clone, Copy)]
enum Place {
    /// Au rang `index` du menu ouvert ; `opens` quand son sous-menu, ouvert, lui a déjà
    /// pris ses entrées.
    Menu { index: usize, opens: bool },
    /// Dans un sous-menu, où le pointeur n'ouvre rien.
    Submenu,
}

/// Une entrée de menu ou de sous-menu.
fn entry_element<M: Clone + 'static>(
    id: ElementId,
    entry: MenuEntry<M>,
    place: Place,
    map: &Rc<dyn Fn(MenuBarMsg<M>) -> M>,
    palette: EditorPalette,
) -> Element {
    if entry.separator {
        return Element::custom(id.0, Separator { palette });
    }
    let (index, open) = match place {
        Place::Menu { index, opens } => (Some(index), opens),
        Place::Submenu => (None, false),
    };
    let opens = open || !entry.children.is_empty();
    let action = match (opens, index, entry.message) {
        _ if !entry.enabled => None,
        (true, Some(index), _) => Some(Action::new(map(MenuBarMsg(Inner::OpenSubmenu(index))))),
        (false, _, Some(message)) => Some(Action::new(map(MenuBarMsg(Inner::Choose(Box::new(
            message,
        )))))),
        _ => None,
    };
    let widget = Entry {
        label: entry.label,
        shortcut: entry.shortcut,
        check: entry.check,
        submenu: opens,
        action,
        shortcut_width: Cell::new(0.),
        palette,
    };
    let element = Element::custom(id.0, widget).height(Length::Px(palette.row_height));
    match index {
        Some(index) => {
            let hover = Rc::clone(map);
            element.on_hover(move |inside| {
                let message = if inside {
                    Inner::HoverEntry(index, opens)
                } else {
                    Inner::Ignore
                };
                hover(MenuBarMsg(message))
            })
        }
        None => element,
    }
}
