//! Événements des zones défilantes : `onscroll` (chaque déplacement) et `onscrollrange`
//! (plage défilable changée).
use crate::ui::tests::support::*;

#[test]
fn on_scroll_range_reports_the_range_when_it_appears_and_when_it_changes() {
    // `onscrollrange` : la plage d'une rangée qui défile est signalée à son apparition,
    // puis seulement quand l'étendue du contenu change (les flèches d'un carrousel).
    struct RangeApp {
        width: Rc<Cell<f32>>,
        seen: Rc<std::cell::RefCell<Vec<f32>>>,
    }
    impl Application for RangeApp {
        type Message = Msg;
        fn view(&self, _: &ViewContext) -> Element {
            Element::scroll(
                1,
                Axis::Horizontal,
                Element::row(
                    2,
                    vec![box_node(3, Length::Px(self.width.get()), Length::Px(40.))],
                )
                .width(Length::Auto),
            )
            .on_scroll_range(|s| Msg::Commit(1, f64::from(s.max_offset())))
        }
        fn update(&mut self, message: Msg) -> bool {
            if let Msg::Commit(1, max) = message {
                self.seen.borrow_mut().push(max as f32);
            }
            false
        }
    }
    let width = Rc::new(Cell::new(2000.));
    let seen = Rc::new(std::cell::RefCell::new(vec![]));
    let app = RangeApp {
        width: width.clone(),
        seen: seen.clone(),
    };
    let mut ui = UiRuntime::new(app, &NoResources).unwrap();
    let mut measure = Measure::default();
    prepare(&mut ui, &mut measure);
    let viewport = scroll_metrics(&ui, 1).viewport_size();
    assert_eq!(seen.borrow().len(), 1);
    near(seen.borrow()[0], 2000. - viewport);
    // Même plage : rien de plus.
    ui.invalidate();
    prepare(&mut ui, &mut measure);
    assert_eq!(seen.borrow().len(), 1);
    // Contenu qui tient dans la vue : plage nulle, signalée.
    width.set(100.);
    ui.invalidate();
    prepare(&mut ui, &mut measure);
    assert_eq!(seen.borrow().len(), 2);
    near(seen.borrow()[1], 0.);
}

#[test]
fn on_scroll_reports_the_new_geometry_after_each_move() {
    // `onscroll` : chaque déplacement réel émet la géométrie à jour (chargement de la
    // page suivante près de la fin) ; un déplacement nul (butée) n'émet rien.
    struct ScrollApp(Rc<std::cell::RefCell<Vec<(f32, f32)>>>);
    impl Application for ScrollApp {
        type Message = Msg;
        fn view(&self, _: &ViewContext) -> Element {
            Element::scroll(
                1,
                Axis::Vertical,
                Element::column(2, vec![box_node(3, Length::Fill(1.), Length::Px(1000.))]),
            )
            .on_scroll(|s| Msg::Commit(1, f64::from(s.offset) + 1e4 * f64::from(s.max_offset())))
        }
        fn update(&mut self, message: Msg) -> bool {
            if let Msg::Commit(1, v) = message {
                let max = (v / 1e4).floor();
                self.0
                    .borrow_mut()
                    .push(((v - max * 1e4) as f32, max as f32));
            }
            false
        }
    }
    let seen = Rc::new(std::cell::RefCell::new(vec![]));
    let mut ui = UiRuntime::new(ScrollApp(seen.clone()), &NoResources).unwrap();
    let mut measure = Measure::default();
    prepare(&mut ui, &mut measure);
    let max = scroll_metrics(&ui, 1).max_offset();
    let p = PointF { x: 30., y: 70. };
    wheel(&mut ui, p, 0., 120.);
    wheel(&mut ui, p, 0., 30.);
    wheel(&mut ui, p, 0., -1000.);
    // Déjà en haut : rien ne bouge, rien n'est émis.
    wheel(&mut ui, p, 0., -10.);
    let seen = seen.borrow();
    assert_eq!(seen.len(), 3);
    for ((offset, m), expected) in seen.iter().zip([120., 150., 0.]) {
        near(*offset, expected);
        near(*m, max);
    }
}
