//! Choix du chemin et ordre des étapes : toutes les commandes passent par Metal, glyphes
//! en couleur sous `background-clip: text` compris ; un `backdrop-filter` ne coupe le
//! dessin de la cible que si sa zone a été touchée.
use super::metal;
use crate::platform::macos::graphics::Renderer as CgRenderer;
use crate::platform::macos::metal::encode::Step;
use crate::ui::{
    element::ElementId,
    rendering::{
        brush::{Background, GradientStop, LinearGradient},
        filter::{Filter, Filters},
    },
    scene::{DrawCommand, RectF, Scene},
    theme::Color,
};
use std::sync::Arc;

fn gradient() -> Arc<LinearGradient> {
    Arc::new(LinearGradient::new(
        0.,
        [Color::rgb(0, 0, 0), Color::rgb(255, 255, 255)]
            .into_iter()
            .enumerate()
            .map(|(i, color)| GradientStop {
                position: i as f32,
                color,
            }),
    ))
}
fn fill(rect: RectF, color: Color) -> DrawCommand {
    DrawCommand::Fill {
        rect,
        radius: 0.,
        color,
    }
}
fn text(scene: &mut Scene, value: &str) {
    scene.commands.push(DrawCommand::Text {
        rect: RectF::new(0., 0., 60., 20.),
        text: value.into(),
        size: 14.,
        bold: false,
        color: Color::rgb(0, 0, 0),
        wrap: false,
    });
}
#[test]
fn filters_backdrops_masks_and_text_fills_are_drawn_by_metal() {
    static BLUR: [Filter; 2] = [Filter::Blur(2.), Filter::Saturate(1.5)];
    let Some(mut metal) = metal() else {
        return;
    };
    let mut cg = CgRenderer::new().unwrap();
    let bounds = RectF::new(0., 0., 40., 20.);
    let mut scene = Scene::new(Color::rgb(255, 255, 255));
    let c = &mut scene.commands;
    c.push(DrawCommand::PushFilter {
        id: ElementId(1),
        bounds,
        filters: Filters::new(&BLUR),
    });
    c.push(fill(bounds, Color::rgb(200, 0, 0)));
    c.push(DrawCommand::PopFilter);
    c.push(DrawCommand::Backdrop {
        rect: bounds,
        radii: [4.; 4],
        filters: Filters::new(&BLUR),
    });
    c.push(DrawCommand::PushMask {
        bounds,
        gradient: gradient(),
    });
    c.push(fill(bounds, Color::rgb(0, 0, 200)));
    c.push(DrawCommand::PopMask);
    c.push(DrawCommand::PushTextFill {
        rect: bounds,
        fill: Background::Linear(gradient()),
    });
    text(&mut scene, "Texte");
    scene.commands.push(DrawCommand::PopTextFill);
    assert!(metal.render(&mut cg, &scene, 60, 30, 1.).is_ok());
}
/// Plus petit écart moyen Metal / Core Graphics de l'émoji `fill` (ou sans remplissage)
/// posé à dix hauteurs d'un dixième de pixel : Core Graphics pose l'image d'un émoji à
/// sa position verticale fractionnaire, Metal sur un pixel entier ; à la hauteur où la
/// ligne de base tombe sur un pixel, les deux rendus coïncident.
fn best_emoji(fill: Option<Background>, scale: f32) -> f64 {
    let Some(mut metal) = metal() else {
        return 0.;
    };
    let mut cg = CgRenderer::new().unwrap();
    (0..10)
        .map(|k| {
            let rect = RectF::new(0., k as f32 / 10. / scale, 60., 20.);
            let mut scene = Scene::new(Color::rgb(255, 255, 255));
            if let Some(fill) = &fill {
                scene.commands.push(DrawCommand::PushTextFill {
                    rect,
                    fill: fill.clone(),
                });
            }
            scene.commands.push(DrawCommand::Text {
                rect,
                text: "😀".into(),
                size: 14.,
                bold: false,
                color: Color::rgb(0, 0, 0),
                wrap: false,
            });
            if fill.is_some() {
                scene.commands.push(DrawCommand::PopTextFill);
            }
            let (w, h) = ((60. * scale) as u32, (24. * scale) as u32);
            let m = metal.render(&mut cg, &scene, w, h, scale).unwrap();
            let c = cg.render(&scene, w, h, scale).unwrap();
            super::difference(&m, &c).mean
        })
        .fold(f64::MAX, f64::min)
}
#[test]
fn colour_glyphs_under_a_text_fill_are_luminance_masks_like_core_graphics() {
    // Émoji sous un dégradé et sous une couleur : sa luminance masque le remplissage,
    // comme le masque en niveaux de gris de Core Graphics ; sans remplissage, il garde
    // ses couleurs.
    for scale in [1f32, 2.] {
        for (label, fill) in [
            ("dégradé", Some(Background::Linear(gradient()))),
            ("couleur", Some(Background::Solid(Color::rgb(200, 0, 90)))),
            ("sans remplissage", None),
        ] {
            let best = best_emoji(fill, scale);
            assert!(best <= 0.25, "émoji ({label}) à {scale}x : écart {best:.3}");
        }
    }
}
/// Passages de dessin de `scene` : nombre d'instances de chaque couche dessinée, dans
/// l'ordre des étapes.
fn draws(scene: &Scene) -> Vec<usize> {
    let Some(mut metal) = metal() else {
        return Vec::new();
    };
    let mut cg = CgRenderer::new().unwrap();
    let built = metal.build(&mut cg, scene, (100, 40), 1.).unwrap();
    metal.resources.end();
    built
        .steps
        .iter()
        .filter_map(|s| match s {
            Step::Draw(layer) => Some(built.layers[*layer].instances.len()),
            Step::Filter(_) => None,
        })
        .collect()
}
#[test]
fn a_backdrop_replays_only_what_was_painted_under_it_before() {
    static BLUR: [Filter; 1] = [Filter::Blur(1.)];
    if metal().is_none() {
        return;
    }
    let backdrop = |left: f32| DrawCommand::Backdrop {
        rect: RectF::new(left, 10., 20., 20.),
        radii: [0.; 4],
        filters: Filters::new(&BLUR),
    };
    // Fond, deux cartes éloignées : chaque zone relue ne contient que le fond ; la cible
    // est dessinée en un passage, en dernier (fond, deux recompositions, carte blanche).
    let mut scene = Scene::new(Color::rgb(255, 255, 255));
    scene
        .commands
        .push(fill(RectF::new(0., 0., 100., 40.), Color::rgb(0, 90, 0)));
    scene.commands.push(backdrop(5.));
    scene.commands.push(fill(
        RectF::new(5., 10., 20., 20.),
        Color::rgb(255, 255, 255),
    ));
    scene.commands.push(backdrop(60.));
    assert_eq!(draws(&scene), [1, 1, 4]);
    // Un élément peint sous la seconde carte avant elle y est relu.
    scene.commands.insert(
        3,
        fill(RectF::new(55., 0., 30., 40.), Color::rgb(0, 0, 200)),
    );
    assert_eq!(draws(&scene), [1, 2, 5]);
}
