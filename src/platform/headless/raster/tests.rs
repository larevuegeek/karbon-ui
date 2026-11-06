//! Tests du rastériseur : formes, clips, effets, flou d'arrière-plan et boîtes de glyphes.
use super::*;
use crate::ui::scene::Image;

fn pixel(pixels: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let offset = (y * width + x) as usize * 4;
    pixels[offset..offset + 4].try_into().unwrap()
}
const RED: Color = Color {
    red: 255,
    green: 0,
    blue: 0,
    alpha: 255,
};
#[test]
fn rounded_fills_strokes_lines_and_clips_are_rasterized_exactly() {
    let mut scene = Scene::new(Color::rgba(0, 0, 0, 0));
    scene.commands.push(DrawCommand::Fill {
        rect: RectF::new(10., 10., 20., 20.),
        radius: 8.,
        color: RED,
    });
    scene.commands.push(DrawCommand::Line {
        start: PointF { x: 0., y: 40.5 },
        end: PointF { x: 50., y: 40.5 },
        width: 1.,
        color: Color::hex(0x00ff00),
    });
    scene
        .commands
        .push(DrawCommand::PushClip(RectF::new(0., 0., 5., 50.)));
    scene.commands.push(DrawCommand::Stroke {
        rect: RectF::new(2., 2., 46., 30.),
        radius: 0.,
        width: 2.,
        color: Color::hex(0x0000ff),
    });
    scene.commands.push(DrawCommand::PopClip);
    let pixels = rasterize(&scene, 50, 50, 1.);
    assert_eq!(pixel(&pixels, 50, 20, 20), [0, 0, 255, 255]);
    // Coin arrondi : le pixel d'angle reste vide.
    assert_eq!(pixel(&pixels, 50, 10, 10), [0; 4]);
    assert_eq!(pixel(&pixels, 50, 25, 40), [0, 255, 0, 255]);
    assert_eq!(pixel(&pixels, 50, 25, 39), [0; 4]);
    // Trait clippé : visible à gauche du clip, absent à droite.
    assert_eq!(pixel(&pixels, 50, 2, 15), [255, 0, 0, 255]);
    assert_eq!(pixel(&pixels, 50, 47, 15), [0; 4]);
    assert_eq!(pixel(&pixels, 50, 20, 15), [0, 0, 255, 255]);
}
#[test]
fn effects_translate_scale_and_fade_their_content() {
    let mut scene = Scene::new(Color::hex(0xffffff));
    scene.commands.push(DrawCommand::PushEffect {
        transform: Affine::translation(20., 0.),
        opacity: 0.5,
        bounds: RectF::new(0., 0., 10., 10.),
    });
    scene.commands.push(DrawCommand::Fill {
        rect: RectF::new(0., 0., 10., 10.),
        radius: 0.,
        color: Color::hex(0x000000),
    });
    scene.commands.push(DrawCommand::PopEffect);
    let pixels = rasterize(&scene, 60, 20, 2.);
    assert_eq!(pixels.len(), 60 * 20 * 4);
    assert_eq!(pixel(&pixels, 60, 5, 5), [255; 4]);
    let faded = pixel(&pixels, 60, 45, 15);
    assert!(
        faded[0] > 120 && faded[0] < 135 && faded[3] == 255,
        "{faded:?}"
    );
    let image = Image::new(60, 20, pixels).unwrap();
    assert_eq!(image.width, 60);
}
#[test]
fn backdrop_blurs_pixels_already_painted_inside_its_shape() {
    static BLUR: [crate::ui::rendering::filter::Filter; 1] =
        [crate::ui::rendering::filter::Filter::Blur(3.)];
    let mut scene = Scene::new(Color::hex(0x000000));
    for i in 0..10 {
        scene.commands.push(DrawCommand::Fill {
            rect: RectF::new(i as f32 * 4., 0., 2., 20.),
            radius: 0.,
            color: Color::hex(0xffffff),
        });
    }
    scene.commands.push(DrawCommand::Backdrop {
        rect: RectF::new(20., 0., 20., 20.),
        radii: [0.; 4],
        filters: crate::ui::rendering::filter::Filters::new(&BLUR),
    });
    let pixels = rasterize(&scene, 40, 20, 1.);
    assert_eq!(pixel(&pixels, 40, 0, 10), [255; 4]);
    assert_eq!(pixel(&pixels, 40, 2, 10), [0, 0, 0, 255]);
    let blurred = pixel(&pixels, 40, 30, 10);
    assert!((90..170).contains(&blurred[0]), "{blurred:?}");
}
#[test]
fn text_is_drawn_as_glyph_boxes_with_measurer_metrics() {
    let mut scene = Scene::new(Color::rgba(0, 0, 0, 0));
    scene.commands.push(DrawCommand::Text {
        rect: RectF::new(0., 0., 100., 40.),
        text: "a b".into(),
        size: 10.,
        bold: false,
        color: RED,
        wrap: false,
    });
    let pixels = rasterize(&scene, 100, 40, 1.);
    // Avance 6 px, ligne 12,5 px : boîte du « a » vers x = 0,6..5,4, y = 2,5..10.
    assert_eq!(pixel(&pixels, 100, 3, 6), [0, 0, 255, 255]);
    // Espace (x = 6..12) non dessiné, puis « b » à x = 12..18.
    assert_eq!(pixel(&pixels, 100, 9, 6), [0; 4]);
    assert_eq!(pixel(&pixels, 100, 15, 6), [0, 0, 255, 255]);
    assert_eq!(pixel(&pixels, 100, 3, 20), [0; 4]);
}
