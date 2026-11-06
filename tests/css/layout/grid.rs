//! Grilles : pistes fixes, fractions et `minmax`, pistes de contenu (`max-content`,
//! `fit-content`), `justify-content`, alignement des cellules ; ratio et `inset`.
use crate::support::*;
use karbon_ui::ui::{layout, prelude::*, scene::RectF};
css_file!(mod css, "grid.css");

#[test]
fn variable_grid_tracks_share_fractions_after_fixed_tracks_and_gaps() {
    let mut root = ui! {@styles(css);<div class="tracks"><div style="height:20px"></div><div style="height:20px"></div><div style="height:20px"></div></div>};
    arrange(&mut root, 500., 100.);
    near(layout::width(root.children[0].bounds()), 80.);
    near(layout::width(root.children[1].bounds()), 400. / 3.);
    near(layout::width(root.children[2].bounds()), 800. / 3.);
    near(root.children[1].bounds().left, 90.);
    arrange(&mut root, 300., 100.);
    near(layout::width(root.children[1].bounds()), 100.);
    near(layout::width(root.children[2].bounds()), 100.);
}

#[test]
fn aspect_ratio_inset_and_grid_self_alignment_shape_boxes() {
    let mut root = ui! {@styles(css);<div>
        <div class="ratio"></div>
        <div style="width: 100px"><div class="ratio-auto"></div></div>
        <div class="positioned"><div class="pinned"></div></div>
        <div class="cellgrid"><div class="self-aligned"></div><div class="tall"></div></div>
        <div class="content-tracks"><p>aaaa bb</p><p>cccc d</p><p>eeeeeeeeee</p><p>x</p></div>
        <div class="centered-tracks"><div class="cell"></div><div class="cell"></div><div class="cell"></div></div>
        <div class="spread-tracks"><div class="cell"></div><div class="cell"></div><div class="cell"></div></div>
    </div>};
    arrange_headless(&mut root, 600., 800.);
    let c = &root.children;
    assert_eq!(layout::height(c[0].bounds()), 90.);
    assert_eq!(layout::height(c[1].children[0].bounds()), 100.);
    let pinned = c[2].children[0].bounds();
    assert_eq!(
        (
            pinned.left - c[2].bounds().left,
            pinned.top - c[2].bounds().top
        ),
        (10., 5.)
    );
    assert_eq!((layout::width(pinned), layout::height(pinned)), (180., 90.));
    // Cellule de 100 × 40 : centré horizontalement, en bas.
    let cell = c[3].children[0].bounds();
    assert_eq!(cell.left - c[3].bounds().left, 40.);
    assert_eq!(cell.top - c[3].bounds().top, 30.);
    // max-content (7 car. × 9 px), fit-content(100px) puis fit-content(30px) bornés.
    // Les largeurs de texte mesurées gardent un pixel de marge (63 → 64, 54 → 55)
    // pour ne jamais repasser à la ligne au dessin.
    let tracks = &c[4].children;
    let w = |i: usize| layout::width(tracks[i].bounds());
    assert_eq!(w(0), 64.);
    assert_eq!(w(1), 55.);
    assert_eq!(w(2), 30.);
    assert_eq!(w(3), 600. - 64. - 55. - 30.);
    // justify-content : 90 px de pistes centrés dans 200 px, puis 3 × 40 px répartis.
    let left = |g: usize, i: usize| c[g].children[i].bounds().left - c[g].bounds().left;
    assert_eq!((left(5, 0), left(5, 1), left(5, 2)), (55., 105., 55.));
    assert_eq!((left(6, 0), left(6, 1), left(6, 2)), (0., 80., 160.));
}

/// `align-items` d'une grille aligne ses cellules (les éléments s'étiraient sur la
/// hauteur de la ligne) ; `align-self` d'une cellule reste prioritaire.
#[test]
fn grid_align_items_aligns_the_cells() {
    let mut node = ui! {@styles(css);
        <div class="grid-center">
            <div id="tall" class="tall"></div>
            <div id="short" class="short"></div>
        </div>
    };
    let bounds = RectF::new(0., 0., 300., 200.);
    karbon_ui::ui::styling::resolve(&mut node, Default::default()).unwrap();
    karbon_ui::ui::layout::validate(&node).unwrap();
    karbon_ui::ui::layout::arrange(&mut node, bounds, bounds, &mut Measure).unwrap();
    let short = node.find(ElementId::named("short")).unwrap().bounds();
    near(height(short), 10.);
    near(short.top, 15.);
    near(
        height(node.find(ElementId::named("tall")).unwrap().bounds()),
        40.,
    );
}
