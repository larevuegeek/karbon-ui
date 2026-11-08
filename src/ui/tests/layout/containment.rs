//! `content-visibility: auto` : contenu des lignes éloignées de la vue ni mesuré, ni
//! disposé, ni peint ; disposé quand le défilement l'approche.
use crate::ui::tests::support::*;

/// Contenu d'une ligne qui compte ses mesures et ses dessins.
struct Counted {
    measured: Rc<Cell<usize>>,
    painted: Rc<Cell<usize>>,
}
impl Widget for Counted {
    fn measure(&self, _: SizeF, _: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
        self.measured.set(self.measured.get() + 1);
        Ok(SizeF {
            width: 80.,
            height: 16.,
        })
    }
    fn paint(&self, _: &PaintContext, _: &mut Scene) {
        self.painted.set(self.painted.get() + 1);
    }
}

const ROWS: u64 = 200;
const ROW_HEIGHT: f32 = 32.;

fn list(measured: Rc<Cell<usize>>, painted: Rc<Cell<usize>>) -> UiRuntime {
    view_runtime(move || {
        let rows = (0..ROWS)
            .map(|i| {
                let content = Element::custom(
                    1000 + i,
                    Counted {
                        measured: measured.clone(),
                        painted: painted.clone(),
                    },
                );
                let mut row = Element::row(100 + i, vec![Element::column(2000 + i, vec![content])])
                    .height(Length::Px(ROW_HEIGHT));
                let rules = row.layout_rules();
                rules.overflow_x = Overflow::Visible;
                rules.overflow_y = Overflow::Visible;
                rules.content_visibility = ContentVisibility::Auto;
                rules.contain_intrinsic_size[1] = ContainIntrinsicLength {
                    remembered: true,
                    length: Some(ROW_HEIGHT),
                };
                row
            })
            .collect();
        Element::scroll(1, Axis::Vertical, Element::column(2, rows))
    })
}

fn skipped(ui: &UiRuntime, row: u64) -> bool {
    ui.root()
        .unwrap()
        .find(ElementId(100 + row))
        .unwrap()
        .content_skipped
}

#[test]
fn rows_far_from_the_viewport_are_neither_measured_nor_painted() {
    let measured = Rc::new(Cell::new(0));
    let painted = Rc::new(Cell::new(0));
    let mut ui = list(measured.clone(), painted.clone());
    let mut m = Measure::default();
    prepare(&mut ui, &mut m);
    // Vue de 400 px élargie de moitié de chaque côté : une vingtaine de lignes.
    let laid_out = (0..ROWS).filter(|&i| !skipped(&ui, i)).count();
    assert!((12..=24).contains(&laid_out), "{laid_out}");
    assert!(!skipped(&ui, 0) && skipped(&ui, 150));
    assert!(measured.get() <= 24 * 4, "{}", measured.get());
    assert!(painted.get() <= 14, "{}", painted.get());
    // Hauteur totale inchangée : chaque ligne garde sa boîte.
    let s = ui
        .root()
        .unwrap()
        .find(ElementId(1))
        .unwrap()
        .scroll
        .unwrap();
    assert_eq!(s.extent, ROWS as f32 * ROW_HEIGHT);

    // Défilement de 40 lignes : celles qu'il approche sont disposées, à leur place.
    for _ in 0..20 {
        painted.set(0);
        wheel(&mut ui, PointF { x: 20., y: 70. }, 0., 64.);
        prepare(&mut ui, &mut m);
        assert!((1..=14).contains(&painted.get()), "{}", painted.get());
    }
    assert!(skipped(&ui, 0) && !skipped(&ui, 45));
    let root = ui.root().unwrap();
    let row = root.find(ElementId(145)).unwrap();
    let content = root.find(ElementId(1045)).unwrap();
    assert!(
        content.bounds().top >= row.bounds().top && content.bounds().bottom <= row.bounds().bottom,
        "{:?} dans {:?}",
        content.bounds(),
        row.bounds()
    );
    let laid_out = (0..ROWS).filter(|&i| !skipped(&ui, i)).count();
    assert!((12..=26).contains(&laid_out), "{laid_out}");
}
