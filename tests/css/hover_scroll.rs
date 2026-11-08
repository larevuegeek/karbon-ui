//! Survol d'une liste après un défilement doux à la molette : la ligne sous le pointeur
//! s'allume quand la souris bouge de nouveau, comme dans un navigateur.
use crate::support::*;
use karbon_ui::ui::{
    prelude::*,
    runtime::{UiEvent, UiRuntime},
    scene::PointF,
};
use std::time::{Duration, Instant};
css_file!(mod css, "hover_scroll.css");

struct List;
impl Application for List {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        ui! {@styles(css);
            <div id="list" class="list">
                {#each (0..30usize) as i (i)}
                    <div class="line" memo={i} onclick={Msg::Click(1)}>{format!("Ligne {i}")}</div>
                {/each}
            </div>
        }
    }
    fn update(&mut self, _: Msg) -> bool {
        false
    }
}

/// Indices des lignes allumées.
fn lit(ui: &UiRuntime) -> Vec<usize> {
    let list = ui.root().unwrap().find(ElementId::named("list")).unwrap();
    // La zone défilante enveloppe son contenu dans un élément intérieur.
    list.children[0]
        .children
        .iter()
        .enumerate()
        .filter(|(_, line)| line.style.background == Some(Color::hex(0xff0000)))
        .map(|(i, _)| i)
        .collect()
}

#[test]
fn the_row_under_the_pointer_lights_up_after_a_wheel_scroll() {
    let mut ui = UiRuntime::new(List, &Services).unwrap();
    let mut now = Instant::now();
    prepare_headless(&mut ui, now);
    let at = PointF { x: 50., y: 50. };
    ui.event(UiEvent::PointerMove(at));
    for _ in 0..20 {
        now += Duration::from_millis(16);
        ui.event(UiEvent::Timer(now));
        prepare_headless(&mut ui, now);
    }
    assert_eq!(lit(&ui), vec![1]);
    // Deux crans vers le bas (200 px), glissement mené à son terme.
    ui.event(UiEvent::Wheel {
        position: at,
        delta_x: 0.,
        delta_y: 200.,
    });
    for _ in 0..30 {
        now += Duration::from_millis(16);
        ui.event(UiEvent::Timer(now));
        prepare_headless(&mut ui, now);
    }
    // La souris bouge d'une ligne : celle qui est dessous s'allume (5 lignes défilées),
    // au terme de sa transition de couleur.
    ui.event(UiEvent::PointerMove(PointF { x: 50., y: 90. }));
    for _ in 0..20 {
        now += Duration::from_millis(16);
        ui.event(UiEvent::Timer(now));
        prepare_headless(&mut ui, now);
    }
    assert_eq!(lit(&ui), vec![7]);
}
