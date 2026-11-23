//! Barre d'outils : ce qui tient, débordement dans l'ordre, bulles avec raccourci,
//! choix et bascules par le menu de débordement, clavier.
use super::{view::fitting, *};
use crate::ui::{
    editor::testing::window,
    element::{Element, ElementId},
    prelude::{Application, ViewContext, column, layers},
    runtime::Key,
};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Bar(ToolbarMsg),
    Tool(u8),
    Grid,
}

struct Tools {
    width: f32,
    bar: ToolbarState,
    tool: u8,
    grid: bool,
    log: Vec<Msg>,
}

impl Tools {
    fn new(width: f32) -> Self {
        Self {
            width,
            bar: ToolbarState::default(),
            tool: 0,
            grid: false,
            log: Vec::new(),
        }
    }
}

impl Application for Tools {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        let names = ["Choisir", "Déplacer", "Tourner", "Agrandir"];
        let keys = ["Q", "G", "R", "S"];
        let mut items: Vec<ToolItem<Msg>> = (0..4u8)
            .map(|k| {
                ToolItem::choice(names[k as usize], self.tool == k, Msg::Tool(k))
                    .text(keys[k as usize])
                    .shortcut(keys[k as usize])
                    .description("Un outil de la vue.")
            })
            .collect();
        items.push(ToolItem::separator());
        items.push(
            ToolItem::toggle("Grille", self.grid, Msg::Grid)
                .text("#")
                .shortcut("Ctrl+G"),
        );
        let (bar, overlays) = Toolbar::new("tools", &self.bar, Msg::Bar, items).build(self.width);
        layers("root", column("page", [bar]), overlays)
    }
    fn update(&mut self, message: Msg) -> bool {
        match &message {
            Msg::Bar(m) => {
                self.bar.update(*m);
            }
            Msg::Tool(k) => self.tool = *k,
            Msg::Grid => self.grid = !self.grid,
        }
        self.log.push(message);
        true
    }
}

fn item(index: u64) -> ElementId {
    ElementId::named("tools").child("item").item(index)
}

fn more() -> ElementId {
    ElementId::named("tools").child("more")
}

#[test]
fn the_items_that_do_not_fit_leave_in_order_with_the_overflow_button_at_the_end() {
    // Calcul : six éléments de 28, deux écarts : tout tient à 200, quatre à 130.
    let widths = [28., 28., 28., 28., 9., 28.];
    assert_eq!(fitting(&widths, 200., 28.), 6);
    assert_eq!(fitting(&widths, 130., 28.), 3);
    assert_eq!(fitting(&widths, 10., 28.), 0);
    // Large : tout est là, pas de bouton « » ».
    let w = window(Tools::new(600.), 800., 300.);
    assert!(w.find(item(5)).is_some() && w.find(more()).is_none());
    // En rétrécissant : les derniers partent, dans l'ordre, et le bouton « » » arrive.
    let mut last_count = 6;
    for width in [150., 120., 90.] {
        let w = window(Tools::new(width), 800., 300.);
        let count = (0..6).filter(|&k| w.find(item(k)).is_some()).count();
        assert!(count <= last_count, "{width}: {count}");
        assert!(
            (0..count as u64).all(|k| w.find(item(k)).is_some()),
            "{width}"
        );
        assert!(w.find(more()).is_some(), "{width}");
        let bar = w.bounds(more()).unwrap();
        assert!(bar.right - bar.left <= width + 0.5, "{width}: {bar:?}");
        last_count = count;
    }
}

#[test]
fn the_overflow_menu_holds_the_hidden_items_with_their_marks_and_shortcuts() {
    let mut w = window(Tools::new(120.), 800., 300.);
    w.click(more()).unwrap();
    assert!(w.app().bar.overflow_open());
    // Le menu dit les raccourcis des éléments cachés ; la grille en fait partie.
    assert!(w.contains_text("Ctrl+G"));
    let overflow = ElementId::named("tools").child("overflow");
    let entries: Vec<ElementId> = (0..6)
        .map(|k| overflow.child("entry").item(k))
        .filter(|id| w.find(*id).is_some())
        .collect();
    w.click(*entries.last().unwrap()).unwrap();
    assert!(w.app().grid);
    assert!(!w.app().bar.overflow_open());
}

#[test]
fn each_button_has_its_tooltip_with_its_shortcut_and_works_from_the_keyboard() {
    let mut w = window(Tools::new(600.), 800., 300.);
    w.hover(item(1)).unwrap();
    w.advance(Duration::from_millis(700)).unwrap();
    let tip = w.runtime().tooltip().expect("tooltip shown");
    assert_eq!(&*tip.text, "Déplacer (G)\nUn outil de la vue.");
    w.click(item(2)).unwrap();
    assert_eq!(w.app().tool, 2);
    // Au clavier : le focus sur un bouton, Entrée l'active.
    w.focus(item(3)).unwrap();
    w.press(Key::Enter).unwrap();
    assert_eq!(w.app().tool, 3);
}
