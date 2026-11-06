//! Découpes, transformations et opacité de groupe.
use super::{both, close, metal, pixel};
use crate::platform::macos::graphics::Renderer as CgRenderer;
use crate::ui::{
    rendering::transform::Affine,
    scene::{DrawCommand, RectF, Scene},
    theme::Color,
};

fn fill(rect: RectF, color: Color) -> DrawCommand {
    DrawCommand::Fill {
        rect,
        radius: 0.,
        color,
    }
}
fn rotation(degrees: f32, cx: f32, cy: f32) -> Affine {
    let (s, c) = degrees.to_radians().sin_cos();
    Affine::translation(-cx, -cy)
        .then(Affine {
            m11: c,
            m12: s,
            m21: -s,
            m22: c,
            dx: 0.,
            dy: 0.,
        })
        .then(Affine::translation(cx, cy))
}
#[test]
fn rectangular_and_rounded_clips_match_core_graphics() {
    let mut scene = Scene::new(Color::rgb(255, 255, 255));
    let c = &mut scene.commands;
    c.push(DrawCommand::PushClip(RectF::new(10.5, 10.25, 50., 40.)));
    c.push(fill(RectF::new(0., 0., 100., 100.), Color::rgb(200, 0, 0)));
    c.push(DrawCommand::PopClip);
    c.push(DrawCommand::PushShapeClip {
        rect: RectF::new(70., 10., 60., 60.),
        radii: [30., 10., 0., 20.],
    });
    c.push(fill(RectF::new(60., 0., 100., 100.), Color::rgb(0, 0, 200)));
    // Découpe arrondie imbriquée.
    c.push(DrawCommand::PushShapeClip {
        rect: RectF::new(90., 30., 30., 30.),
        radii: [15.; 4],
    });
    c.push(fill(RectF::new(60., 0., 100., 100.), Color::rgb(0, 200, 0)));
    c.push(DrawCommand::PopShapeClip);
    c.push(DrawCommand::PopShapeClip);
    close("decoupes", &scene, (140., 80.), 0.5, 0.01);
}
#[test]
fn transforms_and_rotated_clips_match_core_graphics() {
    let mut scene = Scene::new(Color::rgb(255, 255, 255));
    let c = &mut scene.commands;
    let bounds = RectF::new(20., 20., 60., 40.);
    c.push(DrawCommand::PushEffect {
        transform: rotation(20., 50., 40.),
        opacity: 1.,
        bounds,
    });
    c.push(DrawCommand::PushClip(RectF::new(25., 25., 50., 30.)));
    c.push(DrawCommand::Fill {
        rect: bounds,
        radius: 6.,
        color: Color::rgb(30, 90, 200),
    });
    c.push(DrawCommand::PopClip);
    c.push(DrawCommand::Stroke {
        rect: bounds,
        radius: 6.,
        width: 2.,
        color: Color::rgb(0, 0, 0),
    });
    c.push(DrawCommand::PopEffect);
    c.push(DrawCommand::PushEffect {
        transform: Affine {
            m11: 1.5,
            m22: 1.5,
            dx: -50.,
            dy: -5.,
            ..Affine::IDENTITY
        },
        opacity: 1.,
        bounds: RectF::new(100., 10., 30., 30.),
    });
    c.push(fill(RectF::new(100., 10., 30., 30.), Color::rgb(0, 150, 0)));
    c.push(DrawCommand::PopEffect);
    close("transformations", &scene, (160., 80.), 0.8, 0.02);
}
#[test]
fn group_opacity_composes_overlapping_content_once() {
    let mut scene = Scene::new(Color::rgb(255, 255, 255));
    let c = &mut scene.commands;
    let bounds = RectF::new(10., 10., 60., 40.);
    c.push(DrawCommand::PushEffect {
        transform: Affine::IDENTITY,
        opacity: 0.5,
        bounds,
    });
    c.push(fill(RectF::new(10., 10., 40., 40.), Color::rgb(255, 0, 0)));
    c.push(fill(RectF::new(30., 10., 40., 40.), Color::rgb(0, 0, 255)));
    c.push(DrawCommand::PopEffect);
    // Élément seul : opacité appliquée directement.
    c.push(DrawCommand::PushEffect {
        transform: Affine::IDENTITY,
        opacity: 0.25,
        bounds: RectF::new(80., 10., 30., 30.),
    });
    c.push(fill(RectF::new(80., 10., 30., 30.), Color::rgb(0, 0, 0)));
    c.push(DrawCommand::PopEffect);
    // Contenu invisible : sauté.
    c.push(DrawCommand::PushEffect {
        transform: Affine::IDENTITY,
        opacity: 0.,
        bounds: RectF::new(80., 40., 30., 10.),
    });
    c.push(fill(RectF::new(80., 40., 30., 10.), Color::rgb(0, 0, 0)));
    c.push(fill(RectF::new(80., 40., 30., 10.), Color::rgb(0, 0, 0)));
    c.push(DrawCommand::PopEffect);
    close("opacite", &scene, (120., 60.), 0.5, 0.01);
    let Some(mut metal) = metal() else {
        return;
    };
    let mut cg = CgRenderer::new().unwrap();
    let (m, _, w, _) = both(&mut metal, &mut cg, &scene, (120., 60.), 1.);
    // Recouvrement : le bleu (dessus) à moitié sur blanc, sans rouge en dessous.
    let overlap = pixel(&m, w, 40, 30);
    assert!(overlap[0] > 250 && overlap[2] < 135, "{overlap:?}");
    assert_eq!(pixel(&m, w, 95, 45), [255, 255, 255, 255]);
}
#[test]
fn group_inside_a_rounded_clip_is_clipped_at_its_composition() {
    let mut scene = Scene::new(Color::rgb(255, 255, 255));
    let c = &mut scene.commands;
    c.push(DrawCommand::PushShapeClip {
        rect: RectF::new(10., 10., 80., 50.),
        radii: [20.; 4],
    });
    c.push(DrawCommand::PushEffect {
        transform: Affine::translation(3.5, 0.),
        opacity: 0.7,
        bounds: RectF::new(0., 0., 100., 70.),
    });
    c.push(fill(RectF::new(0., 0., 60., 70.), Color::rgb(200, 0, 0)));
    c.push(fill(RectF::new(40., 0., 60., 70.), Color::rgb(0, 160, 0)));
    c.push(DrawCommand::PopEffect);
    c.push(DrawCommand::PopShapeClip);
    close("groupe-decoupe", &scene, (100., 70.), 0.6, 0.01);
}
