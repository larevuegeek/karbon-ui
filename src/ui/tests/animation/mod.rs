//! Animations et transitions : échéances, effets d'entrée, survol, transformations,
//! mouvements réduits et révélation en hauteur.
use crate::ui::tests::support::*;
use crate::ui::{
    animation::{Animation, Easing, Effect, Transition},
    rendering::transform::Affine,
    scene::DrawCommand,
};
use std::time::{Duration, Instant};
mod keyframes;
pub(super) fn at(ui: &mut UiRuntime, time: Instant) {
    ui.prepare_at(
        context(),
        ChromeState::default(),
        &mut Measure::default(),
        time,
    )
    .unwrap();
}
pub(super) fn effect(ui: &UiRuntime) -> (Affine, f32) {
    ui.scene()
        .commands
        .iter()
        .find_map(|c| match c {
            DrawCommand::PushEffect {
                transform, opacity, ..
            } => Some((*transform, *opacity)),
            _ => None,
        })
        .unwrap_or((Affine::IDENTITY, 1.))
}
pub(super) fn target(animation: Animation) -> UiRuntime {
    view_runtime(move || {
        Element::column(
            1,
            vec![
                widgets::button(2, "Animé", Msg::Click(2))
                    .height(40)
                    .animate(animation),
            ],
        )
        .padding(80)
    })
}
#[test]
fn preparing_between_frames_does_not_postpone_the_animation_deadline() {
    let start = Instant::now();
    let mut ui = target(Animation::fade_in().duration(100).easing(Easing::Linear));
    at(&mut ui, start);
    let deadline = ui.next_wake().unwrap();
    // Entrées souris / préparations répétées avant le réveil déjà programmé.
    for ms in [2, 5, 10, 15] {
        at(&mut ui, start + Duration::from_millis(ms));
        assert_eq!(ui.next_wake(), Some(deadline));
    }
    at(&mut ui, deadline);
    assert!(ui.next_wake().unwrap() > deadline);
    at(&mut ui, start + Duration::from_millis(100));
    assert!(ui.next_wake().is_none());
}

/// Une image préparée avant l'ouverture de la fenêtre garde son animation d'entrée au
/// début jusqu'à la première présentation.
#[test]
fn deferred_animations_start_when_the_first_frame_is_presented() {
    let start = Instant::now();
    let mut ui = target(Animation::fade_in().duration(100).easing(Easing::Linear));
    at(&mut ui, start);
    let first = effect(&ui).1;
    let deadline = ui.next_wake().unwrap();
    ui.defer_animations(Duration::from_millis(300));
    assert_eq!(ui.next_wake(), Some(deadline + Duration::from_millis(300)));
    // 50 ms après la présentation : à mi-parcours, pas terminée.
    at(&mut ui, start + Duration::from_millis(350));
    let half = effect(&ui).1;
    assert!(half > first && half < 1., "{first} {half}");
    at(&mut ui, start + Duration::from_millis(400));
    assert!(ui.next_wake().is_none());
}

#[test]
fn fade_delay_completes_without_rebuilding_layout_and_releases_the_wake() {
    let start = Instant::now();
    let mut ui = target(
        Animation::fade_in()
            .delay(100)
            .duration(200)
            .easing(Easing::Linear),
    );
    at(&mut ui, start);
    assert_eq!(effect(&ui).1, 0.);
    assert_eq!(ui.next_wake(), Some(start + Duration::from_millis(100)));
    let before = ui.stats();
    let halfway = start + Duration::from_millis(200);
    assert!(ui.event(UiEvent::Timer(halfway)).repaint);
    at(&mut ui, halfway);
    near(effect(&ui).1, 0.5);
    assert_eq!(ui.stats().views, before.views);
    assert_eq!(ui.stats().layouts, before.layouts);
    at(&mut ui, start + Duration::from_millis(300));
    near(effect(&ui).1, 1.);
    assert!(ui.next_wake().is_none());
    let painted = ui.stats().paints;
    at(&mut ui, start + Duration::from_secs(3));
    assert_eq!(ui.stats().paints, painted);
}
#[test]
fn entry_replays_only_when_key_changes_or_identity_is_reinserted() {
    let key = Rc::new(Cell::new(0));
    let copy = Rc::clone(&key);
    let mut ui = view_runtime(move || {
        Element::column(
            1,
            vec![
                widgets::button(2, "A", Msg::Click(2))
                    .animate(Animation::fade_in().duration(100).key(copy.get())),
            ],
        )
    });
    let start = Instant::now();
    at(&mut ui, start);
    at(&mut ui, start + Duration::from_millis(100));
    assert!(ui.next_wake().is_none());
    ui.invalidate();
    at(&mut ui, start + Duration::from_millis(150));
    assert!(ui.next_wake().is_none());
    key.set(1);
    ui.invalidate();
    at(&mut ui, start + Duration::from_millis(200));
    assert_eq!(effect(&ui).1, 0.);
    assert!(ui.next_wake().is_some());
}
#[test]
fn container_hover_works_over_padding_and_text_without_a_clickable_child() {
    let mut ui = view_runtime(|| {
        Element::column(
            1,
            vec![
                Element::column(2, vec![widgets::text(3, "Carte")])
                    .width(200)
                    .height(100)
                    .padding(20)
                    .hover(Effect::new().opacity(0.5))
                    .transition(Transition::new().duration(100).easing(Easing::Linear)),
            ],
        )
        .padding(60)
    });
    let start = Instant::now();
    at(&mut ui, start);
    let rect = ui.root().unwrap().find(ElementId(2)).unwrap().bounds();
    let p = PointF {
        x: rect.left + 5.,
        y: rect.top + 5.,
    };
    assert!(ui.event(UiEvent::PointerMove(p)).repaint);
    at(&mut ui, start);
    at(&mut ui, start + Duration::from_millis(100));
    near(effect(&ui).1, 0.5);
    ui.event(UiEvent::PointerMove(center(&ui, 3)));
    at(&mut ui, start + Duration::from_millis(100));
    near(effect(&ui).1, 0.5);
    assert_eq!(ui.cursor(), Cursor::Arrow);
    assert!(!ui.event(UiEvent::PointerDown(p)).capture);
    assert!(ui.event(UiEvent::PointerLeave).repaint);
    at(&mut ui, start + Duration::from_millis(100));
    at(&mut ui, start + Duration::from_millis(200));
    near(effect(&ui).1, 1.);
    assert!(ui.next_wake().is_none());
}

#[test]
fn interrupted_hover_transition_starts_at_the_current_value() {
    let mut ui = view_runtime(|| {
        Element::column(
            1,
            vec![
                widgets::button(2, "A", Msg::Click(2))
                    .height(40)
                    .hover(Effect::new().scale(1.2))
                    .transition(Transition::new().duration(100).easing(Easing::Linear)),
            ],
        )
        .padding(80)
    });
    let start = Instant::now();
    at(&mut ui, start);
    let p = center(&ui, 2);
    ui.event(UiEvent::PointerMove(p));
    at(&mut ui, start);
    at(&mut ui, start + Duration::from_millis(50));
    near(effect(&ui).0.m11, 1.1);
    ui.event(UiEvent::PointerMove(PointF { x: 1., y: 41. }));
    at(&mut ui, start + Duration::from_millis(50));
    near(effect(&ui).0.m11, 1.1);
    at(&mut ui, start + Duration::from_millis(100));
    near(effect(&ui).0.m11, 1.05);
    at(&mut ui, start + Duration::from_millis(150));
    near(effect(&ui).0.m11, 1.);
    assert!(ui.next_wake().is_none());
}
#[test]
fn nested_rotation_and_translation_hit_the_visual_button_and_update_accessibility() {
    let mut ui = view_runtime(|| {
        Element::column(
            1,
            vec![
                Element::column(
                    3,
                    vec![widgets::button(2, "A", Msg::Click(2)).width(100).height(40)],
                )
                .width(220)
                .height(160)
                .translate(50., 20.)
                .rotate(30.),
            ],
        )
        .padding(80)
    });
    at(&mut ui, Instant::now());
    let root = ui.root().unwrap();
    let parent = root.find(ElementId(3)).unwrap().bounds();
    let button = root.find(ElementId(2)).unwrap().bounds();
    let matrix = Effect::new()
        .translate(50., 20.)
        .rotate(30.)
        .matrix(parent, (0.5, 0.5));
    let p = matrix.point(PointF {
        x: (button.left + button.right) / 2.,
        y: (button.top + button.bottom) / 2.,
    });
    assert!(ui.event(UiEvent::PointerDown(p)).capture);
    assert_eq!(ui.focused(), Some(ElementId(2)));
    assert!(ui.event(UiEvent::PointerUp(p)).handled);
    let accessible = ui
        .accessibility()
        .into_iter()
        .find(|n| n.id == ElementId(2))
        .unwrap();
    let expected = matrix.bounds(button);
    near(accessible.bounds.left, expected.left);
    near(accessible.bounds.top, expected.top);
}
#[test]
fn repeat_alternate_zero_duration_and_reduced_motion_have_finite_scheduling() {
    let start = Instant::now();
    let mut ui = target(
        Animation::from_to(Effect::new().translate(0., 20.), Effect::NONE)
            .duration(100)
            .easing(Easing::Linear)
            .repeat(2)
            .alternate(),
    );
    at(&mut ui, start);
    at(&mut ui, start + Duration::from_millis(150));
    near(effect(&ui).0.dy, 10.);
    at(&mut ui, start + Duration::from_millis(200));
    near(effect(&ui).0.dy, 20.);
    assert!(ui.next_wake().is_none());
    let mut ui = target(Animation::rotate(360.).duration(0).forever());
    at(&mut ui, start);
    assert!(ui.next_wake().is_none());
    let mut ui = target(Animation::rotate(0.).delay(1000).forever());
    at(&mut ui, start);
    assert!(ui.next_wake().is_none());
    let mut ui = target(Animation::fade_in().delay(1000).forever());
    ui.set_reduced_motion(true);
    at(&mut ui, start);
    assert_eq!(effect(&ui).1, 1.);
    assert!(ui.next_wake().is_none());
}
#[test]
fn removing_an_infinite_animation_cancels_its_timer_and_invalid_configurations_fail() {
    let visible = Rc::new(Cell::new(true));
    let copy = Rc::clone(&visible);
    let mut ui = view_runtime(move || {
        Element::column(
            1,
            if copy.get() {
                vec![widgets::text(2, "A").animate(Animation::rotate(360.).forever())]
            } else {
                vec![]
            },
        )
    });
    let start = Instant::now();
    at(&mut ui, start);
    assert!(ui.next_wake().is_some());
    visible.set(false);
    ui.invalidate();
    at(&mut ui, start + Duration::from_millis(10));
    assert!(ui.next_wake().is_none());
    for animation in [
        Animation::fade_in().repeat(0),
        Animation::rotate(f32::NAN),
        Animation::fade_in().easing(Easing::CubicBezier(-1., 0., 1., 1.)),
    ] {
        assert!(crate::error::rejected(layout::validate(
            &widgets::text(1, "A").animate(animation)
        )));
    }
    assert!(
        layout::validate(&Element::column(
            1,
            vec![widgets::text(2, "A").opacity(0.5)]
        ))
        .is_ok()
    );
    let input = crate::ui::widgets::TextInput::new("", |text| Msg::Text(1, text));
    assert!(crate::error::rejected(layout::validate(
        &Element::custom(1, input).rotate(10.)
    )));
    let input = crate::ui::widgets::TextInput::new("", |text| Msg::Text(1, text));
    assert!(layout::validate(&Element::custom(1, input).animate(Animation::fade_in())).is_ok());
}
#[test]
fn bezier_easing_and_affine_inverse_are_predictable() {
    near(Easing::EaseInOut.sample(0.5), 0.5);
    assert!(Easing::EaseIn.sample(0.5) < 0.5);
    assert!(Easing::EaseOut.sample(0.5) > 0.5);
    assert_eq!(Easing::Ease.sample(0.), 0.);
    assert_eq!(Easing::Ease.sample(1.), 1.);
    let matrix = Effect::new()
        .rotate(47.)
        .scale(1.4)
        .translate(12., -6.)
        .matrix(RectF::new(20., 30., 100., 50.), (0.5, 0.5));
    let p = PointF { x: 40., y: 60. };
    let recovered = matrix.inverse().unwrap().point(matrix.point(p));
    near(recovered.x, p.x);
    near(recovered.y, p.y);
    assert!(
        Effect::new()
            .scale(0.)
            .matrix(RectF::new(0., 0., 10., 10.), (0.5, 0.5))
            .inverse()
            .is_none()
    );
}

#[test]
fn scrolled_out_infinite_effects_do_not_request_frames() {
    let mut ui = view_runtime(|| {
        Element::scroll(
            1,
            Axis::Vertical,
            Element::column(
                2,
                vec![
                    Element::column(3, vec![]).height(600),
                    widgets::button(4, "Boucle", Msg::Click(4))
                        .height(40)
                        .animate(Animation::rotate(360.).forever()),
                ],
            ),
        )
    });
    let start = Instant::now();
    at(&mut ui, start);
    assert!(ui.next_wake().is_none());
    wheel(&mut ui, PointF { x: 20., y: 100. }, 0., 10000.);
    at(&mut ui, start + Duration::from_secs(1));
    assert!(ui.next_wake().is_some());
    wheel(&mut ui, PointF { x: 20., y: 100. }, 0., -10000.);
    at(&mut ui, start + Duration::from_secs(2));
    assert!(ui.next_wake().is_none());
}

#[test]
fn accessibility_geometry_revision_changes_when_the_new_pose_is_presented() {
    let mut ui = target(
        Animation::slide_right(40.)
            .duration(100)
            .easing(Easing::Linear),
    );
    let start = Instant::now();
    at(&mut ui, start);
    let previous = ui.accessibility_version();
    let before = ui
        .accessibility()
        .into_iter()
        .find(|n| n.id == ElementId(2))
        .unwrap()
        .bounds;
    ui.event(UiEvent::Timer(start + Duration::from_millis(50)));
    assert_eq!(ui.accessibility_version(), previous);
    at(&mut ui, start + Duration::from_millis(50));
    let current = ui.accessibility_version();
    assert_eq!(
        (previous.0, previous.1, previous.2),
        (current.0, current.1, current.2)
    );
    assert_ne!(previous.3, current.3);
    let after = ui
        .accessibility()
        .into_iter()
        .find(|n| n.id == ElementId(2))
        .unwrap()
        .bounds;
    near(after.left - before.left, 20.);
}

/// `Element::reveal` : afficher ou masquer (`hidden`) anime la hauteur entre 0 et la
/// hauteur naturelle, contenu disposé à sa taille et rogné ; les voisins suivent.
#[test]
fn reveal_animates_between_zero_and_the_natural_height() {
    let shown = Rc::new(Cell::new(false));
    let flag = shown.clone();
    let mut ui = view_runtime(move || {
        let mut panel = Element::column(
            2,
            vec![
                widgets::button(3, "Un", Msg::Click(3)).height(40),
                widgets::button(4, "Deux", Msg::Click(4)).height(40),
            ],
        )
        .padding(10)
        .reveal(Transition::new().duration(200).easing(Easing::Linear));
        panel.layout_rules().hidden = !flag.get();
        Element::column(
            1,
            vec![panel, widgets::button(5, "Après", Msg::Click(5)).height(20)],
        )
    });
    let rect = |ui: &UiRuntime, id: u64| ui.root().unwrap().find(ElementId(id)).unwrap().rect;
    let height = |ui: &UiRuntime| layout::height(rect(ui, 2));
    let start = Instant::now();
    at(&mut ui, start);
    // Première image masquée : pas d'animation d'apparition.
    assert_eq!(height(&ui), 0.);
    assert_eq!(rect(&ui, 5).top, rect(&ui, 1).top);
    shown.set(true);
    ui.invalidate();
    at(&mut ui, start + Duration::from_millis(10));
    assert_eq!(height(&ui), 0.);
    assert!(ui.next_wake().is_some());
    let layouts = ui.stats().layouts;
    at(&mut ui, start + Duration::from_millis(110));
    // Linéaire : la moitié des 100 px naturels (2 × 40 + 2 × 10).
    near(height(&ui), 50.);
    assert_eq!(ui.stats().layouts, layouts + 1, "one layout per frame");
    assert_eq!(rect(&ui, 5).top, rect(&ui, 2).bottom);
    // Contenu à sa taille naturelle, rogné à la boîte animée.
    assert_eq!(layout::height(rect(&ui, 4)), 40.);
    let clip = ui.root().unwrap().find(ElementId(4)).unwrap().clip;
    assert!(clip.bottom <= rect(&ui, 2).bottom && clip.bottom < rect(&ui, 4).bottom);
    at(&mut ui, start + Duration::from_millis(210));
    assert_eq!(height(&ui), 100.);
    assert!(ui.next_wake().is_none());
    // Fermeture interrompue : repart de la hauteur courante.
    shown.set(false);
    ui.invalidate();
    at(&mut ui, start + Duration::from_millis(300));
    assert_eq!(height(&ui), 100.);
    at(&mut ui, start + Duration::from_millis(350));
    near(height(&ui), 75.);
    shown.set(true);
    ui.invalidate();
    at(&mut ui, start + Duration::from_millis(350));
    at(&mut ui, start + Duration::from_millis(400));
    near(height(&ui), 75. + 25. * 50. / 200.);
    at(&mut ui, start + Duration::from_millis(600));
    assert_eq!(height(&ui), 100.);
    shown.set(false);
    ui.invalidate();
    at(&mut ui, start + Duration::from_millis(700));
    at(&mut ui, start + Duration::from_millis(1000));
    assert_eq!(height(&ui), 0.);
    assert!(layout::hidden(
        ui.root().unwrap().find(ElementId(2)).unwrap()
    ));
    assert!(ui.next_wake().is_none());
    // Mouvements réduits : état final immédiat.
    ui.set_reduced_motion(true);
    shown.set(true);
    ui.invalidate();
    at(&mut ui, start + Duration::from_millis(1010));
    assert_eq!(height(&ui), 100.);
    assert!(ui.next_wake().is_none());
}
