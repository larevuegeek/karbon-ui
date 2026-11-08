//! Piste de carrousel : une rangée translatée de `-100 %` dans un parent qui rogne.
//! Les découpes du layout sont prises avant la transformation ; sous elle, elles sont
//! recalculées : la diapo amenée dans la fenêtre est peinte et cliquable.
use super::at;
use crate::ui::tests::support::*;
use crate::ui::{animation::Effect, scene::DrawCommand};
use std::time::Instant;

fn visible(mut node: Element) -> Element {
    let rules = node.layout_rules();
    rules.overflow_x = Overflow::Visible;
    rules.overflow_y = Overflow::Visible;
    node
}

#[test]
fn a_slide_moved_into_a_clipping_parent_is_painted_and_clickable() {
    let mut ui = view_runtime(|| {
        let slide = |id: u64, label: &'static str| {
            visible(Element::column(
                id,
                vec![widgets::button(id + 10, label, Msg::Click(id + 10)).height(40)],
            ))
            .width(100)
        };
        let track = visible(Element::row(3, vec![slide(4, "Un"), slide(5, "Deux")]))
            .width(100)
            .effect(Effect::new().translate_percent(-100., 0.));
        Element::column(
            1,
            vec![Element::column(2, vec![track]).width(100).height(60)],
        )
    });
    at(&mut ui, Instant::now());
    let painted = |ui: &UiRuntime, label: &str| {
        ui.scene().commands.iter().any(|c| match c {
            DrawCommand::Text { text, .. } | DrawCommand::FormattedText { text, .. } => {
                &**text == label
            }
            DrawCommand::RichText { paragraph, .. } => &*paragraph.text == label,
            _ => false,
        })
    };
    assert!(painted(&ui, "Deux"));

    // Le bouton de la deuxième diapo, à 100 px dans le layout, est sous le pointeur à 50 px.
    let top = ui.root().unwrap().find(ElementId(15)).unwrap().rect.top;
    let p = PointF {
        x: 50.,
        y: top + 20.,
    };
    assert!(ui.event(UiEvent::PointerDown(p)).capture);
    assert_eq!(ui.focused(), Some(ElementId(15)));
}

/// Tuile flottante (`translateY` infini) et ombrée de la deuxième diapo, seule animée et
/// seule ombrée : visible une fois la piste translatée, elle demande ses images et ses
/// ombres sont préparées, bien que sa découpe de layout soit vide.
#[test]
fn a_slide_moved_into_view_animates_and_paints_its_shadows() {
    use crate::ui::{
        animation::{Animation, Easing, Keyframe},
        layout,
        rendering::component_shadow::{BoxShadow, Shadows},
        template::Node,
        theme::Color,
    };
    use std::time::Duration;
    const FLOAT: &[Keyframe] = &[
        Keyframe::at(0.).transform(Effect::new().translate(0., 0.)),
        Keyframe::at(0.5).transform(Effect::new().translate(0., -8.)),
        Keyframe::at(1.).transform(Effect::new().translate(0., 0.)),
    ];
    const GLOW: &[BoxShadow] = &[BoxShadow {
        x: 0.,
        y: 4.,
        blur: 8.,
        spread: 0.,
        color: Color::rgba(0, 0, 0, 128),
        inset: false,
    }];
    let mut ui = view_runtime(|| {
        let tile = Node::element(Element::column(15, Vec::new()).width(40).height(40))
            .box_shadows(Shadows::new(GLOW))
            .build()
            .animate(
                Animation::keyframes(FLOAT)
                    .duration(1000)
                    .easing(Easing::Linear)
                    .forever(),
            );
        // Diapo qui rogne son contenu (`overflow: hidden` de l'illustration) : hors de la
        // vue dans le layout, sa découpe et celles de ses enfants y sont vides.
        let slide =
            |id: u64, children: Vec<Element>| Element::column(id, children).padding(20).width(100);
        let track = visible(Element::row(
            3,
            vec![
                slide(4, vec![Element::column(14, Vec::new()).height(40)]),
                slide(5, vec![tile]),
            ],
        ))
        .width(100)
        .effect(Effect::new().translate_percent(-100., 0.));
        Element::column(
            1,
            vec![Element::column(2, vec![track]).width(100).height(100)],
        )
    });
    let start = Instant::now();
    at(&mut ui, start);
    let tile = ui.root().unwrap().find(ElementId(15)).unwrap();
    assert!(layout::width(tile.clip) <= 0., "empty layout clip");
    assert!(
        ui.next_wake().is_some(),
        "the floating tile asks for frames"
    );
    assert!(
        ui.scene()
            .commands
            .iter()
            .any(|c| matches!(c, DrawCommand::Image { .. })),
        "the tile's blurred shadow is painted"
    );
    at(&mut ui, start + Duration::from_millis(250));
    let lifted = ui.scene().commands.iter().any(|c| match c {
        DrawCommand::PushEffect { transform, .. } => (transform.dy + 4.).abs() < 0.5,
        _ => false,
    });
    assert!(lifted, "the tile is a quarter of the way through its float");
}
