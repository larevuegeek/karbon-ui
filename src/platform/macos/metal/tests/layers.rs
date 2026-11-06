//! `backdrop-filter` (carte translucide floutée et arrondie sur un fond varié, dans un
//! groupe à opacité, ignoré dans un groupe filtré), masques en dégradé et texte rempli
//! (`background-clip: text`).
use super::{
    close,
    filters::{backdrop_content, stops},
};
use crate::ui::{
    element::ElementId,
    rendering::{
        brush::{Background, LinearGradient, RadialGradient},
        filter::{Filter, Filters},
        transform::Affine,
    },
    scene::{DrawCommand, RectF, Scene},
    styling::text::TextFormat,
    theme::Color,
};
use std::sync::Arc;

const SIZE: (f32, f32) = (200., 120.);

fn card(c: &mut Vec<DrawCommand>, rect: RectF, filters: &'static [Filter]) {
    c.push(DrawCommand::Backdrop {
        rect,
        radii: [14., 14., 6., 6.],
        filters: Filters::new(filters),
    });
    c.push(DrawCommand::Shape {
        rect,
        radii: [14., 14., 6., 6.],
        fill: Some(Background::Solid(Color::rgba(255, 255, 255, 70))),
        stroke: None,
    });
}
#[test]
fn backdrop_blur_behind_a_translucent_rounded_card_matches_core_graphics() {
    static GLASS: [Filter; 2] = [Filter::Blur(6.), Filter::Saturate(1.6)];
    static SMALL: [Filter; 1] = [Filter::Blur(2.)];
    let mut scene = Scene::new(Color::rgb(0, 0, 0));
    let c = &mut scene.commands;
    backdrop_content(c, SIZE.0, SIZE.1);
    card(c, RectF::new(30., 15., 110., 70.), &GLASS);
    // Seconde carte qui recouvre la première : elle relit la première déjà composée.
    card(c, RectF::new(120., 50., 70., 60.), &SMALL);
    close("arriere-plan", &scene, SIZE, 0.6, 0.003);
}
#[test]
fn backdrop_inside_an_opacity_group_and_a_transform_matches_core_graphics() {
    static GLASS: [Filter; 1] = [Filter::Blur(10.)];
    let mut scene = Scene::new(Color::rgb(255, 255, 255));
    let c = &mut scene.commands;
    backdrop_content(c, SIZE.0, SIZE.1);
    // Menu en fondu : la carte et son contenu dans un groupe à opacité.
    c.push(DrawCommand::PushEffect {
        transform: Affine::translation(10., 4.),
        opacity: 0.8,
        bounds: RectF::new(20., 10., 120., 90.),
    });
    card(c, RectF::new(20., 10., 120., 90.), &GLASS);
    c.push(DrawCommand::Text {
        rect: RectF::new(30., 20., 100., 20.),
        text: "Bibliothèque".into(),
        size: 14.,
        bold: false,
        color: Color::rgb(20, 20, 20),
        wrap: false,
    });
    c.push(DrawCommand::PopEffect);
    close("arriere-plan-groupe", &scene, SIZE, 0.6, 0.003);
}
#[test]
fn backdrop_inside_a_filter_group_is_ignored_like_core_graphics() {
    static GLASS: [Filter; 1] = [Filter::Blur(4.)];
    static GRAY: [Filter; 1] = [Filter::Grayscale(1.)];
    let mut scene = Scene::new(Color::rgb(255, 255, 255));
    let c = &mut scene.commands;
    backdrop_content(c, SIZE.0, SIZE.1);
    c.push(DrawCommand::PushFilter {
        id: ElementId(3),
        bounds: RectF::new(10., 10., 150., 90.),
        filters: Filters::new(&GRAY),
    });
    c.push(DrawCommand::Fill {
        rect: RectF::new(10., 10., 150., 90.),
        radius: 0.,
        color: Color::rgba(0, 120, 255, 120),
    });
    card(c, RectF::new(30., 20., 90., 60.), &GLASS);
    c.push(DrawCommand::PopFilter);
    close("arriere-plan-filtre", &scene, SIZE, 0.3, 0.001);
}
fn fade(angle: f32) -> Arc<LinearGradient> {
    Arc::new(LinearGradient::new(
        angle,
        stops(&[
            Color::rgba(0, 0, 0, 255),
            Color::rgba(0, 0, 0, 255),
            Color::rgba(0, 0, 0, 0),
        ]),
    ))
}
#[test]
fn gradient_masks_match_core_graphics() {
    let mut scene = Scene::new(Color::rgb(250, 250, 250));
    let c = &mut scene.commands;
    // Liste qui s'efface vers le bas (barre latérale de RustMusic).
    c.push(DrawCommand::PushMask {
        bounds: RectF::new(10., 10., 80., 100.),
        gradient: fade(90.),
    });
    backdrop_content(c, 100., 120.);
    // Masque imbriqué, vers la droite, sur un groupe à opacité.
    c.push(DrawCommand::PushMask {
        bounds: RectF::new(0., 40., 100., 40.),
        gradient: fade(0.),
    });
    c.push(DrawCommand::PushEffect {
        transform: Affine::IDENTITY,
        opacity: 0.7,
        bounds: RectF::new(0., 40., 100., 40.),
    });
    c.push(DrawCommand::Fill {
        rect: RectF::new(0., 40., 60., 40.),
        radius: 0.,
        color: Color::rgb(200, 0, 0),
    });
    c.push(DrawCommand::Fill {
        rect: RectF::new(40., 40., 60., 40.),
        radius: 0.,
        color: Color::rgb(0, 0, 200),
    });
    c.push(DrawCommand::PopEffect);
    c.push(DrawCommand::PopMask);
    c.push(DrawCommand::PopMask);
    // Masque sous une rotation.
    c.push(DrawCommand::PushEffect {
        transform: Affine {
            m11: 0.94,
            m12: 0.34,
            m21: -0.34,
            m22: 0.94,
            dx: 140.,
            dy: -30.,
        },
        opacity: 1.,
        bounds: RectF::new(0., 30., 60., 60.),
    });
    c.push(DrawCommand::PushMask {
        bounds: RectF::new(0., 30., 60., 60.),
        gradient: fade(45.),
    });
    c.push(DrawCommand::Fill {
        rect: RectF::new(0., 30., 60., 60.),
        radius: 8.,
        color: Color::rgb(0, 140, 60),
    });
    c.push(DrawCommand::PopMask);
    c.push(DrawCommand::PopEffect);
    close("masques", &scene, SIZE, 0.6, 0.003);
}
#[test]
fn text_filled_with_gradients_matches_core_graphics() {
    let mut scene = Scene::new(Color::rgb(255, 255, 255));
    let c = &mut scene.commands;
    let fills = [
        Background::Linear(Arc::new(LinearGradient::new(
            0.,
            stops(&[
                Color::rgb(255, 0, 120),
                Color::rgb(120, 0, 255),
                Color::rgb(0, 180, 255),
            ]),
        ))),
        Background::Radial(Arc::new(RadialGradient::new(
            [0.3, 0.5],
            [0.6, 1.],
            stops(&[Color::rgb(250, 180, 0), Color::rgb(200, 0, 40)]),
        ))),
        Background::Solid(Color::rgb(0, 120, 80)),
    ];
    for (i, fill) in fills.into_iter().enumerate() {
        let rect = RectF::new(6., 6. + i as f32 * 36., 188., 34.);
        c.push(DrawCommand::PushTextFill { rect, fill });
        let mut format =
            TextFormat::new(Default::default(), crate::ui::font::Font::system(true), 26.);
        format.decoration.underline = i == 0;
        c.push(DrawCommand::FormattedText {
            rect,
            text: "Bonsoir, David".into(),
            format,
            color: Color::rgb(0, 0, 0),
            wrap: false,
        });
        c.push(DrawCommand::PopTextFill);
    }
    close("texte-rempli", &scene, SIZE, 0.4, 0.004);
}
