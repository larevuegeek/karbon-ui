//! Animations à étapes de dimensions (`@keyframes` qui animent `width` / `height`) :
//! échantillonnées pendant la résolution des styles, un layout par image, départ d'une
//! taille nulle, pourcentages, remplissage et mouvements réduits.
use super::at;
use crate::ui::tests::support::*;
use crate::ui::{
    animation::{Animation, Easing, FillMode, Keyframe},
    element::Length,
    layout,
};
use std::time::{Duration, Instant};

/// `@keyframes eq-bar-1 { 0%, 100% { height: 4px } 50% { height: 18px } }` de RustMusic.
const BAR: &[Keyframe] = &[
    Keyframe::at(0.).height(Length::Px(4.)),
    Keyframe::at(0.5).height(Length::Px(18.)),
    Keyframe::at(1.).height(Length::Px(4.)),
];
/// `@keyframes toast-progress { from { width: 100% } to { width: 0% } }`.
const PROGRESS: &[Keyframe] = &[
    Keyframe::at(0.).width(Length::Percent(100.)),
    Keyframe::at(1.).width(Length::Percent(0.)),
];

fn rect(ui: &UiRuntime, id: u64) -> crate::ui::scene::RectF {
    ui.root().unwrap().find(ElementId(id)).unwrap().rect
}
fn after(ui: &mut UiRuntime, start: Instant, ms: u64) {
    at(ui, start + Duration::from_millis(ms));
}

/// Barre d'égaliseur sans hauteur déclarée, au bas d'une rangée (`items-end`), suivie
/// d'un voisin : la hauteur suit les étapes, le voisin suit la barre.
fn equalizer(animation: Animation) -> UiRuntime {
    view_runtime(move || {
        Element::column(
            1,
            vec![
                Element::column(2, vec![]).width(3).animate(animation),
                widgets::button(3, "Après", Msg::Click(3)).height(20),
            ],
        )
    })
}

#[test]
fn height_keyframes_lay_the_tree_out_every_frame() {
    let start = Instant::now();
    let mut ui = equalizer(
        Animation::keyframes(BAR)
            .duration(800)
            .easing(Easing::Linear)
            .forever(),
    );
    after(&mut ui, start, 0);
    near(layout::height(rect(&ui, 2)), 4.);
    assert!(ui.animating_styles());
    let layouts = ui.stats().layouts;
    after(&mut ui, start, 200);
    // Linéaire : à mi-chemin de 4 → 18.
    near(layout::height(rect(&ui, 2)), 11.);
    assert_eq!(ui.stats().layouts, layouts + 1, "one layout per frame");
    assert_eq!(rect(&ui, 3).top, rect(&ui, 2).bottom);
    after(&mut ui, start, 400);
    near(layout::height(rect(&ui, 2)), 18.);
    // Deuxième itération : retour à 4 px en fin de cycle.
    after(&mut ui, start, 1600);
    near(layout::height(rect(&ui, 2)), 4.);
    assert!(ui.next_wake().is_some());
}

/// Pendant le délai, sans remplissage arrière, la barre garde sa hauteur `auto` (0) : elle
/// doit quand même demander l'image de son départ.
#[test]
fn a_zero_sized_element_still_wakes_for_its_delayed_size_animation() {
    let start = Instant::now();
    let mut ui = equalizer(
        Animation::keyframes(BAR)
            .duration(600)
            .delay(100)
            .easing(Easing::Linear)
            .forever()
            .fill_mode(FillMode::None),
    );
    after(&mut ui, start, 0);
    assert_eq!(layout::height(rect(&ui, 2)), 0.);
    let wake = ui.next_wake().expect("the delay end must be scheduled");
    assert!(wake <= start + Duration::from_millis(100), "{wake:?}");
    after(&mut ui, start, 100);
    near(layout::height(rect(&ui, 2)), 4.);
    after(&mut ui, start, 250);
    near(layout::height(rect(&ui, 2)), 11.);
}

#[test]
fn percentage_widths_interpolate_and_fill_forwards() {
    let start = Instant::now();
    let mut ui = view_runtime(|| {
        let bar = Element::column(2, vec![]).height(2).animate(
            Animation::keyframes(PROGRESS)
                .duration(1000)
                .easing(Easing::Linear)
                .fill_mode(FillMode::Forwards),
        );
        Element::column(1, vec![Element::column(3, vec![bar]).width(200)])
    });
    after(&mut ui, start, 0);
    near(layout::width(rect(&ui, 2)), 200.);
    after(&mut ui, start, 250);
    near(layout::width(rect(&ui, 2)), 150.);
    after(&mut ui, start, 1000);
    assert_eq!(layout::width(rect(&ui, 2)), 0.);
    after(&mut ui, start, 1100);
    assert_eq!(layout::width(rect(&ui, 2)), 0.);
    assert!(ui.next_wake().is_none());
    assert!(!ui.animating_styles());
}

#[test]
fn reduced_motion_shows_the_final_size_and_mixed_units_switch_halfway() {
    let start = Instant::now();
    let mut ui = equalizer(Animation::keyframes(BAR).duration(800).forever());
    ui.set_reduced_motion(true);
    after(&mut ui, start, 0);
    near(layout::height(rect(&ui, 2)), 4.);
    assert!(ui.next_wake().is_none());
    // Pixels vers pourcentage : animation discrète, bascule à mi-parcours.
    const MIXED: &[Keyframe] = &[
        Keyframe::at(0.).height(Length::Px(10.)),
        Keyframe::at(1.).height(Length::Percent(50.)),
    ];
    let mut ui = view_runtime(|| {
        let bar = Element::column(2, vec![]).width(3).animate(
            Animation::keyframes(MIXED)
                .duration(100)
                .easing(Easing::Linear),
        );
        Element::column(1, vec![Element::column(3, vec![bar]).height(100)])
    });
    after(&mut ui, start, 0);
    near(layout::height(rect(&ui, 2)), 10.);
    after(&mut ui, start, 40);
    near(layout::height(rect(&ui, 2)), 10.);
    after(&mut ui, start, 60);
    near(layout::height(rect(&ui, 2)), 50.);
}

#[test]
fn fractional_or_negative_keyframe_sizes_are_rejected() {
    let check = |frames: &'static [Keyframe]| {
        layout::validate(&widgets::text(1, "A").animate(Animation::keyframes(frames)))
    };
    assert!(crate::error::rejected(check(
        const { &[Keyframe::at(1.).width(Length::Fill(1.))] }
    )));
    assert!(crate::error::rejected(check(
        const { &[Keyframe::at(1.).height(Length::Px(-2.))] }
    )));
    assert!(check(const { &[Keyframe::at(1.).height(Length::Percent(150.))] }).is_ok());
}
