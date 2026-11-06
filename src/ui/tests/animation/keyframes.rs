//! Animations à étapes : pistes par propriété (étapes partielles, courbe par étape),
//! directions, délai négatif, pause, listes d'animations et validation.
use super::{at, effect, target};
use crate::ui::tests::support::*;
use crate::ui::{
    animation::{Animation, Easing, Effect, FillMode, Keyframe},
    layout,
};
use std::time::{Duration, Instant};

const FADE: &[Keyframe] = &[Keyframe::at(0.).opacity(0.), Keyframe::at(1.).opacity(1.)];

/// Opacité et translation horizontale présentées à `ms` millisecondes de `start`.
fn pose(ui: &mut UiRuntime, start: Instant, ms: u64) -> (f32, f32) {
    at(ui, start + Duration::from_millis(ms));
    let (matrix, opacity) = effect(ui);
    (opacity, matrix.dx)
}
/// Runtime dont l'animation démarre à `start` (première image).
fn started(animation: Animation, start: Instant) -> UiRuntime {
    let mut ui = target(animation);
    at(&mut ui, start);
    ui
}
fn pair(actual: (f32, f32), expected: (f32, f32)) {
    near(actual.0, expected.0);
    near(actual.1, expected.1);
}

#[test]
fn partial_steps_interpolate_each_property_between_its_own_steps() {
    const FRAMES: &[Keyframe] = &[
        Keyframe::at(0.).opacity(0.),
        Keyframe::at(0.5).transform(Effect::new().translate(40., 0.)),
        Keyframe::at(1.).opacity(1.),
    ];
    let start = Instant::now();
    let mut ui = target(
        Animation::keyframes(FRAMES)
            .duration(1000)
            .easing(Easing::Linear),
    );
    // L'opacité va de 0 à 1 sur toute la durée ; la translation, absente de 0 % et de
    // 100 %, part de la valeur de l'élément (0) et y revient.
    pair(pose(&mut ui, start, 0), (0., 0.));
    pair(pose(&mut ui, start, 250), (0.25, 20.));
    pair(pose(&mut ui, start, 500), (0.5, 40.));
    pair(pose(&mut ui, start, 750), (0.75, 20.));
    pair(pose(&mut ui, start, 1100), (1., 0.));
    assert!(ui.next_wake().is_none());
}

#[test]
fn a_step_easing_runs_until_the_next_step_declaring_the_property() {
    const FRAMES: &[Keyframe] = &[
        Keyframe::at(0.).opacity(0.).easing(Easing::EaseIn),
        Keyframe::at(0.5).transform(Effect::new().translate(10., 0.)),
        Keyframe::at(1.).opacity(1.),
    ];
    let start = Instant::now();
    let mut ui = target(
        Animation::keyframes(FRAMES)
            .duration(1000)
            .easing(Easing::Linear),
    );
    // La courbe de 0 % couvre tout le segment d'opacité (0 → 100 %), l'étape 50 % qui
    // ne déclare que la transformation ne le coupe pas.
    let (opacity, x) = pose(&mut ui, start, 0);
    near(opacity, 0.);
    near(x, 0.);
    let (opacity, x) = pose(&mut ui, start, 500);
    near(opacity, Easing::EaseIn.sample(0.5));
    near(x, 10.);
    let (opacity, x) = pose(&mut ui, start, 750);
    near(opacity, Easing::EaseIn.sample(0.75));
    // Transformation : sans courbe à 50 %, celle de l'animation (linéaire).
    near(x, 5.);
}

#[test]
fn reverse_and_alternate_reverse_directions_and_their_fills() {
    let start = Instant::now();
    let fade = Animation::keyframes(FADE)
        .duration(1000)
        .easing(Easing::Linear)
        .repeat(2);
    let mut ui = started(fade.reverse(), start);
    near(pose(&mut ui, start, 250).0, 0.75);
    near(pose(&mut ui, start, 1250).0, 0.75);
    // Dernière itération à rebours : l'état final est l'étape 0 %.
    near(pose(&mut ui, start, 2100).0, 0.);
    let mut ui = started(fade.alternate().reverse(), start);
    near(pose(&mut ui, start, 250).0, 0.75);
    near(pose(&mut ui, start, 1250).0, 0.25);
    near(pose(&mut ui, start, 2100).0, 1.);
    // Remplissage arrière d'une animation à rebours : l'étape 100 % pendant le délai.
    let mut ui = started(
        fade.reverse().delay(500).fill_mode(FillMode::Backwards),
        start,
    );
    near(pose(&mut ui, start, 100).0, 1.);
    near(pose(&mut ui, start, 750).0, 0.75);
    // Sans remplissage avant : l'élément revient à son état après la fin.
    near(pose(&mut ui, start, 3000).0, 1.);
    assert!(ui.next_wake().is_none());
}

#[test]
fn a_negative_delay_starts_the_animation_advanced() {
    let start = Instant::now();
    let mut ui = target(
        Animation::keyframes(FADE)
            .duration(1000)
            .easing(Easing::Linear)
            .advance(250),
    );
    near(pose(&mut ui, start, 0).0, 0.25);
    near(pose(&mut ui, start, 500).0, 0.75);
    near(pose(&mut ui, start, 760).0, 1.);
    assert!(ui.next_wake().is_none());
}

#[test]
fn several_animations_play_together_and_the_last_wins_per_property() {
    const SLIDE: &[Keyframe] = &[Keyframe::at(1.).transform(Effect::new().translate(100., 0.))];
    const DIM: &[Keyframe] = &[Keyframe::at(0.).opacity(0.2), Keyframe::at(1.).opacity(0.4)];
    const MEMBERS: &[Animation] = &[
        Animation::keyframes(FADE)
            .duration(1000)
            .easing(Easing::Linear),
        Animation::keyframes(SLIDE)
            .duration(2000)
            .easing(Easing::Linear),
        Animation::keyframes(DIM)
            .duration(500)
            .delay(500)
            .easing(Easing::Linear)
            .fill_mode(FillMode::None),
    ];
    let start = Instant::now();
    let mut ui = started(Animation::group(MEMBERS), start);
    // La troisième, encore dans son délai sans remplissage, n'a pas d'effet.
    pair(pose(&mut ui, start, 250), (0.25, 12.5));
    // Active, elle remplace l'opacité de la première ; la translation reste.
    pair(pose(&mut ui, start, 750), (0.3, 37.5));
    // Terminée sans remplissage : la première (finie, remplissage `both`) revient.
    pair(pose(&mut ui, start, 1250), (1., 62.5));
    pair(pose(&mut ui, start, 2100), (1., 100.));
    assert!(ui.next_wake().is_none());
    // Le délai du groupe (`data-stagger`) décale tous les membres.
    let mut ui = started(Animation::group(MEMBERS).delay(1000), start);
    pair(pose(&mut ui, start, 500), (0., 0.));
    pair(pose(&mut ui, start, 1250), (0.25, 12.5));
}

#[test]
fn pausing_freezes_the_clock_and_resuming_does_not_restart() {
    use std::{cell::Cell, rc::Rc};
    let paused = Rc::new(Cell::new(true));
    let copy = Rc::clone(&paused);
    let mut ui = view_runtime(move || {
        Element::column(
            1,
            vec![
                widgets::button(2, "Animé", Msg::Click(2))
                    .height(40)
                    .animate(
                        Animation::keyframes(FADE)
                            .duration(1000)
                            .easing(Easing::Linear)
                            .advance(100)
                            .paused(copy.get()),
                    ),
            ],
        )
    });
    let start = Instant::now();
    near(pose(&mut ui, start, 0).0, 0.1);
    assert!(ui.next_wake().is_none());
    near(pose(&mut ui, start, 400).0, 0.1);
    paused.set(false);
    ui.invalidate();
    near(pose(&mut ui, start, 500).0, 0.1);
    assert!(ui.next_wake().is_some());
    near(pose(&mut ui, start, 750).0, 0.35);
    paused.set(true);
    ui.invalidate();
    near(pose(&mut ui, start, 800).0, 0.4);
    near(pose(&mut ui, start, 2000).0, 0.4);
    paused.set(false);
    ui.invalidate();
    // La reprise compte à partir de l'image qui la constate.
    near(pose(&mut ui, start, 2100).0, 0.4);
    near(pose(&mut ui, start, 2200).0, 0.5);
}

#[test]
fn effect_only_keyframes_do_not_restyle_every_frame_and_reduced_motion_ends_them() {
    let start = Instant::now();
    const MEMBERS: &[Animation] = &[
        Animation::keyframes(FADE).duration(1000).forever(),
        Animation::rotate(90.).duration(500).repeat(2).alternate(),
    ];
    let mut ui = target(Animation::group(MEMBERS));
    at(&mut ui, start);
    at(&mut ui, start + Duration::from_millis(100));
    assert!(ui.next_wake().is_some());
    assert!(!ui.animating_styles());
    ui.set_reduced_motion(true);
    at(&mut ui, start + Duration::from_millis(200));
    // État final de chaque membre : fondu terminé, rotation revenue à 0 (2 allers).
    let (matrix, opacity) = effect(&ui);
    near(opacity, 1.);
    near(matrix.m12, 0.);
    assert!(ui.next_wake().is_none());
}

#[test]
fn malformed_keyframes_and_groups_are_rejected() {
    const TWICE: &[Keyframe] = &[Keyframe::at(0.5).opacity(0.), Keyframe::at(0.5).opacity(1.)];
    const SHARED: &[Keyframe] = &[
        Keyframe::at(0.5).opacity(0.),
        Keyframe::at(0.5).transform(Effect::new().rotate(4.)),
    ];
    const BACKWARDS: &[Keyframe] = &[Keyframe::at(0.8).opacity(0.), Keyframe::at(0.2).opacity(1.)];
    const OUTSIDE: &[Keyframe] = &[Keyframe::at(1.5).opacity(0.)];
    const NESTED: &[Animation] = &[Animation::group(&[Animation::fade_in()])];
    const ZERO: &[Animation] = &[Animation::fade_in(), Animation::fade_in().repeat(0)];
    const MANY: &[Animation] = &[Animation::fade_in(); 17];
    let check = |animation: Animation| layout::validate(&widgets::text(1, "A").animate(animation));
    for animation in [
        Animation::keyframes(TWICE),
        Animation::keyframes(BACKWARDS),
        Animation::keyframes(OUTSIDE),
        Animation::group(NESTED),
        Animation::group(ZERO),
        Animation::group(MANY),
    ] {
        assert!(crate::error::rejected(check(animation)), "{animation:?}");
    }
    // Deux propriétés différentes au même offset, et étapes sans 0 % ni 100 %.
    assert!(check(Animation::keyframes(SHARED)).is_ok());
    assert!(
        check(Animation::keyframes(
            const { &[Keyframe::at(0.5).opacity(0.)] }
        ))
        .is_ok()
    );
}
