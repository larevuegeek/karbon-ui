//! Coût d'un mouvement du pointeur selon le nombre d'éléments hors de la vue (KUI-057, 40 000 : l'arbre en admet 50 000) :
//! une liste défilante de N lignes construites (pas virtualisées), dont une trentaine
//! est visible ; le pointeur va et vient sur les lignes visibles. Mesure à la demande :
//! `cargo test --release --lib pointer_cost -- --ignored --nocapture`.
use crate::{
    platform::headless::{HeadlessApp, HeadlessOptions},
    ui::{
        element::{Axis, Element, Length},
        prelude::*,
        runtime::UiEvent,
        scene::PointF,
    },
};
use std::time::{Duration, Instant};

struct Rows {
    count: usize,
}

impl Application for Rows {
    type Message = bool;
    fn view(&self, _: &ViewContext) -> Element {
        let rows = (0..self.count).map(|i| {
            column(ElementId::named("row").item(i as u64), [])
                .height(Length::Px(20.))
                .on_hover(|on| on)
        });
        let list = column("list", rows.collect::<Vec<_>>());
        Element::scroll(ElementId::from("scroll").0, Axis::Vertical, list)
    }
    fn update(&mut self, _: bool) -> bool {
        false
    }
}

/// Temps médian d'un mouvement du pointeur (événement et image suivante).
fn median_move(count: usize) -> Duration {
    let mut app = HeadlessApp::with_options(
        Rows { count },
        HeadlessOptions {
            width: 400.,
            height: 600.,
            ..Default::default()
        },
    )
    .unwrap();
    let mut times = Vec::new();
    let before = app.runtime().stats();
    for step in 0..400 {
        let p = PointF {
            x: 100.,
            y: 10. + (step % 25) as f32 * 20.,
        };
        app.prepare().unwrap();
        let started = Instant::now();
        app.runtime_mut().event(UiEvent::PointerMove(p));
        let event = started.elapsed();
        app.prepare().unwrap();
        times.push((started.elapsed(), event));
    }
    let after = app.runtime().stats();
    let per = |a: Duration, b: Duration| (a - b).as_secs_f64() * 1000. / 400.;
    eprintln!(
        "  {count} : styles {:.3}, mise en page {:.3}, peinture {:.3} ms par mouvement",
        per(after.style_time, before.style_time),
        per(after.layout_time, before.layout_time),
        per(after.paint_time, before.paint_time)
    );
    times.sort();
    let (total, event) = times[times.len() / 2];
    eprintln!(
        "  {count} : événement {:.3} ms, total {:.3} ms",
        event.as_secs_f64() * 1000.,
        total.as_secs_f64() * 1000.
    );
    total
}

#[test]
#[ignore]
fn pointer_cost() {
    let small = median_move(1_000);
    let large = median_move(40_000);
    println!(
        "Mouvement du pointeur, médiane : 1 000 éléments {:.3} ms, 40 000 éléments {:.3} ms",
        small.as_secs_f64() * 1000.,
        large.as_secs_f64() * 1000.
    );
}
