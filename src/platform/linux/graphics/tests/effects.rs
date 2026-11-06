//! Effets : opacité de groupe, transformations, filtres CPU, `backdrop-filter`,
//! masques en dégradé et texte rempli par un dégradé.
use super::*;
use crate::ui::{
    element::ElementId,
    rendering::{
        brush::{Background, GradientStop, LinearGradient},
        filter::{Filter, Filters},
        transform::Affine,
    },
    scene::{DrawCommand, RectF},
};
use std::sync::Arc;

fn fill(rect: RectF, color: Color) -> DrawCommand {
    DrawCommand::Fill {
        rect,
        radius: 0.,
        color,
    }
}
#[test]
fn group_opacity_and_transforms_apply_to_their_content() {
    let mut scene = clear();
    scene.commands.push(DrawCommand::PushEffect {
        transform: Affine::translation(20., 0.),
        opacity: 0.5,
        bounds: RectF::new(0., 0., 10., 10.),
    });
    scene.commands.push(fill(
        RectF::new(0., 0., 10., 10.),
        Color::rgb(255, 255, 255),
    ));
    scene.commands.push(DrawCommand::PopEffect);
    let px = renderer().render(&scene, 80, 20, 2.).unwrap();
    assert_eq!(pixel(&px, 80, 10, 10)[3], 0, "contenu déplacé");
    let moved = pixel(&px, 80, 50, 10);
    assert!((120..=135).contains(&moved[3]), "{moved:?}");
}
#[test]
fn blur_filter_spreads_content_and_colour_filters_change_it() {
    static BLUR: [Filter; 1] = [Filter::Blur(4.)];
    static GRAY: [Filter; 1] = [Filter::Grayscale(1.)];
    let mut scene = clear();
    scene.commands.push(DrawCommand::PushFilter {
        id: ElementId(1),
        bounds: RectF::new(20., 20., 20., 20.),
        filters: Filters::new(&BLUR),
    });
    scene.commands.push(fill(
        RectF::new(20., 20., 20., 20.),
        Color::rgb(255, 255, 255),
    ));
    scene.commands.push(DrawCommand::PopFilter);
    scene.commands.push(DrawCommand::PushFilter {
        id: ElementId(2),
        bounds: RectF::new(60., 20., 20., 20.),
        filters: Filters::new(&GRAY),
    });
    scene
        .commands
        .push(fill(RectF::new(60., 20., 20., 20.), Color::rgb(255, 0, 0)));
    scene.commands.push(DrawCommand::PopFilter);
    let px = renderer().render(&scene, 100, 60, 1.).unwrap();
    assert!(
        pixel(&px, 100, 17, 30)[3] > 20,
        "le flou déborde du contenu"
    );
    assert!(pixel(&px, 100, 30, 30)[3] > 200);
    let gray = pixel(&px, 100, 70, 30);
    assert!(
        gray[0].abs_diff(gray[2]) <= 2 && gray[2] > 20 && gray[2] < 120,
        "{gray:?}"
    );
}
#[test]
fn backdrop_blurs_pixels_already_painted_inside_its_shape() {
    static BLUR: [Filter; 1] = [Filter::Blur(3.)];
    let mut scene = Scene::new(Color::rgb(0, 0, 0));
    scene.commands.push(fill(
        RectF::new(0., 0., 20., 40.),
        Color::rgb(255, 255, 255),
    ));
    scene.commands.push(DrawCommand::Backdrop {
        rect: RectF::new(10., 0., 20., 40.),
        radii: [0.; 4],
        filters: Filters::new(&BLUR),
    });
    let px = renderer().render(&scene, 40, 40, 1.).unwrap();
    let edge = pixel(&px, 40, 20, 20);
    assert!(edge[0] > 40 && edge[0] < 215, "bord flouté : {edge:?}");
    assert_eq!(
        pixel(&px, 40, 5, 20),
        [255, 255, 255, 255],
        "hors de la forme intact"
    );
    assert_eq!(
        pixel(&px, 40, 35, 20),
        [0, 0, 0, 255],
        "hors de la forme intact"
    );
}
fn fade() -> Arc<LinearGradient> {
    Arc::new(LinearGradient::new(
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
    ))
}
#[test]
fn masks_fade_content_and_text_fill_paints_glyphs_only() {
    let mut scene = clear();
    scene.commands.push(DrawCommand::PushMask {
        bounds: RectF::new(0., 0., 100., 10.),
        gradient: fade(),
    });
    scene.commands.push(fill(
        RectF::new(0., 0., 100., 10.),
        Color::rgb(255, 255, 255),
    ));
    scene.commands.push(DrawCommand::PopMask);
    scene.commands.push(DrawCommand::PushTextFill {
        rect: RectF::new(0., 20., 100., 30.),
        fill: Background::Linear(fade()),
    });
    scene.font_text(
        RectF::new(0., 20., 100., 30.),
        "MMMM",
        24.,
        crate::ui::font::Font::system(true),
        Color::rgba(0, 0, 0, 0),
    );
    scene.commands.push(DrawCommand::PopTextFill);
    let px = renderer().render(&scene, 100, 50, 1.).unwrap();
    let (left, right) = (pixel(&px, 100, 3, 5)[3], pixel(&px, 100, 96, 5)[3]);
    assert!(left > 230 && right < 25, "{left} {right}");
    let glyphs = (20..50)
        .flat_map(|y| (0..100).map(move |x| (x, y)))
        .filter(|(x, y)| pixel(&px, 100, *x, *y)[3] > 128)
        .count();
    assert!(glyphs > 50, "les glyphes sont peints par le dégradé");
    assert!(glyphs < 100 * 30 / 2, "seuls les glyphes sont peints");
}
