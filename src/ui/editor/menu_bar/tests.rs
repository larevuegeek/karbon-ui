//! Barre de menus : ouvrir au clic, à F10 et à Alt+lettre, passer d'un menu à l'autre,
//! choisir (le menu se ferme), entrées grisées, cases, sous-menus.
use super::*;
use crate::ui::{
    editor::testing::{NONE, key, window},
    element::{Element, ElementId},
    prelude::{Application, ViewContext, column, layers},
    runtime::{Key, KeyCode, KeyPress},
};

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Menus(MenuBarMsg<Msg>),
    New,
    Undo,
    Grid,
    Recent(u32),
}

#[derive(Default)]
struct Editor {
    menus: MenuBarState,
    grid: bool,
    chosen: Vec<Msg>,
}

impl Editor {
    fn menus(&self) -> Vec<Menu<Msg>> {
        vec![
            Menu::new("&Fichier")
                .entry(MenuEntry::new("Nouveau", Msg::New).shortcut("Ctrl+N"))
                .entry(MenuEntry::submenu(
                    "Récents",
                    vec![
                        MenuEntry::new("piste.niveau", Msg::Recent(1)),
                        MenuEntry::new("col.niveau", Msg::Recent(2)),
                    ],
                ))
                .separator(),
            Menu::new("&Édition").entry(
                MenuEntry::new("Annuler", Msg::Undo)
                    .shortcut("Ctrl+Z")
                    .enabled(false),
            ),
            Menu::new("&Affichage").entry(MenuEntry::new("Grille", Msg::Grid).checked(self.grid)),
        ]
    }
}

impl Application for Editor {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        let (bar, overlays) = MenuBar::new(BAR, &self.menus, Msg::Menus, self.menus()).build();
        layers("root", column("page", [bar]), overlays)
    }
    fn update(&mut self, message: Msg) -> bool {
        match message {
            Msg::Menus(message) => {
                let reply = self.menus.update(message);
                return match reply.command {
                    Some(command) => self.update(command),
                    None => reply.rebuild,
                };
            }
            Msg::Grid => self.grid = !self.grid,
            _ => {}
        }
        self.chosen.push(message);
        true
    }
    fn key(&mut self, press: KeyPress) -> bool {
        let menus = self.menus();
        self.menus.key(press, &menus)
    }
}

const BAR: &str = "menus";

fn bar() -> ElementId {
    ElementId::named(BAR)
}

fn title(index: u64) -> ElementId {
    bar().child("title").item(index)
}

fn entry(index: u64) -> ElementId {
    bar().child("entry").item(index)
}

#[test]
fn a_click_on_a_title_opens_its_menu_and_a_choice_closes_it() {
    let mut w = window(Editor::default(), 600., 400.);
    w.click(title(2)).unwrap();
    assert_eq!(w.app().menus.open_menu(), Some(2));
    // Le menu s'ouvre sous son titre.
    let (t, menu) = (
        w.bounds(title(2)).unwrap(),
        w.bounds(bar().child("menu")).unwrap(),
    );
    assert!(
        (menu.top - t.bottom).abs() < 1. && (menu.left - t.left).abs() < 1.,
        "{t:?} {menu:?}"
    );
    w.click(entry(0)).unwrap();
    assert!(w.app().grid);
    assert_eq!(w.app().menus.open_menu(), None);
    assert!(w.find(bar().child("menu")).is_none());
    // Un second clic sur le titre ouvert le referme.
    w.click(title(0)).unwrap();
    w.click(title(0)).unwrap();
    assert_eq!(w.app().menus.open_menu(), None);
}

#[test]
fn a_disabled_entry_cannot_be_chosen() {
    let mut w = window(Editor::default(), 600., 400.);
    w.click(title(1)).unwrap();
    w.click(entry(0)).unwrap();
    assert!(!w.app().chosen.contains(&Msg::Undo));
    let item = w.accessible(entry(0)).unwrap();
    assert!(!item.enabled);
}

#[test]
fn f10_alt_and_a_letter_and_the_arrows_open_the_menus() {
    let mut w = window(Editor::default(), 600., 400.);
    key(&mut w, KeyCode::F10, NONE);
    assert_eq!(w.app().menus.open_menu(), Some(0));
    key(&mut w, KeyCode::Right, NONE);
    assert_eq!(w.app().menus.open_menu(), Some(1));
    key(&mut w, KeyCode::Left, NONE);
    key(&mut w, KeyCode::Left, NONE);
    assert_eq!(w.app().menus.open_menu(), Some(2));
    // Échap ferme le menu.
    w.press(Key::Escape).unwrap();
    assert_eq!(w.app().menus.open_menu(), None);
    // Alt+A : « &Affichage ».
    w.prepare().unwrap();
    w.runtime_mut().key_input(KeyPress {
        alt: true,
        ..KeyPress::new(KeyCode::A, true)
    });
    w.prepare().unwrap();
    assert_eq!(w.app().menus.open_menu(), Some(2));
    // Dans le menu, Entrée choisit l'entrée qui a le clavier.
    w.press(Key::Down).unwrap();
    w.press(Key::Enter).unwrap();
    assert!(w.app().grid);
}

#[test]
fn a_submenu_opens_on_the_right_of_its_entry_and_a_choice_in_it_closes_everything() {
    let mut w = window(Editor::default(), 600., 400.);
    w.click(title(0)).unwrap();
    // Le pointeur sur une entrée qui ouvre un sous-menu l'ouvre ; Échap ne ferme que lui.
    w.hover(entry(1)).unwrap();
    assert!(w.find(bar().child("submenu")).is_some());
    w.press(Key::Escape).unwrap();
    assert!(w.find(bar().child("submenu")).is_none());
    assert_eq!(w.app().menus.open_menu(), Some(0));
    // Choisie au clavier, elle l'ouvre aussi, et le menu reste ouvert.
    w.focus(entry(1)).unwrap();
    w.press(Key::Enter).unwrap();
    assert_eq!(w.app().menus.open_menu(), Some(0));
    let (opener, sub) = (
        w.bounds(entry(1)).unwrap(),
        w.bounds(bar().child("submenu")).unwrap(),
    );
    assert!(sub.left >= opener.right - 0.5, "{opener:?} {sub:?}");
    w.click(bar().child("subentry").item(1)).unwrap();
    assert_eq!(w.app().chosen.last(), Some(&Msg::Recent(2)));
    assert_eq!(w.app().menus.open_menu(), None);
}

#[test]
fn titles_and_mnemonics_are_read_from_the_ampersand() {
    let menu: Menu<()> = Menu::new("&Fichier");
    assert_eq!(&*menu.title, "Fichier");
    assert_eq!(menu.mnemonic(), Some('F'));
    let menu: Menu<()> = Menu::new("Aide");
    assert_eq!(menu.mnemonic(), None);
}

#[test]
fn a_click_on_another_title_opens_that_menu_at_once() {
    let mut w = window(Editor::default(), 600., 400.);
    w.click(title(0)).unwrap();
    w.click(title(2)).unwrap();
    assert_eq!(w.app().menus.open_menu(), Some(2));
    assert!(w.find(bar().child("menu")).is_some());
}

#[derive(Clone, Debug, PartialEq)]
enum Pick {
    Rename,
    Duplicate,
    Close,
}

/// Un menu contextuel ouvert, qui note les messages reçus.
struct Contextual {
    open: bool,
    log: Vec<Pick>,
}

impl Application for Contextual {
    type Message = Pick;
    fn view(&self, _: &ViewContext) -> Element {
        let mut overlays = Vec::new();
        if self.open {
            let entries = vec![
                MenuEntry::new("Renommer", Pick::Rename).shortcut("F2"),
                MenuEntry::separator(),
                MenuEntry::new("Dupliquer", Pick::Duplicate).enabled(false),
            ];
            let at = crate::ui::scene::PointF { x: 40., y: 40. };
            overlays.push(context_menu(
                "ctx",
                at,
                entries,
                Pick::Close,
                Default::default(),
            ));
        }
        layers("root", column("page", []), overlays)
    }
    fn update(&mut self, message: Pick) -> bool {
        if message == Pick::Close {
            self.open = false;
        }
        self.log.push(message);
        true
    }
}

#[test]
fn a_context_menu_sends_the_chosen_entry_then_closes() {
    let ctx = || ElementId::named("ctx");
    let mut w = window(
        Contextual {
            open: true,
            log: Vec::new(),
        },
        400.,
        300.,
    );
    // Au point demandé.
    let bounds = w.bounds(ctx()).unwrap();
    assert!(
        (bounds.left - 40.).abs() < 1. && (bounds.top - 40.).abs() < 6.,
        "{bounds:?}"
    );
    w.click(ctx().child("entry").item(2)).unwrap();
    assert!(w.app().log.is_empty(), "a disabled entry sends nothing");
    w.click(ctx().child("entry").item(0)).unwrap();
    assert_eq!(w.app().log, [Pick::Rename, Pick::Close]);
    assert!(w.find(ctx()).is_none());
}
