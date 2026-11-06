//! Listes virtualisées : lignes bornées, découpage par le clip parent, clavier,
//! pointeur et validation.
use crate::ui::tests::support::*;
use crate::ui::virtual_list::VirtualList;

#[derive(Default)]
struct Probes {
    live: Cell<usize>,
    peak: Cell<usize>,
    builds: Cell<usize>,
    paints: Cell<usize>,
}
struct Probe {
    counters: Rc<Probes>,
}
impl Probe {
    fn new(counters: Rc<Probes>) -> Self {
        counters.live.set(counters.live.get() + 1);
        counters
            .peak
            .set(counters.peak.get().max(counters.live.get()));
        counters.builds.set(counters.builds.get() + 1);
        Self { counters }
    }
}
impl Drop for Probe {
    fn drop(&mut self) {
        self.counters.live.set(self.counters.live.get() - 1);
    }
}
impl Widget for Probe {
    fn measure(&self, available: SizeF, text: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
        Text::new("Ligne").measure(available, text)
    }
    fn paint(&self, ctx: &PaintContext, scene: &mut Scene) {
        self.counters.paints.set(self.counters.paints.get() + 1);
        Text::new("Ligne").paint(ctx, scene);
    }
}
fn probes(count: usize, counters: Rc<Probes>) -> Element {
    Element::virtual_list(
        1,
        VirtualList::new(count, 32., move |index| {
            Element::custom(100 + index as u64, Probe::new(counters.clone()))
        }),
    )
}

#[test]
fn ten_thousand_items_keep_bounded_rows_and_reuse_the_overlap() {
    let counters = Rc::new(Probes::default());
    let mut ui = {
        let counters = counters.clone();
        view_runtime(move || probes(10_000, counters.clone()))
    };
    // Les mesures comptées sont celles que le moteur demande : sans la mémoire des
    // mesures de texte, qui servirait les lignes déjà mesurées.
    ui.set_incremental(false);
    let mut m = Measure::default();
    prepare(&mut ui, &mut m);
    assert_eq!(counters.builds.get(), 15);
    assert_eq!(counters.live.get(), 15);
    assert_eq!(ui.stats().virtual_rows, 15);
    assert!(counters.paints.get() <= 13);
    assert!(ui.root().unwrap().find(ElementId(9100)).is_none());
    let before = ui.stats();
    let calls = m.calls;
    let p = PointF { x: 20., y: 70. };
    wheel(&mut ui, p, 0., 0.25);
    prepare(&mut ui, &mut m);
    assert_eq!(counters.builds.get(), 15);
    assert_eq!(m.calls, calls);
    wheel(&mut ui, p, 0., 32.);
    prepare(&mut ui, &mut m);
    assert_eq!(counters.builds.get(), 16);
    assert_eq!(m.calls, calls + 1);
    for i in 0..100 {
        counters.paints.set(0);
        wheel(
            &mut ui,
            p,
            0.,
            if i % 2 == 0 {
                10_000_000.
            } else {
                -10_000_000.
            },
        );
        prepare(&mut ui, &mut m);
        assert!(counters.live.get() <= 18);
        assert!(counters.peak.get() <= 18);
        assert!(counters.paints.get() <= 14);
        assert!(ui.scene().commands.len() < 100);
    }
    assert_eq!(ui.stats().views, before.views);
    assert_eq!(ui.stats().layouts, before.layouts);
    assert!(ui.scene().validate().is_ok());
    wheel(&mut ui, p, 0., 10_000_000.);
    prepare(&mut ui, &mut m);
    let list = ui.root().unwrap();
    let metrics = list.virtual_metrics().unwrap();
    assert_eq!(metrics.realized_range.end, 10_000);
    assert!(list.find(ElementId(10_099)).is_some());
    let last = list.children.last().unwrap().bounds();
    near(last.bottom, scroll_metrics(&ui, 1).viewport.bottom);
    drop(ui);
    assert_eq!(counters.live.get(), 0);
}

#[test]
fn parent_clipping_materializes_only_the_exposed_slice_and_releases_hidden_rows() {
    let counters = Rc::new(Probes::default());
    let mut ui = {
        let counters = counters.clone();
        view_runtime(move || {
            Element::scroll(
                10,
                Axis::Vertical,
                Element::column(
                    11,
                    vec![
                        box_node(12, Length::Fill(1.), Length::Px(600.)),
                        probes(10_000, counters.clone()).height(Length::Px(200.)),
                        box_node(13, Length::Fill(1.), Length::Px(600.)),
                    ],
                ),
            )
        })
    };
    let mut m = Measure::default();
    prepare(&mut ui, &mut m);
    assert_eq!(counters.live.get(), 0);
    assert_eq!(counters.builds.get(), 0);
    let p = PointF { x: 20., y: 70. };
    wheel(&mut ui, p, 0., 300.);
    prepare(&mut ui, &mut m);
    assert!((1..=6).contains(&counters.live.get()));
    wheel(&mut ui, p, 0., 300.);
    prepare(&mut ui, &mut m);
    assert!(counters.live.get() <= 9);
    wheel(&mut ui, p, 0., -600.);
    prepare(&mut ui, &mut m);
    assert_eq!(counters.live.get(), 0);
    wheel(&mut ui, p, 0., 600.);
    prepare(&mut ui, &mut m);
    assert!(counters.live.get() > 0);
}

struct InteractiveRows {
    count: Rc<Cell<usize>>,
    last: Rc<Cell<u64>>,
    activations: Rc<Cell<usize>>,
}
impl Application for InteractiveRows {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        Element::column(
            1,
            vec![
                widgets::button(2, "Avant", Msg::Click(1)),
                Element::virtual_list(
                    3,
                    VirtualList::new(self.count.get(), 32., |index| {
                        widgets::button(
                            100 + index as u64,
                            format!("Ligne {index}"),
                            Msg::Click(1000 + index as u64),
                        )
                    }),
                )
                .height(Length::Px(200.)),
                widgets::button(4, "Après", Msg::Click(2)),
            ],
        )
    }
    fn update(&mut self, message: Msg) -> bool {
        let Msg::Click(id) = message else {
            panic!("unexpected message {message:?}");
        };
        self.last.set(id);
        self.activations.set(self.activations.get() + 1);
        true
    }
}
fn interactive(count: Rc<Cell<usize>>) -> (UiRuntime, Rc<Cell<u64>>, Rc<Cell<usize>>) {
    let last = Rc::new(Cell::new(0));
    let activations = Rc::new(Cell::new(0));
    let ui = UiRuntime::new(
        InteractiveRows {
            count,
            last: last.clone(),
            activations: activations.clone(),
        },
        &NoResources,
    )
    .unwrap();
    (ui, last, activations)
}
#[test]
fn list_is_one_tab_stop_and_keyboard_reaches_and_activates_the_last_item() {
    let (mut ui, last, activations) = interactive(Rc::new(Cell::new(10_000)));
    let mut m = Measure::default();
    prepare(&mut ui, &mut m);
    key(&mut ui, Key::Tab);
    assert_eq!(ui.focused(), Some(ElementId(2)));
    key(&mut ui, Key::Tab);
    assert_eq!(ui.focused(), Some(ElementId(3)));
    key(&mut ui, Key::End);
    prepare(&mut ui, &mut m);
    assert_eq!(
        ui.root()
            .unwrap()
            .find(ElementId(3))
            .unwrap()
            .virtual_metrics()
            .unwrap()
            .active_index,
        Some(9999)
    );
    assert!(ui.stats().virtual_rows <= 12);
    key(&mut ui, Key::Enter);
    prepare(&mut ui, &mut m);
    assert_eq!(last.get(), 10_999);
    assert_eq!(activations.get(), 1);
    assert_eq!(ui.focused(), Some(ElementId(3)));
    ui.event(UiEvent::KeyDown {
        key: Key::Enter,
        shift: false,
        repeat: true,
    });
    assert_eq!(activations.get(), 1);
    key(&mut ui, Key::Space);
    assert_eq!(activations.get(), 1);
    ui.event(UiEvent::KeyUp(Key::Space));
    prepare(&mut ui, &mut m);
    assert_eq!(activations.get(), 2);
    // Défiler ailleurs garde au plus une ligne active hors champ, sans perdre l'action clavier.
    let s = scroll_metrics(&ui, 3);
    wheel(
        &mut ui,
        PointF {
            x: s.viewport.left + 5.,
            y: s.viewport.top + 5.,
        },
        0.,
        -10_000_000.,
    );
    prepare(&mut ui, &mut m);
    assert!(ui.stats().virtual_rows <= 12);
    assert!(ui.root().unwrap().find(ElementId(10_099)).is_some());
    key(&mut ui, Key::Enter);
    prepare(&mut ui, &mut m);
    assert_eq!(last.get(), 10_999);
    key(&mut ui, Key::Tab);
    assert_eq!(ui.focused(), Some(ElementId(4)));
    ui.event(UiEvent::KeyDown {
        key: Key::Tab,
        shift: true,
        repeat: false,
    });
    assert_eq!(ui.focused(), Some(ElementId(3)));
    key(&mut ui, Key::Home);
    prepare(&mut ui, &mut m);
    near(scroll_metrics(&ui, 3).offset, 0.);
    key(&mut ui, Key::Down);
    prepare(&mut ui, &mut m);
    key(&mut ui, Key::Enter);
    assert_eq!(last.get(), 1001);
    let before = activations.get();
    key(&mut ui, Key::Space);
    key(&mut ui, Key::Down);
    prepare(&mut ui, &mut m);
    ui.event(UiEvent::KeyUp(Key::Space));
    assert_eq!(activations.get(), before);
}

#[test]
fn pointer_activation_focus_and_clamped_selection_survive_rebuild_and_data_shrink() {
    let count = Rc::new(Cell::new(10_000));
    let (mut ui, last, _) = interactive(count.clone());
    let mut m = Measure::default();
    prepare(&mut ui, &mut m);
    let p = center(&ui, 101);
    assert!(ui.event(UiEvent::PointerDown(p)).capture);
    assert_eq!(ui.focused(), Some(ElementId(3)));
    assert!(ui.event(UiEvent::PointerUp(p)).release);
    prepare(&mut ui, &mut m);
    assert_eq!(last.get(), 1001);
    key(&mut ui, Key::End);
    prepare(&mut ui, &mut m);
    let offset = scroll_metrics(&ui, 3).offset;
    ui.invalidate();
    prepare(&mut ui, &mut m);
    near(scroll_metrics(&ui, 3).offset, offset);
    count.set(2);
    ui.invalidate();
    prepare(&mut ui, &mut m);
    near(scroll_metrics(&ui, 3).offset, 0.);
    assert_eq!(
        ui.root()
            .unwrap()
            .find(ElementId(3))
            .unwrap()
            .virtual_metrics()
            .unwrap()
            .active_index,
        Some(1)
    );
    count.set(10_000);
    ui.invalidate();
    prepare(&mut ui, &mut m);
    key(&mut ui, Key::Enter);
    assert_eq!(last.get(), 1001);
    prepare(&mut ui, &mut m);
    let held = center(&ui, 101);
    assert!(ui.event(UiEvent::PointerDown(held)).capture);
    count.set(0);
    ui.invalidate();
    prepare(&mut ui, &mut m);
    assert_eq!(ui.stats().virtual_rows, 0);
    assert!(!ui.has_capture());
    assert!(ui.event(UiEvent::PointerUp(held)).release);
    key(&mut ui, Key::End);
    key(&mut ui, Key::Enter);
    assert_eq!(last.get(), 1001);
}

#[test]
fn invalid_dimensions_duplicate_ids_and_recursive_lazy_lists_are_rejected() {
    for height in [0., -1., f32::NAN, f32::INFINITY] {
        let node = Element::virtual_list(
            1,
            VirtualList::new(10, height, |_| panic!("ne doit pas construire")),
        );
        assert!(crate::error::rejected(layout::validate(&node)));
    }
    assert!(crate::error::rejected(layout::validate(
        &Element::virtual_list(
            1,
            VirtualList::new(usize::MAX, 32., |_| Element::column(2, vec![]))
        )
    )));
    assert!(crate::error::rejected(layout::validate(
        &Element::virtual_list(
            1,
            VirtualList::new(10, 32., |_| Element::column(2, vec![])).overscan(100)
        )
    )));
    for kind in 0..3 {
        let mut ui = view_runtime(move || {
            Element::virtual_list(
                1,
                VirtualList::new(10, 32., move |index| match kind {
                    0 => widgets::text(1, "ID du parent"),
                    1 => widgets::text(2, "ID dupliqué entre lignes"),
                    _ => Element::virtual_list(
                        100 + index as u64,
                        VirtualList::new(10, 32., |_| widgets::text(999, "Récursif")),
                    ),
                }),
            )
        });
        assert!(
            ui.prepare(context(), ChromeState::default(), &mut Measure::default())
                .is_err()
        );
    }
    let mut empty = view_runtime(|| {
        Element::virtual_list(1, VirtualList::new(0, 32., |_| panic!("liste vide")))
    });
    prepare(&mut empty, &mut Measure::default());
    assert_eq!(empty.stats().virtual_rows, 0);
}

#[test]
fn viewport_resize_is_bounded_and_zero_size_releases_unfocused_rows() {
    let counters = Rc::new(Probes::default());
    let mut ui = {
        let counters = counters.clone();
        view_runtime(move || probes(10_000, counters.clone()))
    };
    let mut m = Measure::default();
    prepare(&mut ui, &mut m);
    ui.prepare(
        ViewContext {
            width: 350.,
            height: 800.,
            ..context()
        },
        ChromeState::default(),
        &mut m,
    )
    .unwrap();
    assert!(counters.live.get() <= 27);
    ui.prepare(
        ViewContext {
            width: 0.,
            height: 0.,
            ..context()
        },
        ChromeState::default(),
        &mut m,
    )
    .unwrap();
    assert_eq!(counters.live.get(), 0);
    assert!(ui.scene().validate().is_ok());
    prepare(&mut ui, &mut m);
    assert_eq!(counters.live.get(), 15);
}
