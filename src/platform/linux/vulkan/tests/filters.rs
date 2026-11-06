//! Groupes filtrés rendus par Vulkan, comparés au rendu Cairo (modèle : la suite Metal) :
//! flou (petit, réduit, très grand), ombre portée, chaque matrice de couleur,
//! enchaînements, groupe filtré dans un groupe à opacité, et résultat gardé d'une image à
//! l'autre.
use super::{compare, renderer};
use crate::ui::{
    element::ElementId,
    rendering::{
        brush::{GradientStop, LinearGradient},
        filter::{Filter, Filters},
        transform::Affine,
    },
    scene::{DrawCommand, RectF, Scene},
    theme::Color,
};
use std::sync::Arc;

pub(super) fn stops(colors: &[Color]) -> Vec<GradientStop> {
    let last = (colors.len() - 1).max(1) as f32;
    colors
        .iter()
        .enumerate()
        .map(|(i, &color)| GradientStop {
            position: i as f32 / last,
            color,
        })
        .collect()
}
/// Fond varié : dégradé, bandes, pastilles et texte.
pub(super) fn backdrop_content(c: &mut Vec<DrawCommand>, width: f32, height: f32) {
    c.push(DrawCommand::Gradient {
        rect: RectF::new(0., 0., width, height),
        radius: 0.,
        gradient: Arc::new(LinearGradient::new(
            30.,
            stops(&[
                Color::rgb(20, 40, 120),
                Color::rgb(200, 60, 90),
                Color::rgb(250, 210, 80),
            ]),
        )),
    });
    let mut x = 0.;
    while x < width {
        c.push(DrawCommand::Fill {
            rect: RectF::new(x, 0., 6., height),
            radius: 0.,
            color: Color::rgba(255, 255, 255, 90),
        });
        x += 17.;
    }
    for (i, color) in [
        Color::rgb(0, 160, 120),
        Color::rgb(240, 240, 240),
        Color::rgb(30, 30, 30),
    ]
    .into_iter()
    .enumerate()
    {
        c.push(DrawCommand::Fill {
            rect: RectF::new(20. + i as f32 * 45., 25., 34., 34.),
            radius: 17.,
            color,
        });
    }
    c.push(DrawCommand::Text {
        rect: RectF::new(12., 70., 180., 24.),
        text: "Lecture en cours".into(),
        size: 18.,
        bold: true,
        color: Color::rgb(255, 255, 255),
        wrap: false,
    });
}
/// Scène : le fond, puis `content` sous les filtres `filters`.
fn filtered(filters: &'static [Filter], scale: f32) -> Scene {
    let mut scene = Scene::new(Color::rgb(245, 245, 245));
    let c = &mut scene.commands;
    let bounds = RectF::new(30., 20., 120., 70.);
    c.push(DrawCommand::Fill {
        rect: RectF::new(0., 0., 200., 40.),
        radius: 0.,
        color: Color::rgb(40, 120, 220),
    });
    c.push(DrawCommand::PushFilter {
        id: ElementId(7),
        bounds,
        filters: Filters {
            values: filters,
            scale,
        },
    });
    c.push(DrawCommand::Fill {
        rect: bounds,
        radius: 12.,
        color: Color::rgb(220, 70, 40),
    });
    c.push(DrawCommand::Fill {
        rect: RectF::new(50., 35., 50., 30.),
        radius: 0.,
        color: Color::rgba(20, 200, 90, 200),
    });
    c.push(DrawCommand::Text {
        rect: RectF::new(40., 60., 110., 20.),
        text: "Filtré".into(),
        size: 16.,
        bold: true,
        color: Color::rgb(255, 255, 255),
        wrap: false,
    });
    c.push(DrawCommand::PopFilter);
    scene
}
const SIZE: (f32, f32) = (200., 120.);

#[test]
fn small_blur_matches_cairo() {
    static F: [Filter; 1] = [Filter::Blur(1.5)];
    compare("flou-petit", &filtered(&F, 1.), SIZE.0, SIZE.1, 0.6, 0.002);
}
#[test]
fn reduced_blur_matches_cairo() {
    static F: [Filter; 1] = [Filter::Blur(8.)];
    compare("flou-moyen", &filtered(&F, 1.), SIZE.0, SIZE.1, 1.0, 0.002);
}
#[test]
fn large_blur_drawn_reduced_matches_cairo() {
    static F: [Filter; 1] = [Filter::Blur(30.)];
    compare("flou-grand", &filtered(&F, 1.), SIZE.0, SIZE.1, 2.0, 0.005);
}
#[test]
fn drop_shadow_matches_cairo() {
    static F: [Filter; 1] = [Filter::DropShadow {
        x: 4.,
        y: 6.,
        blur: 5.,
        color: Color::rgba(0, 0, 0, 160),
    }];
    compare("ombre", &filtered(&F, 1.), SIZE.0, SIZE.1, 0.6, 0.002);
}
#[test]
fn colour_matrices_match_cairo() {
    static BRIGHTNESS: [Filter; 1] = [Filter::Brightness(1.4)];
    static CONTRAST: [Filter; 1] = [Filter::Contrast(0.6)];
    static SATURATE: [Filter; 1] = [Filter::Saturate(2.)];
    static GRAYSCALE: [Filter; 1] = [Filter::Grayscale(1.)];
    static SEPIA: [Filter; 1] = [Filter::Sepia(0.8)];
    static INVERT: [Filter; 1] = [Filter::Invert(1.)];
    static HUE: [Filter; 1] = [Filter::HueRotate(90.)];
    static OPACITY: [Filter; 1] = [Filter::Opacity(0.5)];
    for (name, filters) in [
        ("luminosite", &BRIGHTNESS[..]),
        ("contraste", &CONTRAST),
        ("saturation", &SATURATE),
        ("gris", &GRAYSCALE),
        ("sepia", &SEPIA),
        ("inversion", &INVERT),
        ("teinte", &HUE),
        ("opacite-filtre", &OPACITY),
    ] {
        compare(name, &filtered(filters, 1.), SIZE.0, SIZE.1, 0.3, 0.001);
    }
}
#[test]
fn filter_chains_match_cairo() {
    static CHAIN: [Filter; 4] = [
        Filter::Blur(3.),
        Filter::Saturate(1.8),
        Filter::DropShadow {
            x: -3.,
            y: 2.,
            blur: 2.,
            color: Color::rgba(40, 0, 80, 200),
        },
        Filter::Brightness(0.8),
    ];
    // Échelle de mise en page 1,5 (rayons et décalages multipliés).
    compare(
        "enchainement",
        &filtered(&CHAIN, 1.5),
        SIZE.0,
        SIZE.1,
        0.6,
        0.003,
    );
    // Grand flou puis saturation (pochettes floutées de RustMusic).
    static COVER: [Filter; 2] = [Filter::Blur(36.), Filter::Saturate(1.8)];
    compare(
        "flou-sature",
        &filtered(&COVER, 1.),
        SIZE.0,
        SIZE.1,
        2.6,
        0.005,
    );
}
#[test]
fn filtered_group_inside_an_opacity_group_and_a_transform_matches_cairo() {
    static F: [Filter; 1] = [Filter::Blur(2.)];
    let inner = filtered(&F, 1.);
    let mut scene = Scene::new(Color::rgb(255, 255, 255));
    let c = &mut scene.commands;
    c.push(DrawCommand::PushEffect {
        transform: Affine::translation(6.5, -4.),
        opacity: 0.6,
        bounds: RectF::new(0., 0., 200., 120.),
    });
    c.extend(inner.commands.iter().cloned());
    c.push(DrawCommand::PopEffect);
    compare("filtre-groupe", &scene, SIZE.0, SIZE.1, 0.6, 0.003);
}
#[test]
fn filtered_groups_are_kept_until_their_content_changes() {
    static F: [Filter; 1] = [Filter::Blur(4.)];
    let Some(mut vulkan) = renderer() else {
        return;
    };
    let (w, h) = (400, 240);
    let scene = filtered(&F, 1.);
    let first = vulkan.render(&scene, w, h, 2.).unwrap();
    let again = vulkan.render(&scene, w, h, 2.).unwrap();
    assert!(first == again, "le résultat gardé est identique");
    // Contenu changé : recalculé, comme par un renderer neuf.
    let mut changed = scene.clone();
    if let Some(DrawCommand::Fill { color, .. }) = changed
        .commands
        .iter_mut()
        .rev()
        .find(|c| matches!(c, DrawCommand::Fill { .. }))
    {
        *color = Color::rgb(0, 0, 0);
    }
    let updated = vulkan.render(&changed, w, h, 2.).unwrap();
    let expected = renderer().unwrap().render(&changed, w, h, 2.).unwrap();
    assert!(updated != first && updated == expected);
}
