//! `--headless N` : le banc dans la fenêtre simulée de karbon-ui, sans écran ni carte
//! graphique. N images de la vue sont livrées une à une ; pour chacune, la préparation
//! de l'image (vue, styles, mise en page, peinture de la scène) est mesurée sur le fil
//! qui l'appelle, avec la part de chaque passe (`RuntimeStats`). Le rendu (Direct2D)
//! n'y est pas : c'est la part de l'interface indépendante du système.
use super::{Bench, producer};
use karbon_ui::{
    platform::headless::{HeadlessApp, HeadlessOptions},
    task::Inbox,
    ui::{scene::LiveImage, template::ImageSlot},
};
use std::time::{Duration, Instant};

/// Mesure `frames` images en fenêtre simulée de `size` unités à 150 %.
pub fn run(frames: usize, size: (i32, i32)) -> Result<(), Box<dyn std::error::Error>> {
    let live = LiveImage::new();
    let slot: ImageSlot = live.slot().clone();
    let bench = Bench::new(slot, Inbox::new(), Duration::from_secs(3600));
    let mut app = HeadlessApp::with_options(
        bench,
        HeadlessOptions {
            width: size.0 as f32,
            height: size.1 as f32,
            scale: 1.5,
            ..Default::default()
        },
    )?;
    app.prepare()?;
    let mut times = Vec::with_capacity(frames);
    let before = app.runtime().stats();
    let mut scene_commands = 0;
    for frame in 0..frames {
        live.set(std::sync::Arc::new(producer::view_image(
            frame as u32,
            true,
        )));
        let started = Instant::now();
        app.prepare()?;
        times.push(started.elapsed());
        scene_commands = app.scene().commands.len();
    }
    let after = app.runtime().stats();
    let per = |a: Duration, b: Duration| (a - b).as_secs_f64() * 1000. / frames as f64;
    times.sort();
    let at = |q: f64| times[((times.len() - 1) as f64 * q) as usize].as_secs_f64() * 1000.;
    println!(
        "Banc éditeur (fenêtre simulée {}×{} à 150 %) : {frames} images, préparation p50 {:.3} ms, p95 {:.3} ms ; par image : vue {:.3}, styles {:.3}, mise en page {:.3}, peinture {:.3} ms ; {} peintures, {} mises en page ; {scene_commands} commandes",
        size.0,
        size.1,
        at(0.5),
        at(0.95),
        per(after.view_time, before.view_time),
        per(after.style_time, before.style_time),
        per(after.layout_time, before.layout_time),
        per(after.paint_time, before.paint_time),
        after.paints - before.paints,
        after.layouts - before.layouts,
    );
    Ok(())
}
