//! Formes à coins indépendants, dégradés radiaux, clip de forme et rayons démesurés.
use super::super::*;
use crate::ui::rendering::brush::{Background, GradientStop, RadialGradient};
pub(super) fn verify_shapes(mode: RenderMode) {
    let mut renderer = Renderer::new(mode).unwrap();
    let mut scene = Scene::new(Color::rgba(0, 0, 0, 0));
    scene.commands.push(DrawCommand::Shape {
        rect: RectF::new(5., 5., 40., 30.),
        radii: [12., 0., 10., 0.],
        fill: Some(Color::rgb(0, 255, 0).into()),
        stroke: None,
    });
    scene.commands.push(DrawCommand::Shape {
        rect: RectF::new(55., 5., 40., 30.),
        radii: [12., 0., 10., 0.],
        fill: None,
        stroke: Some((2., Color::rgb(0, 255, 0))),
    });
    let gradient = Arc::new(RadialGradient::new(
        [0.5, 0.5],
        [0.5, 0.5],
        [
            GradientStop {
                position: 0.,
                color: Color::rgb(255, 0, 0),
            },
            GradientStop {
                position: 1.,
                color: Color::rgba(0, 0, 255, 128),
            },
        ],
    ));
    scene.commands.push(DrawCommand::Shape {
        rect: RectF::new(5., 45., 40., 40.),
        radii: [5., 8., 12., 2.],
        fill: Some(Background::Radial(gradient)),
        stroke: None,
    });
    for scale in [1., 1.5, 2.] {
        let w = (100. * scale) as u32;
        let pixels = renderer.render(&scene, w, w, scale).unwrap();
        let pixel = |x: f32, y: f32| {
            let i = ((y * scale) as usize * w as usize + (x * scale) as usize) * 4;
            &pixels[i..i + 4]
        };
        assert!(pixel(5., 5.)[3] < 20);
        assert!(pixel(43., 6.)[1] > 240);
        assert!(pixel(6., 33.)[1] > 240);
        assert!(pixel(44., 34.)[3] < 20);
        assert!(pixel(75., 6.)[1] > 230);
        assert!(pixel(75., 20.)[3] < 20);
        assert!(pixel(25., 65.)[2] > 220);
        assert!(pixel(43., 65.)[0] > 80);
        assert!(pixel(5., 45.)[3] < 20);
        assert_eq!(renderer.target.as_ref().unwrap().shapes.len(), 3);
        assert_eq!(renderer.target.as_ref().unwrap().gradients.radial_len(), 1);
        renderer.render(&scene, w, w, scale).unwrap();
        assert_eq!(renderer.target.as_ref().unwrap().shapes.len(), 3);
    }
    scene.commands.clear();
    renderer.render(&scene, 100, 100, 1.).unwrap();
    assert_eq!(renderer.target.as_ref().unwrap().shapes.len(), 0);
    assert_eq!(renderer.target.as_ref().unwrap().gradients.radial_len(), 0);
    scene.commands.push(DrawCommand::PushShapeClip {
        rect: RectF::new(5., 5., 40., 30.),
        radii: [12., 0., 10., 0.],
    });
    scene.commands.push(DrawCommand::PushEffect {
        transform: crate::ui::rendering::transform::Affine::IDENTITY,
        opacity: 0.5,
        bounds: RectF::new(0., 0., 60., 50.),
    });
    scene.fill(RectF::new(0., 0., 60., 50.), 0., Color::rgb(0, 255, 0));
    scene.commands.push(DrawCommand::PopEffect);
    scene.commands.push(DrawCommand::PopShapeClip);
    for scale in [1., 1.5, 2.] {
        let w = (100. * scale) as u32;
        let pixels = renderer.render(&scene, w, w, scale).unwrap();
        let pixel = |x: f32, y: f32| {
            let i = ((y * scale) as usize * w as usize + (x * scale) as usize) * 4;
            &pixels[i..i + 4]
        };
        assert!(pixel(5., 5.)[3] < 20);
        assert!((pixel(43., 6.)[3] as i16 - 128).abs() <= 1);
        assert!((pixel(25., 20.)[1] as i16 - 128).abs() <= 1);
        assert!(pixel(44., 34.)[3] < 20);
        assert_eq!(pixel(50., 20.)[3], 0);
    }
    scene.commands.clear();
    renderer.render(&scene, 100, 100, 1.).unwrap();
    assert_eq!(renderer.target.as_ref().unwrap().shapes.len(), 0);
}
#[test]
fn software_independent_corners_radial_gradients_alpha_dpi_and_resource_release() {
    verify_shapes(RenderMode::Software);
}

/// Un rayon uniforme démesuré (`border-radius: 9999px`) donne une pilule : les deux rayons
/// valent la moitié du petit côté, comme la normalisation CSS, et non une ellipse.
#[test]
fn oversized_uniform_radius_draws_a_pill_not_an_ellipse() {
    let pill = rounded(RectF::new(10., 20., 200., 40.), 9999.);
    assert_eq!((pill.radiusX, pill.radiusY), (20., 20.));
    let small = rounded(RectF::new(0., 0., 200., 40.), 12.);
    assert_eq!((small.radiusX, small.radiusY), (12., 12.));
}
