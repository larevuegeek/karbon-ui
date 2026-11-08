//! `onwheel` : gestionnaires de la molette, du plus profond à la racine ; un message
//! rendu consomme l'événement (ni ancêtres ni défilement), `None` le laisse passer.
use crate::ui::tests::support::*;
use std::cell::RefCell;

/// Messages reçus : (élément, valeur).
type Log = Rc<RefCell<Vec<(u64, f32)>>>;

/// Zone défilante 1 de vingt lignes ; la ligne 10 porte le gestionnaire intérieur, la
/// colonne 2 (contenu de la zone, parent des lignes) le gestionnaire extérieur.
struct Wheel {
    log: Log,
    inner: bool,
    outer: bool,
}
impl Application for Wheel {
    type Message = (u64, f32);
    fn view(&self, _: &ViewContext) -> Element {
        let (inner, outer) = (self.inner, self.outer);
        let rows = (0..20u64).map(|n| {
            let row = box_node(10 + n, Length::Fill(1.), Length::Px(50.));
            if n == 0 {
                row.on_wheel(move |e: WheelEvent| inner.then_some((10u64, e.delta_y)))
            } else {
                row
            }
        });
        Element::scroll(
            1,
            Axis::Vertical,
            Element::column(2, rows.collect()).on_wheel(move |e| outer.then_some((2u64, e.y))),
        )
    }
    fn update(&mut self, message: (u64, f32)) -> bool {
        self.log.borrow_mut().push(message);
        false
    }
}
fn runtime(inner: bool, outer: bool) -> (UiRuntime, Log) {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut ui = UiRuntime::new(
        Wheel {
            log: log.clone(),
            inner,
            outer,
        },
        &NoResources,
    )
    .unwrap();
    prepare(&mut ui, &mut Measure::default());
    (ui, log)
}

#[test]
fn onwheel_handlers_run_from_the_target_up_and_a_message_stops_scrolling() {
    let (mut ui, log) = runtime(true, true);
    let first = center(&ui, 10);
    let second = center(&ui, 11);
    // Ligne 10 : son gestionnaire consomme, le parent n'est pas appelé, rien ne défile.
    let result = wheel(&mut ui, first, 0., 120.);
    assert!(result.handled);
    assert_eq!(*log.borrow(), [(10, 120.)]);
    near(scroll_metrics(&ui, 1).offset, 0.);
    // Ligne 11 : l'événement remonte au parent, qui reçoit la position du pointeur.
    assert!(wheel(&mut ui, second, 0., 120.).handled);
    assert_eq!(log.borrow()[1], (2, second.y));
    near(scroll_metrics(&ui, 1).offset, 0.);
}

#[test]
fn onwheel_returning_none_lets_the_event_bubble_and_the_container_scroll() {
    // La ligne ignore : le parent consomme.
    let (mut ui, log) = runtime(false, true);
    let first = center(&ui, 10);
    wheel(&mut ui, first, 0., 120.);
    assert_eq!(log.borrow().len(), 1);
    assert_eq!(log.borrow()[0].0, 2);
    near(scroll_metrics(&ui, 1).offset, 0.);
    // Personne ne consomme : la zone défile comme sans gestionnaire.
    let (mut ui, log) = runtime(false, false);
    assert!(wheel(&mut ui, first, 0., 120.).handled);
    assert!(log.borrow().is_empty());
    near(scroll_metrics(&ui, 1).offset, 120.);
}
