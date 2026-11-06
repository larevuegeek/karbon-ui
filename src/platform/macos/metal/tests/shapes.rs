//! Formes, coins, traits, dégradés, segments, polygones, images, icônes et SVG.
use super::close;
use crate::ui::{
    rendering::{
        brush::{GradientStop, LinearGradient, RadialGradient},
        svg::{FillRule, LineCap, LineJoin, PathOp, SvgImage, SvgPaint, SvgShape},
        vector::{VectorContour, VectorIcon, VectorSegment},
    },
    scene::{DrawCommand, Image, PointF, RectF, Scene},
    theme::Color,
};
use std::sync::Arc;

fn stops(a: Color, b: Color) -> [GradientStop; 2] {
    [
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
fn p(x: f32, y: f32) -> PointF {
    PointF { x, y }
}
#[test]
fn fills_corners_and_strokes_match_core_graphics() {
    let mut scene = Scene::new(Color::rgb(240, 240, 235));
    let c = &mut scene.commands;
    c.push(DrawCommand::Fill {
        rect: RectF::new(4., 4., 40., 24.),
        radius: 0.,
        color: Color::rgb(200, 30, 30),
    });
    c.push(DrawCommand::Fill {
        rect: RectF::new(50.25, 4.5, 40.5, 24.),
        radius: 8.,
        color: Color::rgba(30, 30, 200, 180),
    });
    c.push(DrawCommand::Stroke {
        rect: RectF::new(100., 4., 40., 24.),
        radius: 0.,
        width: 1.,
        color: Color::rgb(0, 0, 0),
    });
    c.push(DrawCommand::Stroke {
        rect: RectF::new(150.5, 4.5, 40., 24.),
        radius: 6.,
        width: 3.,
        color: Color::rgba(0, 120, 0, 200),
    });
    c.push(DrawCommand::Shape {
        rect: RectF::new(4., 36., 60., 40.),
        radii: [20., 0., 10., 4.],
        fill: Some(Color::rgb(20, 160, 160).into()),
        stroke: None,
    });
    c.push(DrawCommand::Shape {
        rect: RectF::new(70., 36., 60., 40.),
        radii: [20., 0., 10., 4.],
        fill: None,
        stroke: Some((2., Color::rgb(90, 0, 90))),
    });
    c.push(DrawCommand::Shape {
        rect: RectF::new(136., 36., 30., 30.),
        radii: [15.; 4],
        fill: None,
        stroke: Some((0.5, Color::rgb(0, 0, 0))),
    });
    c.push(DrawCommand::Fill {
        rect: RectF::new(172., 36., 0.5, 30.),
        radius: 0.,
        color: Color::rgb(0, 0, 0),
    });
    close("formes", &scene, (200., 80.), 0.6, 0.01);
}
#[test]
fn gradients_match_core_graphics() {
    let mut scene = Scene::new(Color::rgb(255, 255, 255));
    let c = &mut scene.commands;
    c.push(DrawCommand::Gradient {
        rect: RectF::new(4., 4., 120., 30.),
        radius: 6.,
        gradient: Arc::new(LinearGradient::new(
            0.,
            stops(Color::rgb(255, 0, 0), Color::rgb(0, 0, 255)),
        )),
    });
    c.push(DrawCommand::Gradient {
        rect: RectF::new(130., 4., 60., 60.),
        radius: 0.,
        gradient: Arc::new(LinearGradient::new(
            45.,
            [
                GradientStop {
                    position: 0.,
                    color: Color::rgba(0, 0, 0, 0),
                },
                GradientStop {
                    position: 0.5,
                    color: Color::rgb(250, 200, 0),
                },
                GradientStop {
                    position: 1.,
                    color: Color::rgb(0, 128, 0),
                },
            ],
        )),
    });
    c.push(DrawCommand::RadialGradient {
        rect: RectF::new(4., 40., 120., 50.),
        radius: 10.,
        gradient: Arc::new(RadialGradient::new(
            [0.5, 0.5],
            [0.5, 0.5],
            stops(Color::rgb(255, 255, 255), Color::rgb(20, 20, 80)),
        )),
    });
    c.push(DrawCommand::Shape {
        rect: RectF::new(130., 70., 60., 24.),
        radii: [12., 12., 0., 0.],
        fill: Some(crate::ui::rendering::brush::Background::Linear(Arc::new(
            LinearGradient::new(90., stops(Color::rgb(0, 0, 0), Color::rgb(255, 255, 255))),
        ))),
        stroke: None,
    });
    close("degrades", &scene, (200., 100.), 1.0, 0.01);
}
#[test]
fn lines_paths_icons_and_svg_match_core_graphics() {
    let mut scene = Scene::new(Color::rgb(250, 250, 250));
    let c = &mut scene.commands;
    c.push(DrawCommand::Line {
        start: p(4., 10.),
        end: p(80., 10.),
        width: 1.,
        color: Color::rgb(0, 0, 0),
    });
    c.push(DrawCommand::Line {
        start: p(4., 20.),
        end: p(80., 44.),
        width: 2.,
        color: Color::rgb(200, 0, 0),
    });
    c.push(DrawCommand::Path {
        points: vec![p(90., 4.), p(140., 4.), p(115., 44.)],
        closed: true,
        color: Color::rgb(0, 100, 200),
    });
    c.push(DrawCommand::Path {
        points: vec![p(150., 40.), p(165., 10.), p(180., 40.)],
        closed: false,
        color: Color::rgb(0, 0, 0),
    });
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
    c.push(DrawCommand::Vector {
        rect: RectF::new(4.5, 50.5, 30., 30.),
        icon: Arc::new(icon),
        color: Color::rgb(120, 0, 160),
        outline: 0.,
    });
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
    c.push(DrawCommand::Svg {
        rect: RectF::new(50., 50., 36., 36.),
        image: &SVG,
        color: Color::rgb(250, 120, 0),
    });
    close("traits", &scene, (200., 90.), 0.5, 0.01);
}
#[test]
fn images_match_core_graphics_at_their_size_and_scaled() {
    // Dégradé doux 32 × 32 : le rééchantillonnage diffère peu d'un filtre à l'autre.
    let mut pixels = Vec::new();
    for y in 0..32u32 {
        for x in 0..32u32 {
            pixels.extend([(x * 8) as u8, (y * 8) as u8, 128, 255]);
        }
    }
    let image = Arc::new(Image::new(32, 32, pixels).unwrap());
    let mut scene = Scene::new(Color::rgb(0, 0, 0));
    scene.commands.push(DrawCommand::Image {
        rect: RectF::new(4., 4., 32., 32.),
        image: Arc::clone(&image),
    });
    scene.commands.push(DrawCommand::Image {
        rect: RectF::new(40., 4., 64., 48.),
        image: Arc::clone(&image),
    });
    scene.commands.push(DrawCommand::Image {
        rect: RectF::new(110., 4., 16., 16.),
        image,
    });
    close("images", &scene, (130., 56.), 1.5, 0.02);
}
