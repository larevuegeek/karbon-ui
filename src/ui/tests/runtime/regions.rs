//! Parties exclues d'une zone de déplacement de fenêtre (`data-no-drag`).
use crate::ui::tests::support::*;

#[test]
fn no_drag_excludes_a_part_of_a_drag_region_and_a_nested_region_moves_again() {
    use crate::ui::view::{column, row, space};
    let mut ui = view_runtime(|| {
        column(
            1,
            [
                // Pochette entière : déplace la fenêtre…
                column(
                    2,
                    [
                        space(3).height(40.),
                        // … sauf la rangée du bas (barre de progression, volume)…
                        row(
                            4,
                            [
                                space(5).width(100.).height(30.),
                                // … où une poignée déplace de nouveau.
                                space(6).width(100.).height(30.).drag_region(true),
                            ],
                        )
                        .height(30.)
                        .no_drag(true),
                    ],
                )
                .drag_region(true),
                // Une exclusion levée (`data-no-drag={false}`) ne retire rien.
                column(7, [space(8).height(20.).no_drag(true).no_drag(false)]).drag_region(true),
            ],
        )
    });
    prepare(&mut ui, &mut Measure::default());
    assert!(ui.drag_region_at(center(&ui, 3)), "zone de déplacement");
    assert!(!ui.drag_region_at(center(&ui, 5)), "partie exclue");
    assert!(ui.drag_region_at(center(&ui, 6)), "zone imbriquée");
    assert!(ui.drag_region_at(center(&ui, 8)), "exclusion levée");
}
