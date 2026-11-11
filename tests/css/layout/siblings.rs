//! Lignes mémorisées (`memo`) dont une règle lit le frère précédent (`.ligne + .ligne`,
//! et sa boîte `::after`) : leurs styles sont repris tant que ce frère ne change pas, et
//! la même suite d'interactions (lignes ajoutées en fin, lâchées en tête, frère qui
//! change de classe, survol) donne avec et sans les mémoires du moteur le même arbre
//! placé, les mêmes styles et les mêmes pixels.
use karbon_ui::{
    platform::headless::{HeadlessApp, HeadlessOptions},
    ui::{prelude::*, runtime::UiEvent, scene::PointF},
};
use std::time::Duration;
css_file!(mod css, "siblings.css");

/// Lignes `start..end` ; `marked` : la ligne qui n'a pas la classe `ligne`.
struct List {
    start: usize,
    end: usize,
    marked: Option<usize>,
}

impl Application for List {
    type Message = ();
    fn view(&self, _: &ViewContext) -> Element {
        let cx = TemplateContext::default();
        let rows: Vec<(usize, bool)> = (self.start..self.end)
            .map(|i| (i, self.marked != Some(i)))
            .collect();
        ui! { @context(cx); @styles(css);
            <div class="list">
                {#each rows as row (row.0)}
                    {#if row.1}
                        <div class="ligne" memo={row}><p class="nom">{format!("Ligne {}", row.0)}</p></div>
                    {:else}
                        <div class="marque" memo={row}><p class="nom">{format!("Marque {}", row.0)}</p></div>
                    {/if}
                {/each}
            </div>
        }
    }
    fn update(&mut self, _: ()) -> bool {
        false
    }
}

fn snapshot(app: &mut HeadlessApp<List>, pixels: bool) -> Vec<String> {
    fn walk(node: &Element, out: &mut Vec<String>) {
        let b = node.bounds();
        out.push(format!(
            "{:?} {:?} {:?}",
            node.id,
            [b.left, b.top, b.right, b.bottom].map(f32::to_bits),
            node.style.background,
        ));
        for child in &node.children {
            walk(child, out);
        }
    }
    let mut out = Vec::new();
    walk(app.root().unwrap(), &mut out);
    if pixels {
        out.push(format!("{:?}", app.render(1.).unwrap().pixels));
    }
    out
}

fn play(incremental: bool) -> (Vec<Vec<String>>, u64) {
    let mut app = HeadlessApp::with_options(
        List {
            start: 0,
            end: 12,
            marked: None,
        },
        HeadlessOptions {
            width: 320.,
            height: 400.,
            ..Default::default()
        },
    )
    .unwrap();
    app.runtime_mut().set_incremental(incremental);
    app.prepare().unwrap();
    let mut steps = vec![snapshot(&mut app, true)];
    let mut views = 0;
    // Lignes ajoutées en fin, puis lâchées en tête (liste chargée par pages).
    for (start, end) in [(0, 15), (0, 18), (3, 18), (6, 21), (6, 24)] {
        app.update_app(|list| {
            list.start = start;
            list.end = end;
        })
        .unwrap();
        views += 1;
        steps.push(snapshot(&mut app, true));
    }
    // Le frère précédent d'une ligne perd sa classe, puis la retrouve.
    for marked in [Some(9), Some(10), None] {
        app.update_app(|list| list.marked = marked).unwrap();
        steps.push(snapshot(&mut app, true));
    }
    // Survol de ligne en ligne pendant des reconstructions.
    for i in 0..8 {
        app.event(UiEvent::PointerMove(PointF {
            x: 100.,
            y: 20. + 34. * i as f32,
        }))
        .unwrap();
        app.update_app(|_| {}).unwrap();
        app.advance(Duration::from_millis(8)).unwrap();
        steps.push(snapshot(&mut app, i % 3 == 0));
    }
    (steps, views)
}

#[test]
fn rows_reading_their_previous_sibling_match_a_full_rebuild() {
    let (full, _) = play(false);
    let (memo, _) = play(true);
    assert_eq!(full.len(), memo.len());
    for (step, (a, b)) in full.iter().zip(&memo).enumerate() {
        assert_eq!(a, b, "étape {step}");
    }
}
