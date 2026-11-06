//! Dégradés linéaires natifs : alpha, DPI, opacité de groupe et cache des pinceaux.
use super::super::*;
use crate::ui::rendering::{brush::linear_gradient, transform::Affine};

pub(super) fn verify_gradients(mode: RenderMode) {
    let mut renderer = Renderer::new(mode).unwrap();
    let horizontal = Arc::new(linear_gradient(
        0.,
        [Color::rgb(255, 0, 0), Color::rgb(0, 0, 255)],
    ));
    let vertical = Arc::new(linear_gradient(
        90.,
        [Color::rgba(0, 255, 0, 128), Color::rgba(255, 255, 255, 128)],
    ));
    let mut scene = Scene::new(Color::rgba(0, 0, 0, 0));
    scene.commands.push(DrawCommand::Gradient {
        rect: RectF::new(0., 0., 40., 20.),
        radius: 5.,
        gradient: Arc::clone(&horizontal),
    });
    scene.commands.push(DrawCommand::PushEffect {
        transform: Affine::translation(50., 0.),
        opacity: 0.5,
        bounds: RectF::new(0., 0., 40., 20.),
    });
    scene.commands.push(DrawCommand::Gradient {
        rect: RectF::new(0., 0., 40., 20.),
        radius: 0.,
        gradient: Arc::clone(&horizontal),
    });
    scene.commands.push(DrawCommand::PopEffect);
    scene.commands.push(DrawCommand::Gradient {
        rect: RectF::new(0., 30., 40., 20.),
        radius: 0.,
        gradient: vertical,
    });
    for scale in [1., 1.5, 2.] {
        let width = (100. * scale) as u32;
        let pixels = renderer.render(&scene, width, width, scale).unwrap();
        let pixel = |x: f32, y: f32| {
            let i = ((y * scale) as usize * width as usize + (x * scale) as usize) * 4;
            &pixels[i..i + 4]
        };
        assert!(pixel(3., 10.)[2] > pixel(3., 10.)[0] + 150);
        assert!(pixel(36., 10.)[0] > pixel(36., 10.)[2] + 150);
        assert_eq!(pixel(20., 10.)[3], 255);
        assert!(pixel(0., 0.)[3] < 20, "rounded corner must be transparent");
        assert!((pixel(70., 10.)[3] as i32 - 128).abs() <= 2);
        assert!((pixel(20., 40.)[3] as i32 - 128).abs() <= 2);
        assert!(
            pixel(20., 47.)[0] > pixel(20., 32.)[0] + 60,
            "vertical direction"
        );
        assert_eq!(renderer.target.as_ref().unwrap().gradients.len(), 2);
        // Same target and Arc retain the brushes between frames.
        renderer.render(&scene, width, width, scale).unwrap();
        assert_eq!(renderer.target.as_ref().unwrap().gradients.len(), 2);
    }
    scene.commands.clear();
    renderer.render(&scene, 100, 100, 1.).unwrap();
    assert_eq!(renderer.target.as_ref().unwrap().gradients.len(), 0);
}
#[test]
fn software_native_gradients_alpha_dpi_effects_and_cache() {
    verify_gradients(RenderMode::Software);
}
