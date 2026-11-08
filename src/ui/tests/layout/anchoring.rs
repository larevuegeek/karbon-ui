//! Ancrage du défilement (`overflow-anchor: auto`), cible initiale
//! (`scroll-initial-target`) et survol pendant un défilement.
use crate::ui::tests::support::*;

/// Liste de lignes de 50 px d'identifiants `100 + n` pour `n` dans `first..first + 40`,
/// dans la zone défilante `zone` ; la ligne `target` est la cible initiale.
fn rows_in(first: Rc<Cell<i64>>, zone: Rc<Cell<u64>>, target: Option<i64>) -> UiRuntime {
    view_runtime(move || {
        let first = first.get();
        Element::scroll(
            zone.get(),
            Axis::Vertical,
            Element::column(
                2,
                (first..first + 40)
                    .map(|n| {
                        let mut row = box_node((100 + n) as u64, Length::Fill(1.), Length::Px(50.));
                        row.layout_rules().scroll_initial_target = target == Some(n);
                        row
                    })
                    .collect(),
            ),
        )
    })
}
fn rows(first: Rc<Cell<i64>>) -> UiRuntime {
    rows_in(first, Rc::new(Cell::new(1)), None)
}
fn top_of(ui: &UiRuntime, id: u64) -> f32 {
    ui.root().unwrap().find(ElementId(id)).unwrap().bounds().top
}

#[test]
fn rows_inserted_or_removed_above_keep_the_watched_row_in_place() {
    let first = Rc::new(Cell::new(0));
    let mut ui = rows(first.clone());
    let mut measure = Measure::default();
    prepare(&mut ui, &mut measure);
    let p = PointF { x: 30., y: 70. };
    wheel(&mut ui, p, 0., 120.);
    prepare(&mut ui, &mut measure);
    // La ligne 102 (100..150) chevauche le haut de la vue (sous la barre de titre de
    // 40 px) : c'est l'ancre.
    let before = top_of(&ui, 102);
    near(before, 20.);
    // Dix lignes ajoutées au-dessus : la vue suit, la ligne 102 ne bouge pas.
    first.set(-10);
    ui.invalidate();
    prepare(&mut ui, &mut measure);
    near(scroll_metrics(&ui, 1).offset, 620.);
    near(top_of(&ui, 102), before);
    // Cinq lignes retirées au-dessus (fenêtre d'une liste chargée par pages).
    first.set(-5);
    ui.invalidate();
    prepare(&mut ui, &mut measure);
    near(scroll_metrics(&ui, 1).offset, 370.);
    near(top_of(&ui, 102), before);
}

#[test]
fn a_container_at_the_top_is_not_anchored() {
    // Comme Chromium : en haut, du contenu ajouté au-dessus se montre.
    let first = Rc::new(Cell::new(0));
    let mut ui = rows(first.clone());
    let mut measure = Measure::default();
    prepare(&mut ui, &mut measure);
    first.set(-4);
    ui.invalidate();
    prepare(&mut ui, &mut measure);
    near(scroll_metrics(&ui, 1).offset, 0.);
    near(top_of(&ui, 96), 40.);
}

#[test]
fn a_new_container_starts_at_its_initial_target_only() {
    let zone = Rc::new(Cell::new(1));
    let mut ui = rows_in(Rc::new(Cell::new(0)), zone.clone(), Some(4));
    let mut measure = Measure::default();
    prepare(&mut ui, &mut measure);
    near(scroll_metrics(&ui, 1).offset, 200.);
    wheel(&mut ui, PointF { x: 30., y: 70. }, 0., 30.);
    ui.invalidate();
    prepare(&mut ui, &mut measure);
    // Une reconstruction garde le défilement de l'utilisateur…
    near(scroll_metrics(&ui, 1).offset, 230.);
    // … une nouvelle zone (nouvelle liste) repart de sa cible.
    zone.set(7);
    ui.invalidate();
    prepare(&mut ui, &mut measure);
    near(scroll_metrics(&ui, 7).offset, 200.);
}

#[test]
fn hover_follows_scrolled_content_at_most_every_100_ms() {
    // Comme Chromium : pendant un défilement sous un pointeur immobile, le survol reste
    // sur la ligne d'avant, puis rejoint celle sous le pointeur au réveil ; un mouvement
    // du pointeur le remet à jour tout de suite.
    let mut ui = view_runtime(|| {
        Element::scroll(
            1,
            Axis::Vertical,
            Element::column(
                2,
                (0..40)
                    .map(|n| {
                        widgets::button(100 + n, format!("Ligne {n}"), Msg::Click(n))
                            .height(Length::Px(50.))
                    })
                    .collect(),
            ),
        )
    });
    let mut measure = Measure::default();
    prepare(&mut ui, &mut measure);
    let p = PointF { x: 30., y: 60. };
    ui.event(UiEvent::PointerMove(p));
    let first = ui.hovered_id();
    assert_eq!(first, Some(ElementId(100)));
    wheel(&mut ui, p, 0., 100.);
    prepare(&mut ui, &mut measure);
    assert_eq!(ui.hovered_id(), first);
    let wake = ui.next_wake().expect("réveil du survol");
    ui.event(UiEvent::Timer(wake));
    assert_eq!(ui.hovered_id(), Some(ElementId(102)));
    wheel(&mut ui, p, 0., 100.);
    ui.event(UiEvent::PointerMove(PointF { x: 31., y: 60. }));
    assert_eq!(ui.hovered_id(), Some(ElementId(104)));
}

#[test]
fn a_wheel_notch_glides_to_its_target_and_notches_add_up() {
    // Comme les navigateurs : un cran de molette (`UiEvent::Wheel`) glisse en 150 ms ;
    // un second cran pendant le glissement allonge la cible.
    let mut ui = rows(Rc::new(Cell::new(0)));
    let mut measure = Measure::default();
    prepare(&mut ui, &mut measure);
    let notch = |ui: &mut UiRuntime| {
        ui.event(UiEvent::Wheel {
            position: PointF { x: 30., y: 70. },
            delta_x: 0.,
            delta_y: 100.,
        })
    };
    let t0 = std::time::Instant::now();
    assert!(notch(&mut ui).handled);
    near(scroll_metrics(&ui, 1).offset, 0.);
    assert!(ui.next_wake().is_some());
    ui.event(UiEvent::Timer(t0 + std::time::Duration::from_millis(75)));
    let half = scroll_metrics(&ui, 1).offset;
    assert!(half > 0. && half < 100., "{half}");
    notch(&mut ui);
    ui.event(UiEvent::Timer(t0 + std::time::Duration::from_secs(2)));
    near(scroll_metrics(&ui, 1).offset, 200.);
    assert!(
        ui.next_wake()
            .is_none_or(|w| w > t0 + std::time::Duration::from_secs(2))
    );
    // Mouvements réduits : le cran s'applique d'un coup.
    ui.set_reduced_motion(true);
    notch(&mut ui);
    near(scroll_metrics(&ui, 1).offset, 300.);
}

/// Cran de molette de 100 px vers le bas.
fn notch(ui: &mut UiRuntime) {
    ui.event(UiEvent::Wheel {
        position: PointF { x: 30., y: 70. },
        delta_x: 0.,
        delta_y: 100.,
    });
}

#[test]
fn a_notch_during_a_glide_keeps_the_speed_reached() {
    // Comme `UpdateTarget` de Chromium : un cran pendant un glissement allonge la cible
    // sans relancer d'élan (la vitesse ne fait pas de saut : pas d'effet de pompage).
    let mut ui = rows(Rc::new(Cell::new(0)));
    let mut measure = Measure::default();
    prepare(&mut ui, &mut measure);
    let t0 = std::time::Instant::now();
    let at = |ms: u64| t0 + std::time::Duration::from_millis(ms);
    ui.event(UiEvent::Timer(t0));
    notch(&mut ui);
    let sample = |ui: &mut UiRuntime, ms: u64| {
        ui.event(UiEvent::Timer(at(ms)));
        scroll_metrics(ui, 1).offset
    };
    let a = sample(&mut ui, 50);
    let b = sample(&mut ui, 60);
    let before = (b - a) / 10.;
    notch(&mut ui);
    let c = sample(&mut ui, 70);
    let after = (c - b) / 10.;
    assert!(before > 0. && after > 0., "{before} {after}");
    assert!(
        after / before < 1.3,
        "la vitesse saute de {before} à {after} px/ms"
    );
    // Les deux crans finissent par s'ajouter, en douceur jusqu'à l'arrêt.
    let end = sample(&mut ui, 1000);
    near(end, 200.);
    assert!(ui.next_wake().is_none_or(|w| w > at(1000)));
}

#[test]
fn a_frame_slightly_early_still_moves_the_glide() {
    // La cadence de l'écran ne tombe pas pile sur le réveil prévu : une image arrivée un
    // peu avant avance quand même (aucune image figée pendant le glissement).
    let mut ui = rows(Rc::new(Cell::new(0)));
    let mut measure = Measure::default();
    prepare(&mut ui, &mut measure);
    let t0 = std::time::Instant::now();
    ui.event(UiEvent::Timer(t0));
    notch(&mut ui);
    let at = |us: u64| t0 + std::time::Duration::from_micros(us);
    ui.event(UiEvent::Timer(at(16_667)));
    let first = scroll_metrics(&ui, 1).offset;
    ui.event(UiEvent::Timer(at(32_000)));
    assert!(scroll_metrics(&ui, 1).offset > first);
}

#[test]
fn an_anchored_shift_carries_the_glide_along() {
    // Une liste chargée par pages retire des lignes au-dessus pendant un glissement :
    // l'ancrage corrige le décalage, et la cible du glissement suit le contenu au lieu
    // de le ramener vers l'ancienne position.
    let first = Rc::new(Cell::new(0));
    let mut ui = rows(first.clone());
    let mut measure = Measure::default();
    prepare(&mut ui, &mut measure);
    let t0 = std::time::Instant::now();
    let at = |ms: u64| t0 + std::time::Duration::from_millis(ms);
    ui.event(UiEvent::Timer(t0));
    // Trois crans : cible 300.
    notch(&mut ui);
    notch(&mut ui);
    notch(&mut ui);
    ui.event(UiEvent::Timer(at(50)));
    prepare(&mut ui, &mut measure);
    let offset = scroll_metrics(&ui, 1).offset;
    assert!(offset > 50. && offset < 300., "{offset}");
    // Une ligne (50 px) retirée au-dessus.
    first.set(1);
    ui.invalidate();
    prepare(&mut ui, &mut measure);
    near(scroll_metrics(&ui, 1).offset, offset - 50.);
    ui.event(UiEvent::Timer(at(1000)));
    near(scroll_metrics(&ui, 1).offset, 250.);
}
