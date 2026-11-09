//! Sous-arbres mémorisés (`view::lazy`, attribut `memo` des gabarits) : vue sautée,
//! sous-arbre repris, et même placement qu'une vue reconstruite en entier.
use crate::ui::prelude::{Animation, Modal};
use crate::ui::tests::support::*;
use crate::ui::view::lazy;

/// Liste défilante de 40 lignes de 50 px ; chaque ligne (`100 + n`, texte `1000 + n`) est
/// mémorisée sur `(n, version)` si `memo`. `built` compte les lignes construites.
fn list(memo: bool, version: Rc<Cell<u64>>, built: Rc<Cell<usize>>) -> UiRuntime {
    view_runtime(move || {
        let v = version.get();
        let rows = (0..40u64)
            .map(|n| {
                let build = || {
                    built.set(built.get() + 1);
                    Element::row(
                        100 + n,
                        vec![widgets::text(1000 + n, format!("Ligne {n} · {}", v % 3))],
                    )
                    .height(Length::Px(50.))
                    .padding(6.)
                };
                if memo {
                    // La ligne 7 lit `version` : elle change avec elle.
                    lazy(100 + n, &(n, if n == 7 { v } else { 0 }), build)
                } else {
                    build()
                }
            })
            .collect();
        Element::scroll(1, Axis::Vertical, Element::column(2, rows))
    })
}
fn rects(ui: &UiRuntime) -> Vec<(u64, [f32; 8])> {
    fn walk(node: &Element, out: &mut Vec<(u64, [f32; 8])>) {
        let (r, c) = (node.rect, node.clip);
        out.push((
            node.id.0,
            [
                r.left, r.top, r.right, r.bottom, c.left, c.top, c.right, c.bottom,
            ],
        ));
        for child in &node.children {
            walk(child, out);
        }
    }
    let mut out = Vec::new();
    walk(ui.root().unwrap(), &mut out);
    out
}

#[test]
fn an_unchanged_key_reuses_the_subtree_without_building_it() {
    let version = Rc::new(Cell::new(0));
    let built = Rc::new(Cell::new(0));
    let mut ui = list(true, version.clone(), built.clone());
    let mut measure = Measure::default();
    prepare(&mut ui, &mut measure);
    assert_eq!(built.get(), 40);
    ui.invalidate();
    prepare(&mut ui, &mut measure);
    assert_eq!(built.get(), 40, "aucune ligne reconstruite");
    // La ligne qui lit `version` est la seule reconstruite.
    version.set(1);
    ui.invalidate();
    prepare(&mut ui, &mut measure);
    assert_eq!(built.get(), 41);
    let row = ui.root().unwrap().find(ElementId(107)).unwrap();
    assert_eq!(row.children.len(), 1);
}

#[test]
fn reused_subtrees_are_placed_like_a_full_rebuild() {
    // Même suite d'événements avec et sans mémoire : mêmes rectangles et mêmes clips.
    let run = |memo: bool| {
        let version = Rc::new(Cell::new(0));
        let mut ui = list(memo, version.clone(), Rc::new(Cell::new(0)));
        let mut measure = Measure::default();
        let mut shots = Vec::new();
        prepare(&mut ui, &mut measure);
        shots.push(rects(&ui));
        wheel(&mut ui, PointF { x: 30., y: 70. }, 0., 130.);
        ui.invalidate();
        prepare(&mut ui, &mut measure);
        shots.push(rects(&ui));
        version.set(2);
        ui.invalidate();
        prepare(&mut ui, &mut measure);
        shots.push(rects(&ui));
        wheel(&mut ui, PointF { x: 30., y: 70. }, 0., -45.);
        prepare(&mut ui, &mut measure);
        shots.push(rects(&ui));
        // Autre largeur : les lignes reprises sont mesurées et placées de nouveau.
        ui.prepare(
            ViewContext {
                width: 420.,
                ..context()
            },
            ChromeState::default(),
            &mut measure,
        )
        .unwrap();
        shots.push(rects(&ui));
        shots
    };
    let (memo, full) = (run(true), run(false));
    for (step, (a, b)) in memo.iter().zip(&full).enumerate() {
        for (x, y) in a.iter().zip(b) {
            assert_eq!(x, y, "étape {step}");
        }
        assert_eq!(a.len(), b.len(), "étape {step}");
    }
}

#[test]
fn without_incremental_memories_every_subtree_is_built() {
    let built = Rc::new(Cell::new(0));
    let mut ui = list(true, Rc::new(Cell::new(0)), built.clone());
    ui.set_incremental(false);
    let mut measure = Measure::default();
    prepare(&mut ui, &mut measure);
    ui.invalidate();
    prepare(&mut ui, &mut measure);
    assert_eq!(built.get(), 80);
}

#[test]
fn a_reused_subtree_clashing_with_a_new_element_is_rejected() {
    let clash = Rc::new(Cell::new(false));
    let toggle = clash.clone();
    let mut ui = view_runtime(move || {
        let reused = lazy(10, &0u8, || Element::row(10, vec![widgets::text(11, "A")]));
        let other = widgets::text(if toggle.get() { 11 } else { 12 }, "B");
        Element::column(1, vec![reused, other])
    });
    let mut measure = Measure::default();
    prepare(&mut ui, &mut measure);
    clash.set(true);
    ui.invalidate();
    let error = ui
        .prepare(context(), ChromeState::default(), &mut measure)
        .expect_err("identifiant en double");
    assert!(
        error.to_string().contains("duplicate element id"),
        "{error}"
    );
}

#[test]
fn a_reused_subtree_is_checked_against_its_new_ancestors() {
    let animated = Rc::new(Cell::new(false));
    let toggle = animated.clone();
    let mut ui = view_runtime(move || {
        let reused = lazy(10, &0u8, || {
            let mut fixed = widgets::text(11, "A");
            fixed.rules.get_or_insert_with(Default::default).position = Position::Fixed;
            Element::row(10, vec![fixed])
        });
        let wrapper = Element::column(2, vec![reused]);
        let wrapper = if toggle.get() {
            wrapper.animate(Animation::fade_in())
        } else {
            wrapper
        };
        Element::column(1, vec![wrapper])
    });
    let mut measure = Measure::default();
    prepare(&mut ui, &mut measure);
    animated.set(true);
    ui.invalidate();
    let error = ui
        .prepare(context(), ChromeState::default(), &mut measure)
        .expect_err("fixed sous un ancêtre animé");
    assert!(error.to_string().contains("position: fixed"), "{error}");
}

#[test]
fn a_memo_nested_in_a_rebuilt_memo_is_still_reused() {
    let version = Rc::new(Cell::new(0u64));
    let built = Rc::new(Cell::new(0));
    let (v, b) = (version.clone(), built.clone());
    let mut ui = view_runtime(move || {
        let inner = b.clone();
        lazy(10, &v.get(), move || {
            Element::column(
                10,
                vec![lazy(20, &0u8, || {
                    inner.set(inner.get() + 1);
                    Element::row(20, vec![widgets::text(21, "A")])
                })],
            )
        })
    });
    let mut measure = Measure::default();
    prepare(&mut ui, &mut measure);
    assert_eq!(built.get(), 1);
    version.set(1);
    ui.invalidate();
    prepare(&mut ui, &mut measure);
    assert_eq!(built.get(), 1, "le sous-arbre interne reste repris");
    assert!(ui.root().unwrap().find(ElementId(21)).is_some());
}

#[test]
fn a_subtree_declaring_a_dialog_is_rebuilt_so_the_dialog_stays_open() {
    let built = Rc::new(Cell::new(0));
    let counter = built.clone();
    let mut ui = view_runtime(move || {
        counter.set(counter.get() + 1);
        let region = lazy(10, &0u8, || {
            Element::column(
                10,
                vec![
                    widgets::text(11, "A"),
                    Element::modal(12, Modal::new("Dialogue"), widgets::text(13, "B")),
                ],
            )
        });
        Element::column(1, vec![region])
    });
    let mut measure = Measure::default();
    prepare(&mut ui, &mut measure);
    for _ in 0..3 {
        ui.invalidate();
        prepare(&mut ui, &mut measure);
        assert!(
            ui.root().unwrap().find(ElementId(13)).is_some(),
            "dialogue perdu"
        );
        assert!(ui.root().unwrap().find(ElementId(11)).is_some());
    }
}
