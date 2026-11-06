//! Opacité de groupe, transformations, clips, fond flouté, masque et remplissage du texte.
use super::super::*;
use crate::ui::rendering::transform::Affine;
pub(super) fn verify_effects(mode: RenderMode) {
    let mut renderer = Renderer::new(mode).unwrap();
    let mut scene = Scene::new(Color::rgba(0, 0, 0, 0));
    scene.commands.push(DrawCommand::PushEffect {
        transform: Affine::translation(10., 5.),
        opacity: 0.5,
        bounds: RectF::new(0., 0., 60., 30.),
    });
    scene.fill(RectF::new(0., 0., 30., 30.), 0., Color::rgb(255, 0, 0));
    scene.fill(RectF::new(15., 0., 30., 30.), 0., Color::rgb(0, 0, 255));
    scene.commands.push(DrawCommand::PushEffect {
        transform: Affine::IDENTITY,
        opacity: 0.5,
        bounds: RectF::new(0., 0., 60., 30.),
    });
    scene.fill(RectF::new(50., 0., 8., 8.), 0., Color::rgb(255, 255, 255));
    scene.commands.push(DrawCommand::PopEffect);
    scene.commands.push(DrawCommand::PopEffect);
    scene.fill(RectF::new(70., 0., 10., 10.), 0., Color::rgb(0, 255, 0));
    scene
        .commands
        .push(DrawCommand::PushClip(RectF::new(0., 40., 40., 40.)));
    scene.commands.push(DrawCommand::PushEffect {
        transform: crate::ui::animation::Effect::new()
            .rotate(90.)
            .matrix(RectF::new(0., 40., 20., 10.), (0.5, 0.5)),
        opacity: 1.,
        bounds: RectF::new(0., 30., 30., 30.),
    });
    scene.fill(RectF::new(0., 40., 20., 10.), 0., Color::rgb(255, 255, 0));
    scene.commands.push(DrawCommand::PopEffect);
    scene.commands.push(DrawCommand::PopClip);
    for scale in [1., 1.5, 2.] {
        let width = (100. * scale) as u32;
        let pixels = renderer.render(&scene, width, width, scale).unwrap();
        let pixel = |x: f32, y: f32| {
            let i = ((y * scale) as usize * width as usize + (x * scale) as usize) * 4;
            &pixels[i..i + 4]
        };
        for (x, color) in [(15., [0i32, 0, 128, 128]), (30., [128, 0, 0, 128])] {
            for (a, b) in pixel(x, 15.).iter().zip(color) {
                assert!(
                    (*a as i32 - b).abs() <= 2,
                    "alpha de groupe incorrect : {:?}",
                    pixel(x, 15.)
                );
            }
        }
        assert_eq!(pixel(72., 3.), [0, 255, 0, 255]);
        for value in pixel(64., 8.) {
            assert!((*value as i32 - 64).abs() <= 2);
        }
        assert_eq!(pixel(10., 42.), [0, 255, 255, 255]);
        assert_eq!(pixel(10., 36.), [0, 0, 0, 0]);
    }
    // Une erreur de géométrie au milieu de la pile doit restaurer calque, clip et matrice.
    scene
        .commands
        .insert(1, DrawCommand::PushClip(RectF::new(0., 0., 60., 30.)));
    scene.commands.insert(
        2,
        DrawCommand::Vector {
            rect: RectF::new(0., 0., 10., 10.),
            icon: Arc::new(crate::ui::vector::VectorIcon {
                width: 0.,
                height: 10.,
                contours: vec![],
            }),
            color: Color::rgb(255, 0, 0),
            outline: 0.,
        },
    );
    scene.commands.insert(3, DrawCommand::PopClip);
    assert!(renderer.render(&scene, 100, 100, 1.).is_err());
    let mut clean = Scene::new(Color::rgba(0, 0, 0, 0));
    clean.fill(RectF::new(0., 0., 10., 10.), 0., Color::rgb(0, 255, 0));
    let pixels = renderer.render(&clean, 100, 100, 1.).unwrap();
    assert_eq!(
        &pixels[(5 * 100 + 5) * 4..(5 * 100 + 5) * 4 + 4],
        [0, 255, 0, 255]
    );
}
#[test]
fn software_group_opacity_transform_clipping_and_error_unwind() {
    verify_effects(RenderMode::Software);
}

/// Fond flouté (`backdrop-filter`), masque en dégradé (`mask-image`) et texte rempli
/// par un dégradé (`background-clip: text`), sur WARP.
#[test]
fn software_backdrop_mask_and_text_fill() {
    use crate::ui::rendering::{
        brush::{Background, GradientStop, LinearGradient},
        filter::{Filter, Filters},
    };
    let mut renderer = Renderer::new(RenderMode::Software).unwrap();
    let mut scene = Scene::new(Color::rgb(0, 0, 0));
    // Bandes verticales blanches de 4 px sous la zone floutée.
    for i in 0..20 {
        scene.fill(
            RectF::new(i as f32 * 8., 0., 4., 40.),
            0.,
            Color::rgb(255, 255, 255),
        );
    }
    static BLUR: [Filter; 1] = [Filter::Blur(6.)];
    scene.commands.push(DrawCommand::Backdrop {
        rect: RectF::new(40., 0., 80., 40.),
        radii: [0.; 4],
        filters: Filters::new(&BLUR),
    });
    // Masque : opaque à gauche, transparent à droite.
    let fade = Arc::new(LinearGradient::new(
        0.,
        [
            GradientStop {
                position: 0.,
                color: Color::rgba(0, 0, 0, 255),
            },
            GradientStop {
                position: 1.,
                color: Color::rgba(0, 0, 0, 0),
            },
        ],
    ));
    scene.commands.push(DrawCommand::PushMask {
        bounds: RectF::new(0., 50., 100., 20.),
        gradient: fade,
    });
    scene.fill(RectF::new(0., 50., 100., 20.), 0., Color::rgb(255, 0, 0));
    scene.commands.push(DrawCommand::PopMask);
    let rainbow = Arc::new(LinearGradient::new(
        0.,
        [
            GradientStop {
                position: 0.,
                color: Color::rgb(0, 255, 0),
            },
            GradientStop {
                position: 1.,
                color: Color::rgb(0, 0, 255),
            },
        ],
    ));
    scene.commands.push(DrawCommand::PushTextFill {
        rect: RectF::new(0., 80., 160., 40.),
        fill: Background::Linear(rainbow),
    });
    scene.commands.push(DrawCommand::FontText {
        rect: RectF::new(0., 80., 160., 40.),
        text: Arc::from("██████████"),
        size: 30.,
        font: Font::system(true),
        color: Color::rgb(255, 0, 0),
        wrap: false,
    });
    scene.commands.push(DrawCommand::PopTextFill);
    let pixels = renderer.render(&scene, 160, 120, 1.).unwrap();
    let px = |x: usize, y: usize| {
        let i = (y * 160 + x) * 4;
        [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]
    };
    // Hors du flou : bandes nettes ; dedans : gris intermédiaire.
    assert_eq!(px(2, 20)[0], 255);
    assert_eq!(px(6, 20)[0], 0);
    let blurred = px(82, 20);
    assert!((60..200).contains(&blurred[0]), "{blurred:?}");
    // Masque : rouge plein à gauche, presque rien à droite (BGRA).
    assert!(px(2, 60)[2] > 230, "{:?}", px(2, 60));
    assert!(px(97, 60)[2] < 30, "{:?}", px(97, 60));
    // Texte : vert à gauche, bleu à droite, jamais rouge.
    let mut left = [0u32; 3];
    let mut right = [0u32; 3];
    for y in 80..120 {
        for x in 0..160 {
            let p = px(x, y);
            let target = if x < 50 {
                &mut left
            } else if x > 110 {
                &mut right
            } else {
                continue;
            };
            target[0] += p[0] as u32;
            target[1] += p[1] as u32;
            target[2] += p[2] as u32;
        }
    }
    assert!(
        left[1] > left[0] && right[0] > right[1],
        "{left:?} {right:?}"
    );
    assert!(
        left[2] < left[1] / 4 && right[2] < right[0] / 4,
        "{left:?} {right:?}"
    );
}
