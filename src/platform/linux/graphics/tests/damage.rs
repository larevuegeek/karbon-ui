//! Redessin partiel : une image mise à jour par zone modifiée est identique à la même
//! scène dessinée en entier.
use super::*;
use crate::{
    platform::linux::bitmap::Bitmap,
    ui::{
        element::ElementId,
        font::Font,
        rendering::filter::{Filter, Filters},
        scene::{DrawCommand, RectF},
    },
};

static BLUR: [Filter; 1] = [Filter::Blur(6.)];
fn scene(button: Color, x: f32) -> Scene {
    let mut scene = Scene::new(Color::rgb(20, 20, 30));
    scene.commands.push(DrawCommand::PushFilter {
        id: ElementId(7),
        bounds: RectF::new(20., 20., 60., 60.),
        filters: Filters::new(&BLUR),
    });
    scene.commands.push(DrawCommand::Fill {
        rect: RectF::new(20., 20., 60., 60.),
        radius: 8.,
        color: Color::rgb(200, 40, 40),
    });
    scene.commands.push(DrawCommand::PopFilter);
    scene.font_text(
        RectF::new(100., 20., 180., 30.),
        "Bonsoir, David",
        18.,
        Font::system(true),
        Color::rgb(255, 255, 255),
    );
    scene.commands.push(DrawCommand::Fill {
        rect: RectF::new(x, 120., 40., 20.),
        radius: 4.,
        color: Color::rgb(30, 200, 90),
    });
    scene.commands.push(DrawCommand::Fill {
        rect: RectF::new(200., 120., 60., 30.),
        radius: 6.,
        color: button,
    });
    scene.commands.push(DrawCommand::Backdrop {
        rect: RectF::new(0., 100., 300., 60.),
        radii: [0.; 4],
        filters: Filters::new(&BLUR),
    });
    scene
}
#[test]
fn partial_redraws_match_a_full_redraw_pixel_for_pixel() {
    let (w, h, scale) = (600, 400, 2.);
    let mut incremental = renderer();
    let target = Bitmap::new(w, h).unwrap();
    let first = scene(Color::rgb(60, 60, 200), 20.);
    assert!(incremental.draw_changes(&target, &first, scale).unwrap());
    assert!(
        !incremental.draw_changes(&target, &first, scale).unwrap(),
        "une scène identique ne redessine rien"
    );
    for (button, x) in [
        (Color::rgb(90, 90, 240), 20.),
        (Color::rgb(90, 90, 240), 60.),
    ] {
        let next = scene(button, x);
        assert!(incremental.draw_changes(&target, &next, scale).unwrap());
        let full = renderer().render(&next, w as u32, h as u32, scale).unwrap();
        let differing = target
            .pixels()
            .iter()
            .zip(&full)
            .filter(|(a, b)| a.abs_diff(**b) > 1)
            .count();
        assert_eq!(differing, 0, "pixels différents après un redessin partiel");
    }
}
#[test]
fn distant_changes_are_separate_zones_and_a_window_sized_clip_changes_nothing() {
    let (w, h, scale) = (600, 400, 2.);
    let mut incremental = renderer();
    let target = Bitmap::new(w, h).unwrap();
    let first = scene(Color::rgb(60, 60, 200), 20.);
    incremental.draw_changes(&target, &first, scale).unwrap();
    // Deux changements éloignés : deux zones, pas une zone englobante.
    let mut next = scene(Color::rgb(90, 90, 240), 20.);
    if let DrawCommand::FontText { color, .. } = &mut next.commands[3] {
        *color = Color::rgb(200, 200, 0);
    }
    assert!(incremental.draw_changes(&target, &next, scale).unwrap());
    let zones = incremental.last_region.clone().expect("redessin partiel");
    assert!(zones.len() >= 2, "{zones:?}");
    let full = renderer().render(&next, w as u32, h as u32, scale).unwrap();
    assert!(
        target
            .pixels()
            .iter()
            .zip(&full)
            .all(|(a, b)| a.abs_diff(*b) <= 1)
    );
    // Une découpe de la taille de la fenêtre ajoutée autour de tout (ouverture d'un
    // menu) ne change aucun pixel : rien n'est redessiné.
    let mut wrapped = Scene::new(next.clear);
    wrapped
        .commands
        .push(DrawCommand::PushClip(RectF::new(0., 0., 300., 200.)));
    wrapped.commands.extend(next.commands.iter().cloned());
    wrapped.commands.push(DrawCommand::PopClip);
    assert!(!incremental.draw_changes(&target, &wrapped, scale).unwrap());
}
