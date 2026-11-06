//! Démo Bootstrap pilotée comme un utilisateur, dans une fenêtre de 1280 × 900 plus
//! courte que la page : navigation (défilement, accordéon, cases), transitions et rendu
//! au pixel près.
mod incremental;
mod navigation;
mod rendering;
mod transitions;

use super::bs;
use crate::support::Msg;
use karbon_ui::{
    platform::headless::{HeadlessApp, HeadlessOptions},
    ui::{bootstrap, element::Element, prelude::*, scene::PointF},
};

/// Même état et même `update` que `examples/bootstrap_demo.rs`.
struct Demo {
    checked: bool,
    switched: bool,
    range: f64,
}

impl Application for Demo {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        use karbon_ui::window::WindowAction;
        let cx = TemplateContext::default();
        let (theme, chrome) = ("light", true);
        let (checked, switched, range) = (self.checked, self.switched, self.range);
        ui_file!(
            "../../../../examples/bootstrap_demo/page.ui",
            context = cx,
            styles = bs
        )
    }
    fn update(&mut self, message: Msg) -> bool {
        match message {
            Msg::Check(on) => self.checked = on,
            Msg::Switch(on) => self.switched = on,
            Msg::Range(value) => self.range = value,
            _ => return false,
        }
        true
    }
}

fn demo() -> HeadlessApp<Demo> {
    bootstrap::reset();
    HeadlessApp::with_options(
        Demo {
            checked: true,
            switched: false,
            range: 50.,
        },
        HeadlessOptions {
            width: 1280.,
            height: 900.,
            ..Default::default()
        },
    )
    .unwrap()
}

/// Éléments portant `class`, dans l'ordre du document.
fn with_class<'a>(node: &'a Element, class: &str) -> Vec<&'a Element> {
    let mut out = Vec::new();
    if node.classes().contains(&class) {
        out.push(node);
    }
    for child in &node.children {
        out.extend(with_class(child, class));
    }
    out
}

fn center(e: &Element) -> PointF {
    let r = e.bounds();
    PointF {
        x: (r.left + r.right) / 2.,
        y: (r.top + r.bottom) / 2.,
    }
}

/// Fait défiler la page jusqu'à ce que le centre de l'élément soit visible, et le rend.
fn scroll_to(app: &mut HeadlessApp<Demo>, class: &str, index: usize) -> PointF {
    for _ in 0..40 {
        let p = center(with_class(app.root().unwrap(), class)[index]);
        if (80. ..860.).contains(&p.y) {
            return p;
        }
        app.scroll("page", 0., 120.).unwrap();
    }
    panic!("`.{class}` never scrolled into view");
}

/// Hauteurs des volets de l'accordéon.
fn panes(app: &HeadlessApp<Demo>) -> Vec<f32> {
    with_class(app.root().unwrap(), "accordion-collapse")
        .iter()
        .map(|e| e.bounds().bottom - e.bounds().top)
        .collect()
}

/// Ouvre le second volet (le premier se referme) et laisse l'accordéon se stabiliser
/// avec les mouvements réduits : hauteurs naturelles de référence.
fn settled_panes() -> Vec<f32> {
    bootstrap::reset();
    let mut app = HeadlessApp::with_options(
        Demo {
            checked: true,
            switched: false,
            range: 50.,
        },
        HeadlessOptions {
            width: 1280.,
            height: 900.,
            reduced_motion: true,
            ..Default::default()
        },
    )
    .unwrap();
    let second = scroll_to(&mut app, "accordion-button", 1);
    app.click_at(second).unwrap();
    let heights = panes(&app);
    assert!(heights[0] == 0. && heights[1] > 0., "{heights:?}");
    heights
}

/// Démo dans un affichage à 150 %.
fn demo_at_150() -> HeadlessApp<Demo> {
    bootstrap::reset();
    HeadlessApp::with_options(
        Demo {
            checked: true,
            switched: false,
            range: 50.,
        },
        HeadlessOptions {
            width: 1280.,
            height: 900.,
            scale: 1.5,
            ..Default::default()
        },
    )
    .unwrap()
}
