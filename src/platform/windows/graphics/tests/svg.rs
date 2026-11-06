//! Dessins SVG compilés : remplissages, traits, bouts et règles de remplissage.
use super::super::*;
use crate::ui::rendering::svg::*;

static SHAPES: [SvgShape; 3] = [
    // Carré plein à trou (pair-impair), couleur courante.
    SvgShape {
        ops: &[
            PathOp::MoveTo(0., 0.),
            PathOp::LineTo(10., 0.),
            PathOp::LineTo(10., 10.),
            PathOp::LineTo(0., 10.),
            PathOp::Close,
            PathOp::MoveTo(3., 3.),
            PathOp::LineTo(7., 3.),
            PathOp::LineTo(7., 7.),
            PathOp::LineTo(3., 7.),
            PathOp::Close,
        ],
        fill: SvgPaint::CurrentColor,
        stroke: SvgPaint::None,
        stroke_width: 1.,
        line_cap: LineCap::Butt,
        line_join: LineJoin::Miter,
        miter_limit: 4.,
        fill_rule: FillRule::EvenOdd,
        fill_opacity: 1.,
        stroke_opacity: 1.,
    },
    // Ligne ouverte à bouts ronds (le remplissage d'une figure ouverte reste vide).
    SvgShape {
        ops: &[PathOp::MoveTo(12., 5.), PathOp::LineTo(18., 5.)],
        fill: SvgPaint::None,
        stroke: SvgPaint::Color(Color::rgb(0, 255, 0)),
        stroke_width: 2.,
        line_cap: LineCap::Round,
        line_join: LineJoin::Round,
        miter_limit: 4.,
        fill_rule: FillRule::NonZero,
        fill_opacity: 1.,
        stroke_opacity: 1.,
    },
    // Courbe fermée tracée seulement.
    SvgShape {
        ops: &[
            PathOp::MoveTo(12., 12.),
            PathOp::CubicTo(12., 18., 18., 18., 18., 12.),
            PathOp::Close,
        ],
        fill: SvgPaint::None,
        stroke: SvgPaint::Color(Color::rgb(0, 0, 255)),
        stroke_width: 1.,
        line_cap: LineCap::Butt,
        line_join: LineJoin::Bevel,
        miter_limit: 4.,
        fill_rule: FillRule::NonZero,
        fill_opacity: 1.,
        stroke_opacity: 1.,
    },
];
static IMAGE: SvgImage = SvgImage {
    view_box: [0., 0., 20., 20.],
    preserve_aspect_ratio: true,
    shapes: &SHAPES,
};
pub(super) fn verify_svg(mode: RenderMode) {
    let mut renderer = Renderer::new(mode).unwrap();
    let mut scene = Scene::new(Color::rgba(0, 0, 0, 0));
    // viewBox 20 × 20 dans 40 × 40 : facteur 2.
    scene.commands.push(DrawCommand::Svg {
        rect: RectF::new(0., 0., 40., 40.),
        image: &IMAGE,
        color: Color::rgb(255, 0, 0),
    });
    for scale in [1., 1.5] {
        let w = (40. * scale) as u32;
        let pixels = renderer.render(&scene, w, w, scale).unwrap();
        let pixel = |x: f32, y: f32| {
            let i = ((y * scale) as usize * w as usize + (x * scale) as usize) * 4;
            [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]
        };
        // Carré rouge (currentColor), trou transparent.
        assert!(pixel(3., 3.)[2] > 240, "{:?}", pixel(3., 3.));
        assert!(pixel(10., 10.)[3] < 20, "{:?}", pixel(10., 10.));
        // Trait vert de 4 px (2 × 2) centré sur y = 10, bouts ronds au-delà de x = 36.
        assert!(pixel(30., 10.)[1] > 240);
        assert!(pixel(30., 14.)[3] < 20);
        assert!(pixel(37., 10.)[1] > 120, "{:?}", pixel(37., 10.));
        // Contour bleu de la courbe, intérieur vide.
        assert!(pixel(24., 24.5)[0] > 200 || pixel(24., 23.5)[0] > 200);
        assert!(pixel(30., 28.)[3] < 20);
        assert_eq!(renderer.target.as_ref().unwrap().svgs.len(), 3);
    }
    scene.commands.clear();
    renderer.render(&scene, 40, 40, 1.).unwrap();
    assert_eq!(renderer.target.as_ref().unwrap().svgs.len(), 0);
}
#[test]
fn software_svg_fills_strokes_caps_and_fill_rules() {
    verify_svg(RenderMode::Software);
}
