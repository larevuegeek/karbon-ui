//! Chemin de dessin de base : alpha, clip, image, chemin, icônes, texte et DPI.
use super::super::*;

pub(super) fn verify(mode: RenderMode) {
    let mut renderer = Renderer::new(mode).expect("création du backend natif");
    println!("Test natif : {}", renderer.label());
    let mut scene = Scene::new(Color::rgba(0, 0, 0, 0));
    scene.fill(
        RectF::new(4., 4., 12., 12.),
        0.,
        Color::rgba(200, 100, 50, 128),
    );
    scene
        .commands
        .push(DrawCommand::PushClip(RectF::new(22., 4., 8., 8.)));
    scene.fill(RectF::new(20., 0., 20., 20.), 0., Color::rgb(255, 0, 0));
    scene.commands.push(DrawCommand::PopClip);
    scene.commands.push(DrawCommand::Image {
        rect: RectF::new(44., 4., 8., 8.),
        image: Arc::new(Image::new(1, 1, vec![255, 0, 0, 255]).unwrap()),
    });
    scene.commands.push(DrawCommand::Path {
        points: vec![
            PointF { x: 70., y: 4. },
            PointF { x: 90., y: 4. },
            PointF { x: 80., y: 24. },
        ],
        closed: true,
        color: Color::rgb(0, 255, 0),
    });
    // Les icônes contiennent plusieurs contours, dont des trous transparents.
    let square = |a, b| crate::ui::vector::VectorContour {
        start: PointF { x: a, y: a },
        segments: [
            PointF { x: b, y: a },
            PointF { x: b, y: b },
            PointF { x: a, y: b },
        ]
        .into_iter()
        .map(crate::ui::vector::VectorSegment::Line)
        .collect(),
    };
    scene.commands.push(DrawCommand::Vector {
        rect: RectF::new(96., 4., 20., 20.),
        icon: Arc::new(crate::ui::vector::VectorIcon {
            width: 20.,
            height: 20.,
            contours: vec![square(0., 20.), square(5., 15.)],
        }),
        color: Color::rgb(255, 255, 0),
        outline: 0.,
    });
    let circle = |radius: f32| {
        use crate::ui::vector::{VectorContour, VectorSegment};
        let mut segments = Vec::new();
        for i in 0..4 {
            let angle = i as f32 * std::f32::consts::FRAC_PI_2;
            let (s, c) = angle.sin_cos();
            let rotate = |x, y| PointF {
                x: 10. + radius * (c * x - s * y),
                y: 10. + radius * (s * x + c * y),
            };
            segments.push(VectorSegment::Curve {
                control1: rotate(1., 0.552_284_8),
                control2: rotate(0.552_284_8, 1.),
                end: rotate(0., 1.),
            });
        }
        VectorContour {
            start: PointF {
                x: 10. + radius,
                y: 10.,
            },
            segments,
        }
    };
    scene.commands.push(DrawCommand::Vector {
        rect: RectF::new(96., 28., 20., 20.),
        icon: Arc::new(crate::ui::vector::VectorIcon {
            width: 20.,
            height: 20.,
            contours: vec![circle(10.), circle(5.)],
        }),
        color: Color::rgb(255, 255, 0),
        outline: 0.,
    });
    scene.text(
        RectF::new(4., 50., 112., 40.),
        "Été العربية",
        16.,
        false,
        Color::rgb(255, 255, 255),
    );
    for scale in [1., 1.5, 2.] {
        let width = (120. * scale) as u32;
        let height = (96. * scale) as u32;
        let pixels = renderer
            .render(&scene, width, height, scale)
            .expect("rendu et lecture des pixels");
        let mut surface =
            super::super::super::surface::LayeredSurface::new(width as i32, height as i32).unwrap();
        let address = surface.pixels_mut().as_ptr();
        for _ in 0..2 {
            surface.pixels_mut().fill(0xA5);
            renderer
                .render_into(&scene, width, height, scale, surface.pixels_mut())
                .unwrap();
            assert_eq!(
                surface.pixels_mut(),
                pixels,
                "Le chemin DIB doit conserver tous les pixels, alpha compris"
            );
            assert_eq!(surface.pixels_mut().as_ptr(), address);
        }
        let mut invalid = [0xA5; 5];
        assert!(
            renderer
                .render_into(&scene, width, height, scale, &mut invalid)
                .is_err()
        );
        assert_eq!(invalid, [0xA5; 5]);
        let pixel = |x: f32, y: f32| {
            let i = ((y * scale) as usize * width as usize + (x * scale) as usize) * 4;
            &pixels[i..i + 4]
        };
        assert_eq!(pixel(0., 0.), [0, 0, 0, 0]);
        for (actual, expected) in pixel(8., 8.).iter().zip([25i32, 50, 100, 128]) {
            assert!((*actual as i32 - expected).abs() <= 2);
        }
        assert_eq!(pixel(20., 6.)[3], 0);
        assert_eq!(pixel(24., 6.)[3], 255);
        assert_eq!(pixel(35., 6.)[3], 0);
        assert_eq!(pixel(47., 7.), [255, 0, 0, 255]);
        assert_eq!(pixel(80., 10.), [0, 255, 0, 255]);
        assert_eq!(pixel(98., 6.), [0, 255, 255, 255]);
        assert_eq!(pixel(106., 14.), [0, 0, 0, 0]);
        assert_eq!(pixel(106., 30.), [0, 255, 255, 255]);
        assert_eq!(pixel(106., 38.), [0, 0, 0, 0]);
        assert!(
            pixels
                .as_chunks::<4>()
                .0
                .iter()
                .skip((50. * scale) as usize * width as usize)
                .any(|p| p[3] > 0),
            "texte absent"
        );
    }
    // Une scène invalide ne laisse pas le renderer dans un BeginDraw ouvert.
    scene.commands.push(DrawCommand::PopClip);
    assert!(renderer.render(&scene, 120, 96, 1.).is_err());
    scene.commands.pop();
    assert!(renderer.render(&scene, 120, 96, 1.).is_ok());
}
#[test]
fn software_renders_alpha_clip_image_path_text_and_dpi() {
    verify(RenderMode::Software);
}
