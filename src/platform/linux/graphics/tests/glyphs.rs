//! Services prêtés au renderer Vulkan (`graphics/gpu.rs`) : un glyphe rastérisé seul à la
//! position et à la phase que donnent `device_position` et `quarter` reproduit exactement
//! le dessin Cairo du même glyphe, quelle que soit sa position fractionnaire, à 1x, 1,5x
//! et 2x ; de même pour un émoji en couleur (pixel le plus proche, sans phase).
use super::super::gpu::{GlyphImage, device_position, quarter, rasterize_glyph, show_glyph};
use super::renderer;
use crate::{
    platform::{
        linux::{bitmap::Bitmap, ffi::cairo::Matrix},
        shared::{plain, simple_format},
    },
    ui::{font::Font, scene::RectF, theme::Color},
};

const SIDE: usize = 64;

/// Pixels d'un bitmap de `SIDE` × `SIDE` (`bytes` octets par pixel) où `image` est posée
/// à (`x`, `y`).
fn placed(image: &GlyphImage, (x, y): (i32, i32), stride: usize, bytes: usize) -> Vec<u8> {
    let mut pixels = vec![0; SIDE * stride];
    for row in 0..image.height {
        for col in 0..image.width {
            let (px, py) = (x + image.left + col as i32, y + image.top + row as i32);
            if (0..SIDE as i32).contains(&px) && (0..SIDE as i32).contains(&py) {
                let to = py as usize * stride + px as usize * bytes;
                let from = row * image.stride + col * bytes;
                pixels[to..to + bytes].copy_from_slice(&image.pixels[from..from + bytes]);
            }
        }
    }
    pixels
}
/// Dessine `text` (un glyphe) par Cairo puis par `rasterize_glyph`, à 16 positions
/// fractionnaires sur chaque axe et à 1x, 1,5x et 2x : mêmes pixels.
fn check(text: &str, size: f32, colored: bool) {
    if !crate::platform::linux::ffi::cairo::subpixel_glyphs() {
        eprintln!("Cairo sans positionnement sous-pixel des glyphes (< 1.17.4) : test sauté");
        return;
    }
    let mut cairo = renderer();
    let paragraph = plain(&text.into(), simple_format(Font::system(false), size));
    let laid = cairo
        .text_glyphs(
            &paragraph,
            RectF::new(0., 0., 100., 30.),
            Color::rgb(255, 255, 255),
            false,
        )
        .unwrap();
    let run = &laid.runs[0];
    assert_eq!(run.colored, colored, "{text}");
    let (font, glyph) = (&run.font, run.glyphs[0]);
    for scale in [1f32, 1.5, 2.] {
        let s = f64::from(scale);
        for k in 0..16 {
            let d = f64::from(k) / 16.;
            for (x, y) in [(10. + d, 40.), (10., 40. + d)] {
                let reference = if colored {
                    Bitmap::new(SIDE, SIDE)
                } else {
                    Bitmap::alpha(SIDE, SIDE)
                }
                .unwrap();
                {
                    let context = reference.context().unwrap();
                    context.transform(Matrix::scaling(s, s));
                    context.set_source_rgba(1., 1., 1., 1.);
                    context.move_to(x / s, y / s);
                    show_glyph(&context, font, glyph);
                }
                let at = |v: f64| quarter(device_position(v / s, 0., s, 0.), colored);
                let ((px, fx), (py, fy)) = (at(x), at(y));
                let image = rasterize_glyph(
                    font,
                    glyph,
                    scale,
                    (f32::from(fx) / 4., f32::from(fy) / 4.),
                    colored,
                )
                .unwrap()
                .unwrap();
                let bytes = if colored { 4 } else { 1 };
                assert!(
                    placed(&image, (px, py), reference.row_bytes(), bytes) == reference.pixels(),
                    "{text} : échelle {scale}, position ({x}, {y})"
                );
            }
        }
    }
}
#[test]
fn rasterized_glyphs_match_cairo_at_every_subpixel_position() {
    check("g", 14., false);
}
#[test]
fn color_glyphs_are_placed_on_the_nearest_pixel_like_cairo() {
    let mut cairo = renderer();
    let emoji = plain(&"🎧".into(), simple_format(Font::system(false), 18.));
    let laid = cairo
        .text_glyphs(
            &emoji,
            RectF::new(0., 0., 100., 30.),
            Color::rgb(0, 0, 0),
            false,
        )
        .unwrap();
    // Sans police d'émojis en couleur installée, rien à vérifier.
    if laid.runs.first().is_some_and(|r| r.colored) {
        check("🎧", 18., true);
    }
}
#[test]
fn quarter_rounds_like_cairo() {
    assert_eq!(quarter(20., false), (20, 0));
    assert_eq!(quarter(20.1, false), (20, 0));
    assert_eq!(quarter(20.125, false), (20, 1));
    assert_eq!(quarter(20.6, false), (20, 2));
    assert_eq!(quarter(20.875, false), (21, 0));
    assert_eq!(quarter(-0.2, false), (-1, 3));
    assert_eq!(quarter(20.49, true), (20, 0));
    assert_eq!(quarter(20.5, true), (21, 0));
}
