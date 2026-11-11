//! Images et rastérisations (`encode/raster.rs`) rendues par Vulkan, comparées au rendu
//! Cairo à 1x, 1,5x et 2x : image à sa taille, réduite, agrandie, à une position
//! fractionnaire, à demi opaque et tournée ; polygones, polyligne, icône vectorielle et
//! SVG, droits puis tournés ; textures réutilisées d'une image à l'autre.
use super::{compare, renderer};
use crate::ui::{
    rendering::{
        svg::{FillRule, LineCap, LineJoin, PathOp, SvgImage, SvgPaint, SvgShape},
        transform::Affine,
        vector::{VectorContour, VectorIcon, VectorSegment},
    },
    scene::{DrawCommand, Image, PointF, RectF, Scene},
    theme::Color,
};
use std::sync::Arc;

fn p(x: f32, y: f32) -> PointF {
    PointF { x, y }
}
/// Image `side` × `side` : dégradé et damier (contours nets, sensibles au placement).
fn picture(side: u32) -> Arc<Image> {
    let mut pixels = Vec::new();
    for y in 0..side {
        for x in 0..side {
            let check = if (x / 4 + y / 4) % 2 == 0 { 255 } else { 60 };
            let alpha = if x < side / 8 { 128 } else { 255 };
            let premultiply = |v: u32| (v * alpha / 255) as u8;
            pixels.extend([
                premultiply(x * 255 / side),
                premultiply(y * 255 / side),
                premultiply(check),
                alpha as u8,
            ]);
        }
    }
    Arc::new(Image::new(side, side, pixels).unwrap())
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
fn images_match_cairo_at_their_size_reduced_enlarged_and_translucent() {
    let image = picture(64);
    let mut scene = Scene::new(Color::rgb(20, 20, 24));
    let draw = |rect: RectF| DrawCommand::Image {
        rect,
        image: Arc::clone(&image),
    };
    scene.commands.extend([
        draw(RectF::new(4., 4., 64., 64.)),
        draw(RectF::new(72.3, 4.6, 40., 40.)),
        draw(RectF::new(118., 4., 96., 72.)),
        DrawCommand::PushEffect {
            transform: Affine::IDENTITY,
            opacity: 0.5,
            bounds: RectF::new(4., 72., 32., 32.),
        },
        draw(RectF::new(4., 72., 32., 32.)),
        DrawCommand::PopEffect,
    ]);
    compare("images", &scene, 220., 108., 0.05, 0.0005);
}
#[test]
fn rotated_images_stay_close_to_cairo() {
    // Cairo filtre l'image tournée à sa façon (pixman) ; le GPU par mipmaps.
    let image = picture(64);
    let mut scene = Scene::new(Color::rgb(20, 20, 24));
    scene.commands.extend([
        DrawCommand::PushEffect {
            transform: rotation(0.35, p(50., 50.)),
            opacity: 1.,
            bounds: RectF::new(18., 18., 64., 64.),
        },
        DrawCommand::Image {
            rect: RectF::new(18., 18., 64., 64.),
            image,
        },
        DrawCommand::PopEffect,
    ]);
    compare("images-tournees", &scene, 100., 100., 1., 0.012);
}
#[test]
fn images_animated_in_size_are_stretched_like_cairo() {
    // Taille changée d'une image à l'autre : Cairo étire l'image d'origine (filtre
    // `GOOD`), le GPU l'échantillonne avec ses mipmaps.
    let Some(mut vulkan) = renderer() else {
        return;
    };
    let image = picture(64);
    let scene = |side: f32| {
        let mut scene = Scene::new(Color::rgb(20, 20, 24));
        scene.commands.push(DrawCommand::Image {
            rect: RectF::new(4., 4., side, side),
            image: Arc::clone(&image),
        });
        scene
    };
    let mut cairo = super::super::super::graphics::Renderer::new().unwrap();
    for side in [40., 37., 30., 52.] {
        let frame = scene(side);
        let expected = cairo.render(&frame, 60, 60, 1.).unwrap();
        let gpu = vulkan.render(&frame, 60, 60, 1.).unwrap();
        let (m, f) = super::difference(&expected, &gpu);
        eprintln!(
            "côté {side} : écart moyen {m:.3}, au-delà de 16 : {:.4} %",
            f * 100.
        );
        assert!(
            m <= 2. && f <= 0.03,
            "côté {side} : {m:.3}, {:.4} %",
            f * 100.
        );
    }
}
/// Polygone, polyligne, icône et SVG (modèle : la suite Metal).
fn vectors() -> Vec<DrawCommand> {
    let icon = VectorIcon {
        width: 10.,
        height: 10.,
        contours: vec![VectorContour {
            start: p(0., 0.),
            segments: vec![
                VectorSegment::Line(p(10., 5.)),
                VectorSegment::Curve {
                    control1: p(5., 10.),
                    control2: p(2., 10.),
                    end: p(0., 10.),
                },
            ],
        }],
    };
    static SHAPES: [SvgShape; 1] = [SvgShape {
        ops: &[
            PathOp::MoveTo(2., 2.),
            PathOp::LineTo(22., 2.),
            PathOp::CubicTo(22., 12., 12., 22., 2., 22.),
            PathOp::Close,
        ],
        fill: SvgPaint::CurrentColor,
        stroke: SvgPaint::Color(Color::rgb(0, 0, 0)),
        stroke_width: 1.5,
        line_cap: LineCap::Round,
        line_join: LineJoin::Round,
        miter_limit: 4.,
        fill_rule: FillRule::NonZero,
        fill_opacity: 1.,
        stroke_opacity: 1.,
    }];
    static SVG: SvgImage = SvgImage {
        view_box: [0., 0., 24., 24.],
        preserve_aspect_ratio: true,
        shapes: &SHAPES,
    };
    vec![
        DrawCommand::Path {
            points: vec![p(90., 4.), p(140., 4.), p(115., 44.)],
            closed: true,
            color: Color::rgb(0, 100, 200),
        },
        DrawCommand::Path {
            points: vec![p(150., 40.), p(165., 10.), p(180., 40.)],
            closed: false,
            color: Color::rgb(0, 0, 0),
        },
        DrawCommand::Vector {
            rect: RectF::new(4.5, 50.5, 30., 30.),
            icon: Arc::new(icon),
            color: Color::rgb(120, 0, 160),
            outline: 0.,
        },
        DrawCommand::Svg {
            rect: RectF::new(50., 50., 36., 36.),
            image: &SVG,
            color: Color::rgb(250, 120, 0),
        },
    ]
}
#[test]
fn paths_icons_and_svg_match_cairo() {
    let mut scene = Scene::new(Color::rgb(250, 250, 250));
    scene.commands.extend(vectors());
    compare("traces", &scene, 200., 90., 0.05, 0.0005);
}
#[test]
fn rotated_paths_icons_and_svg_stay_close_to_cairo() {
    let mut scene = Scene::new(Color::rgb(250, 250, 250));
    scene.commands.push(DrawCommand::PushEffect {
        transform: rotation(0.25, p(100., 45.)),
        opacity: 1.,
        bounds: RectF::new(0., 0., 200., 90.),
    });
    scene.commands.extend(vectors());
    scene.commands.push(DrawCommand::PopEffect);
    compare("traces-tournes", &scene, 200., 90., 1.2, 0.03);
}
#[test]
fn textures_are_kept_between_frames_and_released_when_unused_or_trimmed() {
    let Some(mut vulkan) = renderer() else {
        return;
    };
    let mut scene = Scene::new(Color::rgb(0, 0, 0));
    scene.commands.push(DrawCommand::Image {
        rect: RectF::new(0., 0., 32., 32.),
        image: picture(64),
    });
    scene.commands.extend(vectors());
    for _ in 0..3 {
        vulkan.render(&scene, 200, 90, 1.).unwrap();
        assert_eq!(vulkan.resources.textures.counts(), [1, 4]);
    }
    // Repos : les textures de la dernière image restent ; fenêtre cachée : rien.
    vulkan.trim(super::super::Trim::Idle);
    assert_eq!(vulkan.resources.textures.counts(), [1, 4]);
    vulkan.render(&scene, 200, 90, 1.).unwrap();
    vulkan.trim(super::super::Trim::Hidden);
    assert_eq!(vulkan.resources.textures.counts(), [0, 0]);
    let again = vulkan.render(&scene, 200, 90, 1.).unwrap();
    assert_eq!(vulkan.resources.textures.counts(), [1, 4]);
    let empty = Scene::new(Color::rgb(0, 0, 0));
    for _ in 0..61 {
        vulkan.render(&empty, 200, 90, 1.).unwrap();
    }
    assert_eq!(vulkan.resources.textures.counts(), [0, 0]);
    assert!(!again.is_empty());
}
#[test]
fn a_fractional_scroll_reuses_the_rasterizations() {
    let Some(mut vulkan) = renderer() else {
        return;
    };
    // Vingt positions d'un défilement fractionnaire sur un cinquième de pixel : quelques
    // rastérisations par contenu (quarts touchés, pixel entier franchi) au lieu d'une par
    // position (80 avant la quantification).
    for i in 0..20 {
        let mut scene = Scene::new(Color::rgb(0, 0, 0));
        scene.commands.push(DrawCommand::PushEffect {
            transform: Affine::translation(0.3 + i as f32 * 0.01, 0.),
            opacity: 1.,
            bounds: RectF::new(0., 0., 200., 90.),
        });
        scene.commands.extend(vectors());
        scene.commands.push(DrawCommand::PopEffect);
        vulkan.render(&scene, 200, 90, 1.).unwrap();
    }
    assert!(vulkan.resources.textures.counts()[1] <= 16);
}
