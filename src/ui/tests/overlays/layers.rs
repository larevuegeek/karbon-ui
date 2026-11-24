//! La pile des calques (`Element::layers`) : la page d'abord, puis les dialogues et les
//! popovers peints par-dessus, dans l'ordre où le pointeur les atteint. Une page
//! positionnée (`position: relative`, pour y poser des éléments `absolute`) ne passe pas
//! devant eux : la règle CSS « les enfants positionnés après ceux du flux » vaut entre
//! frères d'une boîte, pas entre les calques d'une pile.
use crate::ui::tests::support::*;
use crate::ui::{modal::Modal, scene::DrawCommand, theme::Color};
use std::cell::RefCell;

/// Couleur de fond de la page : on retrouve sa peinture dans la scène.
const PAGE: Color = Color::rgba(10, 20, 30, 255);
/// Couleur du dialogue.
const SHEET: Color = Color::rgba(200, 40, 40, 255);

struct App {
    positioned: bool,
    open: Rc<RefCell<bool>>,
}
impl Application for App {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        let mut page = Element::column(
            1,
            vec![widgets::button(2, "Sous le dialogue", Msg::Click(2))],
        )
        .surface(PAGE, 0.);
        if self.positioned {
            page.layout_rules().position = Position::Relative;
        }
        let mut overlays = Vec::new();
        if *self.open.borrow() {
            let sheet = Element::column(11, vec![widgets::button(12, "Fermer", Msg::Click(1))])
                .surface(SHEET, 0.);
            overlays.push(
                Element::modal(10, Modal::new("Accueil").on_dismiss(Msg::Click(1)), sheet)
                    .width(Length::Px(300.)),
            );
        }
        Element::layers(20, page, overlays)
    }
    fn update(&mut self, message: Msg) -> bool {
        if message == Msg::Click(1) {
            *self.open.borrow_mut() = false;
        }
        true
    }
}

/// Le rang dans la scène de la première peinture pleine de la couleur `color`.
fn painted_at(scene: &Scene, color: Color) -> Option<usize> {
    scene
        .commands
        .iter()
        .position(|c| matches!(c, DrawCommand::Fill { color: c, .. } if *c == color))
}

fn runtime(positioned: bool) -> (UiRuntime, Rc<RefCell<bool>>) {
    let open = Rc::new(RefCell::new(true));
    let app = App {
        positioned,
        open: Rc::clone(&open),
    };
    let mut ui = UiRuntime::new(app, &NoResources).unwrap();
    prepare(&mut ui, &mut Measure::default());
    (ui, open)
}

#[test]
fn a_positioned_page_stays_under_the_dialogs_of_the_stack() {
    for positioned in [false, true] {
        let (ui, _) = runtime(positioned);
        let page = painted_at(ui.scene(), PAGE).expect("la page est peinte");
        let sheet = painted_at(ui.scene(), SHEET).expect("le dialogue est peint");
        assert!(
            sheet > page,
            "page positionnée : {positioned} ; le dialogue ({sheet}) doit être peint après la page \
             ({page})"
        );
    }
}

#[test]
fn the_dialog_over_a_positioned_page_takes_the_click_where_it_is_drawn() {
    let (mut ui, open) = runtime(true);
    let close = center(&ui, 12);
    ui.event(UiEvent::PointerMove(close));
    ui.event(UiEvent::PointerDown(close));
    ui.event(UiEvent::PointerUp(close));
    assert!(!*open.borrow(), "le bouton du dialogue, visé par sa place");
    prepare(&mut ui, &mut Measure::default());
    assert!(
        painted_at(ui.scene(), SHEET).is_none(),
        "le dialogue fermé n'est plus peint"
    );
}
