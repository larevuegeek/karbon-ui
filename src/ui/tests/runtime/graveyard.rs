//! Anciens arbres d'une reconstruction : un grand arbre est libéré par tranches, hors de
//! l'image qui reconstruit la vue ; un petit l'est tout de suite.
use crate::ui::tests::support::*;
use std::time::Duration;

/// Feuille qui compte les instances vivantes.
struct Alive(Rc<Cell<usize>>);
impl Drop for Alive {
    fn drop(&mut self) {
        self.0.set(self.0.get() - 1);
    }
}
impl Widget for Alive {
    fn measure(&self, _: SizeF, _: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
        Ok(SizeF {
            width: 10.,
            height: 10.,
        })
    }
    fn paint(&self, _: &PaintContext, _: &mut Scene) {}
}

fn runtime(count: u64, live: &Rc<Cell<usize>>) -> UiRuntime {
    let live = live.clone();
    view_runtime(move || {
        let cells = (0..count)
            .map(|i| {
                live.set(live.get() + 1);
                Element::custom(10 + i, Alive(live.clone()))
            })
            .collect();
        Element::column(1, cells)
    })
}

#[test]
fn large_replaced_trees_are_released_in_slices_after_the_rebuild() {
    let live = Rc::new(Cell::new(0));
    let mut ui = runtime(5000, &live);
    let mut m = Measure::default();
    prepare(&mut ui, &mut m);
    assert_eq!(live.get(), 5000);
    ui.invalidate();
    prepare(&mut ui, &mut m);
    // L'ancien arbre n'est pas libéré par l'image qui reconstruit.
    assert!(live.get() > 5000, "{}", live.get());
    let wake = ui.next_wake().expect("tranche planifiée");
    for _ in 0..100 {
        if live.get() == 5000 {
            break;
        }
        ui.event(UiEvent::Timer(wake + Duration::from_secs(1)));
    }
    assert_eq!(live.get(), 5000);
    assert!(ui.next_wake().is_none());
    drop(ui);
    assert_eq!(live.get(), 0);
}

#[test]
fn small_replaced_trees_are_released_by_the_rebuild() {
    let live = Rc::new(Cell::new(0));
    let mut ui = runtime(100, &live);
    let mut m = Measure::default();
    prepare(&mut ui, &mut m);
    ui.invalidate();
    prepare(&mut ui, &mut m);
    assert_eq!(live.get(), 100);
    assert!(ui.next_wake().is_none());
}
