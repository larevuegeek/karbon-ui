//! Premier plan du hit-test : comme dans un navigateur, seul l'élément peint au-dessus
//! du pointeur (et ses ancêtres) est survolé ou cliqué. Un calque absolu de `z-index`
//! supérieur, un panneau `fixed` ou un menu ouvert ne laisse ni `:hover`, ni
//! `group-hover`, ni curseur, ni clic atteindre le contenu qu'il recouvre.
use crate::support::*;
use karbon_ui::ui::{
    prelude::*,
    runtime::{Cursor, UiEvent, UiRuntime},
    scene::PointF,
};
use std::{cell::RefCell, rc::Rc, time::Instant};
css_file!(mod css, "occlusion.css");

/// Une ligne cliquable recouverte en partie par un calque absolu et un panneau `fixed`,
/// et un menu ouvert par-dessus.
struct Page {
    menu: bool,
    clicks: Rc<RefCell<Vec<u8>>>,
}
impl Application for Page {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        let at = Anchor::Point(PointF { x: 50., y: 10. });
        ui! {@styles(css);
            <div class="page">
                <button id="row" class="row group" onclick={Msg::Click(1)}>
                    <div id="reveal" class="reveal"></div>
                </button>
                <div class="cover">Calque</div>
                <div class="pinned">Fixe</div>
                {#if self.menu}
                    <k-popover anchor={at} menu placement="bottom-start" offset="0"
                               aria-label="Menu" onclose={Msg::Click(9)} class="menu">
                        <button id="item" class="item" role="menuitem" onclick={Msg::Click(2)}>Lire</button>
                    </k-popover>
                {/if}
            </div>
        }
    }
    fn update(&mut self, message: Msg) -> bool {
        if let Msg::Click(n) = message {
            self.clicks.borrow_mut().push(n);
        }
        false
    }
}

fn page(menu: bool) -> (UiRuntime, Rc<RefCell<Vec<u8>>>) {
    let clicks = Rc::new(RefCell::new(Vec::new()));
    let app = Page {
        menu,
        clicks: Rc::clone(&clicks),
    };
    let mut ui = UiRuntime::new(app, &Services).unwrap();
    prepare_headless(&mut ui, Instant::now());
    (ui, clicks)
}
fn background(ui: &UiRuntime, id: &str) -> Option<Color> {
    ui.root()?.find(ElementId::named(id))?.style.background
}
fn point(ui: &mut UiRuntime, x: f32, y: f32) -> PointF {
    let p = PointF { x, y };
    ui.event(UiEvent::PointerMove(p));
    prepare_headless(ui, Instant::now());
    p
}
fn click(ui: &mut UiRuntime, p: PointF) {
    ui.event(UiEvent::PointerDown(p));
    ui.event(UiEvent::PointerUp(p));
    prepare_headless(ui, Instant::now());
}
fn row_hovered(ui: &UiRuntime) -> bool {
    background(ui, "row") == Some(Color::hex(0xff0000))
}
fn revealed(ui: &UiRuntime) -> bool {
    background(ui, "reveal") == Some(Color::hex(0xffffff))
}

#[test]
fn an_uncovered_row_is_hovered_and_clicked() {
    let (mut ui, clicks) = page(false);
    let p = point(&mut ui, 100., 20.);
    assert!(row_hovered(&ui));
    assert!(revealed(&ui));
    assert_eq!(ui.cursor(), Cursor::Hand);
    click(&mut ui, p);
    assert_eq!(*clicks.borrow(), [1]);
}

#[test]
fn an_overlay_above_a_row_takes_hover_cursor_and_clicks() {
    let (mut ui, clicks) = page(false);
    // Calque absolu (`z-index: 10`) puis panneau `fixed` : la ligne dessous ne réagit pas.
    for x in [250., 450.] {
        let p = point(&mut ui, x, 20.);
        assert!(!row_hovered(&ui), "x = {x}");
        assert!(!revealed(&ui), "x = {x}");
        assert_eq!(ui.cursor(), Cursor::Arrow, "x = {x}");
        click(&mut ui, p);
        assert!(clicks.borrow().is_empty(), "x = {x}: {:?}", clicks.borrow());
    }
}

#[test]
fn an_open_menu_keeps_the_rows_underneath_from_reacting() {
    let (mut ui, clicks) = page(true);
    // Sur l'élément du menu, au-dessus de la ligne : seul l'élément est survolé.
    let p = point(&mut ui, 100., 25.);
    assert_eq!(background(&ui, "item"), Some(Color::hex(0x333333)));
    assert!(!row_hovered(&ui));
    assert!(!revealed(&ui));
    // Hors du menu : le contenu reste inerte tant que le menu est ouvert.
    point(&mut ui, 20., 20.);
    assert!(!row_hovered(&ui));
    assert_eq!(background(&ui, "item"), Some(Color::hex(0x222222)));
    point(&mut ui, 100., 25.);
    click(&mut ui, p);
    assert!(clicks.borrow().contains(&2), "{:?}", clicks.borrow());
    assert!(!clicks.borrow().contains(&1), "{:?}", clicks.borrow());
}

#[test]
fn a_press_in_the_padding_of_a_menu_item_activates_it() {
    let (mut ui, clicks) = page(true);
    // Menu en (50, 10), bordure intérieure de 4 px : haut de l'élément vers y = 14..22.
    let item = ui
        .root()
        .unwrap()
        .find(ElementId::named("item"))
        .unwrap()
        .bounds();
    let p = point(&mut ui, item.left + 30., item.top + 2.);
    assert_eq!(ui.cursor(), Cursor::Hand);
    click(&mut ui, p);
    assert!(clicks.borrow().contains(&2), "{:?}", clicks.borrow());
}
