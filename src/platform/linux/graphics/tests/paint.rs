//! Fond, formes, traits, clips, dégradés et images, aux échelles 1, 1,5 et 2.
use super::*;
use crate::ui::{
    rendering::brush::{GradientStop, LinearGradient},
    scene::{DrawCommand, Image, PointF, RectF},
};
use std::sync::Arc;

#[test]
fn clear_fill_stroke_and_clip_land_on_the_expected_pixels_at_every_scale() {
    for scale in [1f32, 1.5, 2.] {
        let mut scene = Scene::new(Color::rgb(0, 0, 255));
        scene
            .commands
            .push(DrawCommand::PushClip(RectF::new(0., 0., 20., 40.)));
        scene.commands.push(DrawCommand::Fill {
            rect: RectF::new(10., 10., 20., 20.),
            radius: 0.,
            color: Color::rgb(255, 0, 0),
        });
        scene.commands.push(DrawCommand::PopClip);
        scene.commands.push(DrawCommand::Stroke {
            rect: RectF::new(2., 32., 36., 6.),
            radius: 0.,
            width: 2.,
            color: Color::rgb(0, 255, 0),
        });
        let (w, h) = ((40. * scale) as u32, (40. * scale) as u32);
        let px = renderer().render(&scene, w, h, scale).unwrap();
        let at = |x: f32, y: f32| pixel(&px, w, (x * scale) as u32, (y * scale) as u32);
        // BGRA : fond bleu opaque, carré rouge coupé à x = 20, trait vert.
        assert_eq!(at(1., 1.), [255, 0, 0, 255], "scale {scale}");
        assert_eq!(at(15., 15.), [0, 0, 255, 255], "scale {scale}");
        assert_eq!(at(25., 15.), [255, 0, 0, 255], "scale {scale}");
        assert_eq!(at(20., 32.), [0, 255, 0, 255], "scale {scale}");
    }
}
#[test]
fn rounded_corners_are_transparent_and_lines_and_paths_are_painted() {
    let mut scene = clear();
    scene.commands.push(DrawCommand::Shape {
        rect: RectF::new(0., 0., 40., 40.),
        radii: [20., 0., 0., 0.],
        fill: Some(Color::rgb(255, 255, 255).into()),
        stroke: None,
    });
    scene.commands.push(DrawCommand::Line {
        start: PointF { x: 0., y: 45. },
        end: PointF { x: 40., y: 45. },
        width: 2.,
        color: Color::rgb(255, 0, 0),
    });
    scene.commands.push(DrawCommand::Path {
        points: vec![
            PointF { x: 0., y: 50. },
            PointF { x: 40., y: 50. },
            PointF { x: 0., y: 60. },
        ],
        closed: true,
        color: Color::rgb(0, 255, 0),
    });
    let px = renderer().render(&scene, 40, 60, 1.).unwrap();
    assert_eq!(pixel(&px, 40, 1, 1)[3], 0, "coin arrondi transparent");
    assert_eq!(pixel(&px, 40, 38, 1), [255, 255, 255, 255]);
    assert_eq!(pixel(&px, 40, 20, 45), [0, 0, 255, 255]);
    assert_eq!(pixel(&px, 40, 5, 52), [0, 255, 0, 255]);
}
#[test]
fn linear_gradients_interpolate_and_images_keep_their_orientation() {
    let mut scene = clear();
    scene.commands.push(DrawCommand::Gradient {
        rect: RectF::new(0., 0., 100., 10.),
        radius: 0.,
        gradient: Arc::new(LinearGradient::new(
            0.,
            [
                GradientStop {
                    position: 0.,
                    color: Color::rgb(0, 0, 0),
                },
                GradientStop {
                    position: 1.,
                    color: Color::rgb(255, 255, 255),
                },
            ],
        )),
    });
    // Image 1 × 2 : rouge en haut, vert en bas.
    let image = Arc::new(Image::new(1, 2, vec![0, 0, 255, 255, 0, 255, 0, 255]).unwrap());
    scene.commands.push(DrawCommand::Image {
        rect: RectF::new(0., 20., 10., 20.),
        image,
    });
    let px = renderer().render(&scene, 100, 40, 1.).unwrap();
    let (left, middle, right) = (
        pixel(&px, 100, 2, 5),
        pixel(&px, 100, 50, 5),
        pixel(&px, 100, 97, 5),
    );
    assert!(left[0] < 20 && right[0] > 235, "{left:?} {right:?}");
    assert!((100..156).contains(&middle[0]), "{middle:?}");
    assert_eq!(pixel(&px, 100, 5, 22)[2], 255, "haut de l'image en haut");
    assert_eq!(pixel(&px, 100, 5, 38)[1], 255, "bas de l'image en bas");
}
