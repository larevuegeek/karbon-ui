//! Redessin partiel de la chaîne d'échange (KUI-055) : sur une suite d'images où
//! changent un survol, un compteur, une image, un groupe translucide, un groupe filtré
//! et un fond flouté, le tampon dessiné en partie est, à chaque image, celui d'un rendu
//! entier, au pixel près ; un survol ne redessine qu'une petite part de la fenêtre.
use super::super::*;
use super::swapchain::back_buffer;
use crate::ui::{
    element::ElementId,
    rendering::filter::{Filter, Filters},
};

const ROWS: usize = 24;

/// L'image `frame` du scénario : chaque partie change à son rythme ; une image sur dix
/// répète la précédente, une sur 97 change tout (le fond).
fn scene(frame: usize) -> Scene {
    let t = if frame % 10 == 9 { frame - 1 } else { frame };
    let shade = if (t / 97) % 2 == 0 { 24 } else { 40 };
    let mut scene = Scene::new(Color::rgb(shade, shade, shade + 8));
    scene.fill(RectF::new(0., 0., 120., 200.), 0., Color::rgb(36, 38, 44));
    let hovered = (t / 3) % ROWS;
    for row in 0..ROWS {
        let top = 4. + row as f32 * 8.;
        if row == hovered {
            scene.fill(RectF::new(2., top, 116., 8.), 3., Color::rgb(70, 90, 160));
        }
        scene.text(
            RectF::new(6., top, 110., 8.),
            format!("Objet {row}"),
            6.,
            row % 5 == 0,
            Color::rgb(220, 220, 230),
        );
    }
    scene.text(
        RectF::new(200., 2., 60., 10.),
        format!("Image {t}"),
        7.,
        true,
        Color::rgb(255, 200, 80),
    );
    let tone = ((t / 5) * 37 % 256) as u8;
    let pixels = (0..16 * 12)
        .flat_map(|i| [tone, (i * 7 % 256) as u8, 255 - tone, 255])
        .collect();
    scene.commands.push(DrawCommand::Image {
        rect: RectF::new(130., 20., 64., 48.),
        image: Arc::new(Image::new(16, 12, pixels).unwrap()),
    });
    let x = 130. + ((t / 7) % 6) as f32 * 9.;
    scene.commands.push(DrawCommand::PushEffect {
        transform: crate::ui::rendering::transform::Affine::IDENTITY,
        opacity: 0.5,
        bounds: RectF::new(120., 80., 100., 40.),
    });
    scene.fill(RectF::new(x, 84., 40., 30.), 6., Color::rgb(240, 80, 80));
    scene.commands.push(DrawCommand::PopEffect);
    scene.commands.push(DrawCommand::PushShapeClip {
        rect: RectF::new(210., 30., 80., 40.),
        radii: [10.; 4],
    });
    scene.fill(
        RectF::new(200., 20., 100., 60.),
        0.,
        Color::rgb(60, 160, 90),
    );
    scene.text(
        RectF::new(214., 40., 70., 10.),
        format!("clé {}", (t / 11) % 9),
        7.,
        false,
        Color::rgb(10, 10, 10),
    );
    scene.commands.push(DrawCommand::PopShapeClip);
    scene.commands.push(DrawCommand::PushFilter {
        id: ElementId(7),
        bounds: RectF::new(220., 120., 40., 40.),
        filters: Filters::new(&[Filter::Blur(3.)]),
    });
    let blurred = if (t / 50) % 2 == 0 { 0 } else { 200 };
    scene.fill(
        RectF::new(226., 126., 28., 28.),
        0.,
        Color::rgb(blurred, 120, 255),
    );
    scene.commands.push(DrawCommand::PopFilter);
    scene.commands.push(DrawCommand::Backdrop {
        rect: RectF::new(60., 150., 80., 30.),
        radii: [4.; 4],
        filters: Filters::new(&[Filter::Blur(2.)]),
    });
    scene
}

/// Rejoue `frames` images du scénario : le tampon dessiné en partie égale le rendu
/// entier de la même scène, image par image ; rend la part des pixels redessinés.
pub(super) fn verify_partial(mode: RenderMode, frames: usize) -> f64 {
    let (width, height, scale) = (450, 300, 1.5);
    let mut partial = Renderer::new(mode).unwrap();
    let mut full = Renderer::new(mode).unwrap();
    for frame in 0..frames {
        let scene = scene(frame);
        partial
            .prepare_swap_chain(width, height, scale, true)
            .unwrap();
        partial
            .render_changes_to_swap_chain(&scene, width, height, scale, true)
            .unwrap();
        let expected = full.render(&scene, width, height, scale).unwrap();
        let drawn = back_buffer(&partial);
        if drawn != expected {
            let wrong = drawn
                .chunks(4)
                .zip(expected.chunks(4))
                .position(|(a, b)| a != b)
                .unwrap();
            panic!(
                "image {frame} : le pixel ({}, {}) diffère du rendu entier",
                wrong % width as usize,
                wrong / width as usize
            );
        }
        partial.present().unwrap();
    }
    partial.partial_share().unwrap()
}

#[test]
fn partial_redraws_match_whole_redraws_pixel_for_pixel() {
    let share = verify_partial(RenderMode::Software, 300);
    // Bien moins que tout : le scénario ne change que de petites parties.
    assert!(share < 0.6, "part redessinée {share}");
}

/// Les 1 000 images du critère (KUI-055) ; une trentaine de secondes sur WARP.
#[test]
#[ignore = "long : lancer avec --ignored"]
fn a_thousand_partial_redraws_match_whole_redraws() {
    verify_partial(RenderMode::Software, 1000);
}

/// Une liste de 60 lignes dans une fenêtre de 1 280 × 800 : le survol passe d'une
/// ligne à la suivante.
fn list(hovered: usize) -> Scene {
    let mut scene = Scene::new(Color::rgb(20, 20, 24));
    scene.fill(RectF::new(0., 0., 300., 800.), 0., Color::rgb(36, 38, 44));
    for row in 0..60 {
        let top = row as f32 * 13.;
        if row == hovered {
            scene.fill(RectF::new(2., top, 296., 13.), 3., Color::rgb(70, 90, 160));
        }
        scene.text(
            RectF::new(8., top, 280., 13.),
            format!("Ligne {row}"),
            9.,
            false,
            Color::rgb(220, 220, 230),
        );
    }
    scene.fill(
        RectF::new(320., 20., 900., 700.),
        8.,
        Color::rgb(50, 60, 70),
    );
    scene
}

#[test]
fn hovering_a_row_redraws_less_than_five_percent_of_the_window() {
    let (width, height) = (1280, 800);
    let mut renderer = Renderer::new(RenderMode::Software).unwrap();
    let mut step = |scene: &Scene| {
        let before = renderer.partial.painted;
        renderer
            .prepare_swap_chain(width, height, 1., false)
            .unwrap();
        renderer
            .render_changes_to_swap_chain(scene, width, height, 1., false)
            .unwrap();
        renderer.present().unwrap();
        (renderer.partial.painted - before) as f64 / f64::from(width * height)
    };
    // Les deux premières images sont entières (tampons inconnus), puis rien ne change.
    assert_eq!(step(&list(3)), 1.);
    assert_eq!(step(&list(3)), 1.);
    assert_eq!(step(&list(3)), 0.);
    let hover = step(&list(4));
    let next = step(&list(4));
    assert!(hover > 0. && hover < 0.05, "survol : {hover}");
    assert!(next > 0. && next < 0.05, "image suivante : {next}");
    assert_eq!(step(&list(4)), 0.);
}
