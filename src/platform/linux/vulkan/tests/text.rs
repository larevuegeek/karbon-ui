//! Texte traduit par `encode/text.rs` et rendu par Vulkan, comparé au rendu Cairo à 1x,
//! 1,5x et 2x : tailles, graisses, couleurs par portion, décorations, retours à la ligne,
//! découpe, positions fractionnaires ; puis sous rotation et opacité de groupe (glyphes
//! transformés par le shader), et émojis en couleur.
use super::compare;
use crate::ui::{
    font::Font,
    rendering::transform::Affine,
    scene::{DrawCommand, RectF, Scene},
    styling::text::{RichParagraph, SpanStyle, TextFormat, TextSpan},
    theme::Color,
};
use std::sync::Arc;

fn text(rect: RectF, text: &str, size: f32, bold: bool, color: Color) -> DrawCommand {
    DrawCommand::Text {
        rect,
        text: Arc::from(text),
        size,
        bold,
        color,
        wrap: false,
    }
}
#[test]
fn text_matches_cairo_glyph_for_glyph() {
    let mut scene = Scene::new(Color::rgb(24, 26, 34));
    let white = Color::rgb(240, 240, 240);
    let base = TextFormat::new(Default::default(), Font::system(false), 15.);
    let rich = RichParagraph {
        text: Arc::from("Lecture en cours : Nocturne n° 2, opus 9"),
        format: base,
        spans: vec![
            TextSpan {
                range: 0..7,
                style: SpanStyle::new().bold().color(Color::rgb(255, 120, 80)),
            },
            TextSpan {
                range: 19..27,
                style: SpanStyle::new().underline().size(19.),
            },
            TextSpan {
                range: 29..36,
                style: SpanStyle::new().strike().color(Color::rgb(120, 200, 255)),
            },
        ],
    };
    let mut wrapped = base;
    wrapped.clip = true;
    scene.commands.extend([
        text(
            RectF::new(8., 6., 300., 20.),
            "Bibliothèque musicale",
            13.,
            false,
            white,
        ),
        text(
            RectF::new(8.3, 26.7, 300., 24.),
            "Albums récents — 2026",
            16.,
            true,
            white,
        ),
        DrawCommand::FontText {
            rect: RectF::new(8., 50., 300., 30.),
            text: Arc::from("Grand titre 22 px"),
            size: 22.,
            font: Font::system(false),
            color: Color::rgba(250, 210, 90, 230),
            wrap: false,
        },
        DrawCommand::RichText {
            rect: RectF::new(8., 82., 340., 30.),
            paragraph: Arc::new(rich),
            color: white,
            wrap: false,
        },
        DrawCommand::FormattedText {
            rect: RectF::new(8., 112., 150., 34.),
            text: Arc::from("Un paragraphe assez long pour passer à la ligne puis être coupé"),
            format: wrapped,
            color: Color::rgb(200, 200, 210),
            wrap: true,
        },
        // Translation fractionnaire d'un effet : phases au quart de pixel.
        DrawCommand::PushEffect {
            transform: Affine::translation(0.37, 0.61),
            opacity: 1.,
            bounds: RectF::new(170., 112., 180., 30.),
        },
        text(
            RectF::new(170., 112., 180., 20.),
            "Décalé de 0,37 px",
            12.,
            false,
            white,
        ),
        DrawCommand::PopEffect,
    ]);
    compare("texte", &scene, 360., 150., 0.05, 0.0002);
}
#[test]
fn rotated_text_stays_close_to_cairo() {
    // Cairo rastérise le contour tourné ; le GPU tourne le glyphe droit (bords seuls).
    let mut scene = Scene::new(Color::rgb(245, 245, 248));
    let (s, c) = 0.2f32.sin_cos();
    let turn = Affine {
        m11: c,
        m12: s,
        m21: -s,
        m22: c,
        ..Affine::IDENTITY
    };
    scene.commands.extend([
        DrawCommand::PushEffect {
            transform: Affine::translation(-90., -30.)
                .then(turn)
                .then(Affine::translation(90., 30.)),
            opacity: 1.,
            bounds: RectF::new(10., 15., 160., 30.),
        },
        text(
            RectF::new(10., 15., 160., 30.),
            "Texte tourné",
            18.,
            false,
            Color::rgb(30, 30, 40),
        ),
        DrawCommand::PopEffect,
    ]);
    compare("texte-tourne", &scene, 180., 60., 1.5, 0.04);
}
#[test]
fn text_in_a_translucent_group_matches_cairo() {
    let mut scene = Scene::new(Color::rgb(245, 245, 248));
    scene.commands.extend([
        DrawCommand::PushEffect {
            transform: Affine::IDENTITY,
            opacity: 0.5,
            bounds: RectF::new(10., 10., 160., 50.),
        },
        text(
            RectF::new(10., 10., 160., 20.),
            "Groupe à demi",
            15.,
            true,
            Color::rgb(30, 30, 40),
        ),
        DrawCommand::Fill {
            rect: RectF::new(10., 34., 120., 16.),
            radius: 4.,
            color: Color::rgb(220, 60, 70),
        },
        DrawCommand::PopEffect,
    ]);
    compare("texte-groupe", &scene, 180., 60., 0.1, 0.001);
}
#[test]
fn color_emoji_match_cairo() {
    let mut scene = Scene::new(Color::rgb(255, 255, 255));
    scene.commands.push(text(
        RectF::new(6., 4., 200., 30.),
        "Écoute 🎵🎧 ok",
        18.,
        false,
        Color::rgb(0, 0, 0),
    ));
    compare("emojis", &scene, 210., 36., 0.5, 0.01);
}
