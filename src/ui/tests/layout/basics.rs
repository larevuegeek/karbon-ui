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

#[test]
fn trees_are_limited_to_max_elements() {
    // Une longue liste riche (500 lignes d'une cinquantaine d'éléments) passe ; au-delà
    // de la limite, la validation refuse l'arbre (utiliser une liste virtuelle).
    let column = |n: usize| {
        Element::column(
            1,
            (0..n as u64)
                .map(|i| Element::column(10 + i, vec![]))
                .collect(),
        )
    };
    assert!(layout::validate(&column(layout::MAX_ELEMENTS - 1)).is_ok());
    assert!(crate::error::rejected(layout::validate(&column(
        layout::MAX_ELEMENTS
    ))));
}

/// Un volet ou un contenu de défilement masqué est retiré du flux : l'autre volet occupe
/// toute la place, sans séparateur, et la zone de défilement reste vide.
#[test]
fn a_hidden_pane_or_scroll_content_leaves_the_flow() {
    let hidden = |mut node: Element| {
        node.layout_rules().hidden = true;
        node
    };
    let available = SizeF {
        width: 400.,
        height: 200.,
    };
    let fill = |id| box_node(id, Length::Fill(1.), Length::Fill(1.));
    let mut split = Element::split(1, Split::horizontal(0.25), hidden(fill(2)), fill(3));
    layout::measure(&mut split, available, &mut Measure::default()).unwrap();
    arrange(&mut split, 400., 200.);
    assert_eq!(split.children[0].id, ElementId(2));
    near(layout::width(split.children[0].bounds()), 0.);
    near(split.children[1].bounds().left, 0.);
    near(layout::width(split.children[1].bounds()), 400.);
    assert!(split.divider.is_none());

    let mut both = Element::split(1, Split::horizontal(0.5), hidden(fill(2)), hidden(fill(3)));
    layout::measure(&mut both, available, &mut Measure::default()).unwrap();
    arrange(&mut both, 400., 200.);

    for height in [Length::Auto, Length::Fill(1.)] {
        let content = hidden(box_node(2, Length::Fill(1.), Length::Px(900.)));
        let mut scroll = Element::scroll(1, Axis::Vertical, content).height(height);
        layout::measure(&mut scroll, available, &mut Measure::default()).unwrap();
        arrange(&mut scroll, 400., 200.);
        near(scroll.scroll_metrics().unwrap().max_offset(), 0.);
        near(layout::height(scroll.children[0].bounds()), 0.);
    }
}

/// Grille `auto-fill` : le nombre de colonnes suit la largeur, donc une étendue ou une
/// colonne explicite qui n'y tient plus est ramenée dans la grille au lieu d'échouer.
#[test]
fn a_fitted_grid_keeps_wide_spans_and_explicit_columns_inside_its_columns() {
    let cell = |id| box_node(id, Length::Fill(1.), Length::Px(20.));
    let mut root = Element::grid(
        1,
        3,
        vec![cell(2).span(3), cell(3).at(1, 3), cell(4).at(1, 1), cell(5)],
    );
    let rules = root.layout_rules();
    rules.columns = Some(Arc::from([layout::GridTrack {
        min: layout::TrackBreadth::Px(100.),
        max: layout::TrackBreadth::Fr(1.),
    }]));
    rules.grid_auto_repeat = Some(false);
    arrange(&mut root, 320., 400.);
    near(layout::width(root.children[0].bounds()), 320.);
    near(root.children[1].bounds().left, 213.33);
    // Une seule colonne : chaque cellule a sa ligne, la position explicite garde la sienne.
    arrange(&mut root, 150., 400.);
    for child in &root.children {
        near(child.bounds().left, 0.);
        near(layout::width(child.bounds()), 150.);
    }
    near(root.children[2].bounds().top, 0.);
    let mut tops = root
        .children
        .iter()
        .map(|c| c.bounds().top as i32)
        .collect::<Vec<_>>();
    tops.sort_unstable();
    assert_eq!(tops, [0, 20, 40, 60]);
    // Hors grille ajustée, la même étendue reste une erreur, sans dépassement d'entier.
    assert!(crate::error::rejected(layout::validate(&Element::grid(
        1,
        3,
        vec![cell(2).span(2).column_start(usize::MAX)]
    ))));
}
