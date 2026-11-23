//! Grille de vignettes virtualisée : seules les lignes proches de la vue sont faites, la
//! dernière ligne garde la largeur des cases, les colonnes suivent la largeur.
use crate::ui::tests::support::*;
use crate::ui::virtual_grid::VirtualGrid;

fn grid(count: usize, columns: usize, built: Rc<Cell<usize>>) -> Element {
    let grid = VirtualGrid::new(count, columns, 40., move |index| {
        built.set(built.get() + 1);
        Element::column(10_000 + index as u64, Vec::new()).height(Length::Px(40.))
    })
    .gap(4.);
    Element::virtual_grid(1, grid)
}

#[test]
fn twenty_thousand_cells_build_only_the_rows_in_view() {
    let built = Rc::new(Cell::new(0));
    let mut ui = {
        let built = built.clone();
        view_runtime(move || grid(20_000, 5, built.clone()))
    };
    let mut m = Measure::default();
    prepare(&mut ui, &mut m);
    let rows = ui.stats().virtual_rows;
    assert!(rows > 0 && rows < 40, "{rows} lignes faites");
    // Chaque ligne faite a ses cinq cases, et seulement elles.
    assert_eq!(built.get(), rows * 5);
    let root = ui.root().unwrap();
    assert_eq!(root.virtual_metrics().unwrap().item_count, 4_000);
    assert!(root.find(ElementId(10_000 + 19_999)).is_none());
    let first = root.find(ElementId(10_000)).unwrap().rect;
    let second = root.find(ElementId(10_001)).unwrap().rect;
    near(second.left - first.right, 4.);
}

#[test]
fn the_last_row_keeps_the_width_of_the_cells() {
    let built = Rc::new(Cell::new(0));
    let mut ui = {
        let built = built.clone();
        view_runtime(move || grid(7, 3, built.clone()))
    };
    let mut m = Measure::default();
    prepare(&mut ui, &mut m);
    let root = ui.root().unwrap();
    let width = |id: u64| {
        let r = root.find(ElementId(10_000 + id)).unwrap().rect;
        r.right - r.left
    };
    near(width(6), width(0));
    assert_eq!(built.get(), 7);
}

#[test]
fn the_columns_follow_the_width() {
    assert_eq!(VirtualGrid::columns_for(500., 100., 10.), 4);
    assert_eq!(VirtualGrid::columns_for(50., 100., 10.), 1);
    assert_eq!(VirtualGrid::columns_for(f32::NAN, 100., 10.), 1);
    let grid = VirtualGrid::new(10, 4, 30., |i| Element::column(i as u64, Vec::new()));
    assert_eq!((grid.rows(), grid.row_of(9)), (3, 2));
}
