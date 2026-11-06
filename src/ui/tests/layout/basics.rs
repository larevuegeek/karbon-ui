//! Mise en page de base : répartition en ligne, contraintes, grille et validation.
use crate::ui::tests::support::*;
#[test]
fn row_distributes_fixed_percent_and_fractional_space_with_gap() {
    let mut root = Element::row(
        1,
        vec![
            box_node(2, Length::Px(100.), Length::Px(20.)),
            box_node(3, Length::Percent(25.), Length::Px(20.)),
            box_node(4, Length::Fill(1.), Length::Px(20.)),
        ],
    )
    .gap(10.);
    arrange(&mut root, 500., 60.);
    near(layout::width(root.children[0].bounds()), 100.);
    near(layout::width(root.children[1].bounds()), 125.);
    near(layout::width(root.children[2].bounds()), 255.);
    near(root.children[2].bounds().right, 500.);
    near(layout::height(root.children[0].bounds()), 20.); // Stretch respecte une taille explicite.
}

#[test]
fn fractional_space_respects_minima_maxima_and_padding() {
    let mut root = Element::row(
        1,
        vec![
            box_node(2, Length::Fill(1.), Length::Auto).max_width(100.),
            box_node(3, Length::Fill(1.), Length::Auto).min_width(50.),
        ],
    )
    .padding(10.)
    .gap(10.);
    arrange(&mut root, 500., 100.);
    near(root.children[0].bounds().left, 10.);
    near(layout::width(root.children[0].bounds()), 100.);
    near(layout::width(root.children[1].bounds()), 370.);
    near(root.children[1].bounds().right, 490.);
}

#[test]
fn grid_spans_wrap_and_explicit_positions_reserve_their_cells() {
    let mut root = Element::grid(
        1,
        12,
        vec![
            box_node(2, Length::Fill(1.), Length::Px(40.)).span(4),
            box_node(3, Length::Fill(1.), Length::Px(50.)).span(4),
            box_node(4, Length::Fill(1.), Length::Px(30.))
                .span(4)
                .at(1, 1),
            box_node(5, Length::Fill(1.), Length::Px(20.)).span(12),
        ],
    )
    .gap(12.);
    arrange(&mut root, 600., 200.);
    near(root.children[2].bounds().left, 0.);
    near(root.children[0].bounds().left, 204.);
    near(root.children[1].bounds().left, 408.);
    near(root.children[3].bounds().top, 62.);
    near(layout::width(root.children[0].bounds()), 192.);
    near(layout::height(root.children[2].bounds()), 30.);
    root.children[0].style.row = Some(1);
    root.children[0].style.column = Some(1);
    assert!(crate::error::rejected(layout::validate(&root)));
}

#[test]
fn invalid_ids_spans_and_constraints_fail_before_layout() {
    assert!(crate::error::rejected(layout::validate(&Element::column(
        1,
        vec![Element::column(1, vec![])]
    ))));
    assert!(crate::error::rejected(layout::validate(&Element::grid(
        1,
        0,
        vec![]
    ))));
    assert!(crate::error::rejected(layout::validate(&Element::grid(
        1,
        3,
        vec![Element::column(2, vec![]).span(4)]
    ))));
    assert!(crate::error::rejected(layout::validate(
        &Element::column(1, vec![]).width(Length::Fill(f32::NAN))
    )));
    assert!(crate::error::rejected(layout::validate(
        &Element::column(1, vec![]).min_width(10.).max_width(5.)
    )));
}
