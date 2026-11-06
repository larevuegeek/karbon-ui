//! Texte : polices du système et embarquées, couleurs par portion, décorations, coupe à
//! la boîte, texte sous transformation.
use super::close;
use crate::platform::macos::graphics::Renderer as CgRenderer;
use crate::ui::{
    font::{Font, FontAsset},
    rendering::transform::Affine,
    scene::{DrawCommand, RectF, Scene},
    styling::text::{RichParagraph, SpanStyle, TextFormat, TextSpan},
    theme::Color,
};
use std::sync::Arc;

fn scene_with_text() -> Scene {
    let mut scene = Scene::new(Color::rgb(255, 255, 255));
    let c = &mut scene.commands;
    c.push(DrawCommand::Text {
        rect: RectF::new(4., 4., 200., 20.),
        text: "Bonjour, monde ! éàç 0123".into(),
        size: 13.,
        bold: false,
        color: Color::rgb(20, 20, 20),
        wrap: false,
    });
    c.push(DrawCommand::FontText {
        rect: RectF::new(4.5, 26.25, 200., 24.),
        text: "Lecture en cours".into(),
        size: 18.,
        font: Font::new("Inter", 700),
        color: Color::rgb(160, 0, 60),
        wrap: false,
    });
    let text = "Voir la file d'attente complète maintenant, puis revenir à l'accueil";
    c.push(DrawCommand::RichText {
        rect: RectF::new(4., 54., 160., 60.),
        paragraph: Arc::new(RichParagraph {
            text: Arc::from(text),
            format: TextFormat::new(Default::default(), Font::new("Inter", 400), 14.),
            spans: vec![
                TextSpan {
                    range: 0..4,
                    style: SpanStyle::new().bold().color(Color::rgb(0, 90, 200)),
                },
                TextSpan {
                    range: 8..12,
                    style: SpanStyle::new().underline(),
                },
            ],
        }),
        color: Color::rgb(40, 40, 40),
        wrap: true,
    });
    let mut clipped = TextFormat::new(Default::default(), Font::new("Inter", 400), 14.);
    clipped.clip = true;
    c.push(DrawCommand::FormattedText {
        rect: RectF::new(170., 54., 30., 16.),
        text: "Texte coupé".into(),
        format: clipped,
        color: Color::rgb(0, 0, 0),
        wrap: false,
    });
    scene
}
fn renderer() -> CgRenderer {
    let mut cg = CgRenderer::new().unwrap();
    cg.load_fonts(&[FontAsset {
        family: "Inter",
        data: include_bytes!("../../../../../tests/fixtures/fonts/inter.woff2"),
    }])
    .unwrap();
    cg
}
/// Comme `close`, avec la police embarquée Inter.
fn close_with_fonts(name: &str, scene: &Scene, size: (f32, f32), mean: f64, outliers: f64) {
    let Some(mut metal) = super::metal() else {
        return;
    };
    let mut cg = renderer();
    for scale in [1f32, 2.] {
        let (m, c, w, h) = super::both(&mut metal, &mut cg, scene, size, scale);
        super::dump(name, scale, w, h, &m, &c);
        let d = super::difference(&m, &c);
        assert!(
            d.mean <= mean && d.outliers <= outliers,
            "{name} à {scale}x : {d:?}"
        );
    }
}
#[test]
fn text_matches_core_graphics() {
    close_with_fonts("texte", &scene_with_text(), (210., 116.), 0.6, 0.01);
}
#[test]
fn transformed_text_is_drawn_close_to_core_graphics() {
    let mut scene = Scene::new(Color::rgb(255, 255, 255));
    scene.commands.push(DrawCommand::PushEffect {
        transform: Affine {
            m11: 0.96,
            m12: 0.28,
            m21: -0.28,
            m22: 0.96,
            dx: 20.,
            dy: -4.,
        },
        opacity: 1.,
        bounds: RectF::new(0., 0., 200., 40.),
    });
    scene.commands.push(DrawCommand::Text {
        rect: RectF::new(10., 10., 180., 24.),
        text: "Texte tourné".into(),
        size: 16.,
        bold: true,
        color: Color::rgb(0, 0, 0),
        wrap: false,
    });
    scene.commands.push(DrawCommand::PopEffect);
    close("texte-tourne", &scene, (200., 80.), 2.5, 0.05);
}
