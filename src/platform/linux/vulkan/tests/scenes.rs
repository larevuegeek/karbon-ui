//! Scènes traduites (`encode/`) et rendues par Vulkan, comparées au rendu Cairo des mêmes
//! scènes à 1x, 1,5x et 2x : formes, traits, segments, dégradés, découpes, transformations,
//! opacité d'un élément et d'un groupe. Texte, images et effets : `text.rs`, `images.rs`,
//! `filters.rs`, `layers.rs`.
use super::compare;
use crate::ui::{
    rendering::{
        brush::{Background, GradientStop, LinearGradient, RadialGradient},
        transform::Affine,
    },
    scene::{DrawCommand, PointF, RectF, Scene},
    theme::Color,
};
use std::sync::Arc;

fn stops(a: Color, b: Color) -> Vec<GradientStop> {
    vec![
        GradientStop {
            position: 0.,
            color: a,
        },
        GradientStop {
            position: 1.,
            color: b,
        },
    ]
}
/// Rotation de `angle` radians autour de `center`.
fn rotation(angle: f32, center: PointF) -> Affine {
    let (s, c) = angle.sin_cos();
    let turn = Affine {
        m11: c,
        m12: s,
        m21: -s,
        m22: c,
        ..Affine::IDENTITY
    };
    Affine::translation(-center.x, -center.y)
        .then(turn)
        .then(Affine::translation(center.x, center.y))
}
#[test]
fn shapes_strokes_segments_and_gradients_match_cairo() {
    let mut scene = Scene::new(Color::rgb(24, 26, 34));
    scene.commands.extend([
        DrawCommand::Fill {
            rect: RectF::new(8., 8., 60., 36.),
            radius: 10.,
            color: Color::rgb(220, 60, 70),
        },
        DrawCommand::Shape {
            rect: RectF::new(76., 8., 60., 36.),
            radii: [16., 2., 0., 10.],
            fill: Some(Background::Solid(Color::rgba(60, 200, 120, 220))),
            stroke: None,
        },
        DrawCommand::Shape {
            rect: RectF::new(144., 8., 60., 36.),
            radii: [8.; 4],
            fill: None,
            stroke: Some((3., Color::rgb(240, 200, 40))),
        },
        DrawCommand::Stroke {
            rect: RectF::new(8.5, 52.5, 60., 30.),
            radius: 0.,
            width: 1.,
            color: Color::rgb(255, 255, 255),
        },
        DrawCommand::Line {
            start: PointF { x: 76., y: 60. },
            end: PointF { x: 136., y: 60. },
            width: 2.,
            color: Color::rgb(120, 180, 255),
        },
        DrawCommand::Line {
            start: PointF { x: 76., y: 84. },
            end: PointF { x: 136., y: 66. },
            width: 1.5,
            color: Color::rgb(255, 255, 255),
        },
        DrawCommand::Gradient {
            rect: RectF::new(144., 52., 60., 30.),
            radius: 6.,
            gradient: Arc::new(LinearGradient::new(
                45.,
                stops(Color::rgb(255, 0, 120), Color::rgb(0, 120, 255)),
            )),
        },
        DrawCommand::RadialGradient {
            rect: RectF::new(8., 90., 196., 40.),
            radius: 12.,
            gradient: Arc::new(RadialGradient::new(
                [0.3, 0.5],
                [0.4, 0.6],
                stops(Color::rgb(255, 255, 255), Color::rgba(40, 40, 120, 255)),
            )),
        },
    ]);
    compare("formes", &scene, 212., 138., 0.5, 0.002);
}
#[test]
fn clips_transforms_and_group_opacity_match_cairo() {
    let mut scene = Scene::new(Color::rgb(240, 240, 244));
    let fill = |rect: RectF, color: Color| DrawCommand::Fill {
        rect,
        radius: 0.,
        color,
    };
    scene.commands.extend([
        // Découpe rectangulaire puis arrondie.
        DrawCommand::PushClip(RectF::new(8., 8., 50., 40.)),
        fill(RectF::new(0., 0., 80., 80.), Color::rgb(30, 120, 220)),
        DrawCommand::PopClip,
        DrawCommand::PushShapeClip {
            rect: RectF::new(66., 8., 50., 40.),
            radii: [20.; 4],
        },
        fill(RectF::new(60., 0., 80., 80.), Color::rgb(220, 90, 30)),
        DrawCommand::PopShapeClip,
        // Élément seul à demi opaque, puis rotation.
        DrawCommand::PushEffect {
            transform: Affine::IDENTITY,
            opacity: 0.5,
            bounds: RectF::new(124., 8., 50., 40.),
        },
        fill(RectF::new(124., 8., 50., 40.), Color::rgb(20, 160, 90)),
        DrawCommand::PopEffect,
        DrawCommand::PushEffect {
            transform: rotation(0.3, PointF { x: 40., y: 90. }),
            opacity: 1.,
            bounds: RectF::new(15., 75., 50., 30.),
        },
        DrawCommand::Fill {
            rect: RectF::new(15., 75., 50., 30.),
            radius: 8.,
            color: Color::rgb(150, 40, 200),
        },
        DrawCommand::PopEffect,
        // Groupe de deux éléments à opacité : couche intermédiaire recomposée.
        DrawCommand::PushEffect {
            transform: Affine::IDENTITY,
            opacity: 0.6,
            bounds: RectF::new(90., 60., 80., 50.),
        },
        fill(RectF::new(90., 60., 50., 40.), Color::rgb(230, 30, 60)),
        fill(RectF::new(120., 70., 50., 40.), Color::rgb(30, 60, 230)),
        DrawCommand::PopEffect,
    ]);
    compare("decoupes-groupes", &scene, 180., 116., 0.5, 0.003);
}
