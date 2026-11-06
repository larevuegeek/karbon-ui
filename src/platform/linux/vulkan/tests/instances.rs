//! Instances dessinées hors écran puis relues : formes et peintures du shader (boîte,
//! coins arrondis, anneau, segment, dégradé, découpe, opacité) vérifiées au pixel, et
//! boîtes arrondies comparées au rendu Cairo des mêmes commandes.
use super::{pixel, renderer};
use crate::{
    platform::shared::{
        gpu::{self, Instance, PAINT_LINEAR, SHAPE_RING, SHAPE_SEGMENT, Stop},
        normalized,
    },
    ui::{
        rendering::{brush::Background, transform::Affine},
        scene::{DrawCommand, RectF, Scene},
        theme::Color,
    },
};

fn scale(s: f32) -> Affine {
    Affine {
        m11: s,
        m22: s,
        ..Affine::IDENTITY
    }
}
fn solid(rect: RectF, radii: [f32; 4], m: Affine, c: Color) -> Instance {
    let mut i = Instance::rounded(rect, radii, m);
    i.color = gpu::color(c);
    i
}
#[test]
fn shapes_paints_clips_and_opacity_land_on_the_expected_pixels() {
    let Some(mut r) = renderer() else {
        return;
    };
    let one = Affine::IDENTITY;
    let mut box_ = solid(
        RectF::new(10., 10., 40., 40.),
        [0.; 4],
        one,
        Color::rgb(255, 0, 0),
    );
    // Découpe rectangulaire : la moitié droite de la boîte seulement.
    box_.clip = [30., 0., 1000., 1000.];
    let round = solid(
        RectF::new(60., 10., 40., 40.),
        [20.; 4],
        one,
        Color::rgb(0, 255, 0),
    );
    let mut ring = solid(
        RectF::new(110., 10., 40., 40.),
        [0.; 4],
        one,
        Color::rgb(0, 0, 255),
    );
    ring.kind[0] = SHAPE_RING;
    ring.inner = gpu::rect(RectF::new(114., 14., 32., 32.));
    let mut segment = Instance::new(
        RectF::new(0., 58., 160., 8.),
        one,
        SHAPE_SEGMENT,
        gpu::PAINT_SOLID,
    );
    segment.shape = [0., 62., 160., 62.];
    segment.radii = [1., 0., 0., 0.];
    segment.color = gpu::color(Color::rgb(255, 255, 255));
    let mut faded = solid(
        RectF::new(10., 70., 40., 20.),
        [0.; 4],
        one,
        Color::rgb(255, 255, 255),
    );
    faded.t[3] = 0.5;
    let mut gradient = solid(
        RectF::new(60., 70., 100., 20.),
        [0.; 4],
        one,
        Color::rgb(0, 0, 0),
    );
    gradient.kind[1] = PAINT_LINEAR;
    gradient.kind[2] = 0;
    gradient.kind[3] = 2;
    gradient.paint = [60., 0., 160., 0.];
    let stops = [
        Stop {
            color: [0., 0., 0., 1.],
            position: [0.; 4],
        },
        Stop {
            color: [1., 1., 1., 1.],
            position: [1., 0., 0., 0.],
        },
    ];
    let instances = [box_, round, ring, segment, faded, gradient];
    let (w, h) = (170, 100);
    let px = r
        .render_instances(w, h, [0.; 4], &instances, &stops, &[])
        .unwrap();
    // BGRA prémultiplié.
    assert_eq!(pixel(&px, w, 35, 30), [0, 0, 255, 255], "boîte");
    assert_eq!(pixel(&px, w, 20, 30)[3], 0, "coupée par la découpe");
    assert_eq!(pixel(&px, w, 80, 30), [0, 255, 0, 255], "centre arrondi");
    assert_eq!(pixel(&px, w, 61, 11)[3], 0, "coin arrondi transparent");
    assert_eq!(pixel(&px, w, 111, 30), [255, 0, 0, 255], "anneau");
    assert_eq!(pixel(&px, w, 130, 30)[3], 0, "intérieur de l'anneau");
    assert_eq!(pixel(&px, w, 80, 62), [255, 255, 255, 255], "segment");
    assert_eq!(pixel(&px, w, 80, 64)[3], 0, "hors du segment");
    let half = pixel(&px, w, 30, 80);
    assert!(
        half.iter().all(|c| c.abs_diff(128) <= 1),
        "opacité {half:?}"
    );
    let (left, right) = (pixel(&px, w, 62, 80), pixel(&px, w, 158, 80));
    assert!(
        left[0] < 10 && right[0] > 245 && left[3] == 255,
        "{left:?} {right:?}"
    );
}
#[test]
fn rounded_boxes_match_the_cairo_rendering_of_the_same_shapes() {
    let Some(mut r) = renderer() else {
        return;
    };
    let shapes = [
        (
            RectF::new(4., 4., 60., 30.),
            [12., 4., 0., 18.],
            Color::rgb(220, 40, 60),
        ),
        (
            RectF::new(70., 6., 25., 25.),
            [12.5; 4],
            Color::rgba(40, 200, 90, 200),
        ),
        (
            RectF::new(10.3, 40.6, 80.2, 20.5),
            [6.; 4],
            Color::rgb(250, 250, 250),
        ),
    ];
    for s in [1f32, 1.5, 2.] {
        let mut scene = Scene::new(Color::rgba(0, 0, 0, 0));
        let mut instances = Vec::new();
        for (rect, radii, color) in shapes {
            scene.commands.push(DrawCommand::Shape {
                rect,
                radii,
                fill: Some(Background::Solid(color)),
                stroke: None,
            });
            let (shape, radii) = normalized(rect, radii, None);
            instances.push(solid(shape, radii, scale(s), color));
        }
        let (w, h) = ((100. * s) as usize, (64. * s) as usize);
        let cairo = super::super::super::graphics::Renderer::new()
            .unwrap()
            .render(&scene, w as u32, h as u32, s)
            .unwrap();
        let vulkan = r
            .render_instances(w, h, [0.; 4], &instances, &[], &[])
            .unwrap();
        let total: u64 = cairo
            .iter()
            .zip(&vulkan)
            .map(|(a, b)| u64::from(a.abs_diff(*b)))
            .sum();
        let mean = total as f64 / cairo.len() as f64;
        let far = cairo
            .iter()
            .zip(&vulkan)
            .filter(|(a, b)| a.abs_diff(**b) > 16)
            .count();
        assert!(mean < 0.5, "échelle {s} : écart moyen {mean:.3}");
        assert!(
            far * 1000 < cairo.len(),
            "échelle {s} : {far} canaux écartés de plus de 16"
        );
    }
}
