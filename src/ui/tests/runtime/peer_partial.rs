//! Résolution partielle des styles au passage du pointeur (KUI-057) : la cascade ne
//! parcourt plus tous les enfants d'un élément du chemin, seulement ceux qu'elle peut
//! avoir à refaire ; une règle `peer-hover` (qui lit le frère précédent) suit toujours le
//! survol, comme une résolution complète.
use crate::{
    platform::headless::{HeadlessApp, HeadlessOptions},
    ui::{
        prelude::*,
        rendering::filter::Filters,
        runtime::UiEvent,
        scene::{DrawCommand, PointF, RectF, signature::Signature},
    },
};

/// Un remplissage bleu : la règle de frère a pris.
fn blue(commands: &[DrawCommand]) -> bool {
    commands.iter().any(|c| {
        matches!(c, DrawCommand::Fill { color, .. } | DrawCommand::Shape { fill: Some(crate::ui::rendering::brush::Background::Solid(color)), .. }
            if color.blue > 200 && color.red < 50)
    })
}

crate::css_file!(mod css, "peer_partial.css");

struct Peers;

impl Application for Peers {
    type Message = ();
    fn view(&self, _: &ViewContext) -> Element {
        crate::ui! {
            @styles(css);
            <div class="list">
                {#each 0..40 as row (row)}
                    <div class="row"></div>
                    <div class="next"></div>
                {/each}
            </div>
        }
    }
    fn update(&mut self, _: ()) -> bool {
        false
    }
}

/// Les scènes après chaque mouvement du pointeur, mémoires actives ou non.
fn scenes(incremental: bool) -> Vec<(u64, bool)> {
    let mut app = HeadlessApp::with_options(
        Peers,
        HeadlessOptions {
            width: 300.,
            height: 800.,
            ..Default::default()
        },
    )
    .unwrap();
    app.runtime_mut().set_incremental(incremental);
    app.prepare().unwrap();
    (0..30)
        .map(|step| {
            let y = 2. + (step * 37 % 640) as f32;
            app.runtime_mut()
                .event(UiEvent::PointerMove(PointF { x: 100., y }));
            app.prepare().unwrap();
            let commands = &app.scene().commands;
            let digest =
                Signature::new(commands, RectF::new(0., 0., 0., 0.), Filters::new(&[]), 0.)
                    .digest();
            (digest, blue(commands))
        })
        .collect()
}

#[test]
fn a_peer_rule_follows_the_pointer_as_in_a_full_resolution() {
    let partial = scenes(true);
    let full = scenes(false);
    // Le survol d'une ligne colore bien la suivante (`.row:hover + .next`).
    assert!(full.iter().filter(|(_, blue)| *blue).count() > 10);
    for (step, (a, b)) in partial.iter().zip(&full).enumerate() {
        assert_eq!(a, b, "mouvement {step}");
    }
}
