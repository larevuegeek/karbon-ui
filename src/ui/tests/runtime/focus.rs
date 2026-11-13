//! Focus et navigation : ordre de tabulation à travers les zones défilantes, révélation
//! du contrôle focalisé et commandes d'accessibilité.
use crate::ui::tests::support::*;

#[test]
fn tab_reveals_clipped_controls_in_both_directions() {
    let mut ui = view_runtime(|| {
        Element::scroll(
            1,
            Axis::Vertical,
            Element::column(
                2,
                vec![
                    widgets::button(3, "Début", Msg::Click(1)),
                    box_node(4, Length::Fill(1.), Length::Px(1000.)),
                    widgets::button(5, "Fin", Msg::Click(2)),
                ],
            ),
        )
    });
    let mut m = Measure::default();
    prepare(&mut ui, &mut m);
    // La zone défilante contient des boutons : elle n'est pas une étape de Tab (Chromium).
    key(&mut ui, Key::Tab);
    assert_eq!(ui.focused(), Some(ElementId(3)));
    key(&mut ui, Key::Tab);
    assert_eq!(ui.focused(), Some(ElementId(5)));
    let end = ui.root().unwrap().find(ElementId(5)).unwrap();
    assert!(end.bounds().bottom <= scroll_metrics(&ui, 1).viewport.bottom + 0.01);
    ui.event(UiEvent::KeyDown {
        key: Key::Tab,
        shift: true,
        repeat: false,
    });
    near(scroll_metrics(&ui, 1).offset, 0.);
    assert_eq!(ui.focused(), Some(ElementId(3)));
}

#[test]
fn scroll_areas_without_focusable_content_remain_tab_stops() {
    let mut ui = view_runtime(|| {
        Element::column(
            9,
            vec![
                Element::scroll(
                    1,
                    Axis::Vertical,
                    box_node(2, Length::Fill(1.), Length::Px(1000.)),
                ),
                widgets::button(3, "Après", Msg::Click(3)),
            ],
        )
    });
    prepare(&mut ui, &mut Measure::default());
    key(&mut ui, Key::Tab);
    assert_eq!(ui.focused(), Some(ElementId(1)));
    key(&mut ui, Key::Tab);
    assert_eq!(ui.focused(), Some(ElementId(3)));
}

#[test]
fn accessibility_invokes_buttons_and_reveals_focus_in_a_scroll_parent() {
    use crate::ui::runtime::AccessibilityAction as Action;
    let count = Rc::new(Cell::new(0));
    let mut ui = UiRuntime::new(
        Counter {
            value: count.clone(),
        },
        &NoResources,
    )
    .unwrap();
    prepare(&mut ui, &mut Measure::default());
    ui.accessibility_action(Action::Invoke(ElementId(10)))
        .unwrap();
    assert_eq!(count.get(), 1);
    let mut ui = view_runtime(|| {
        Element::scroll(
            1,
            Axis::Vertical,
            Element::column(
                2,
                vec![
                    widgets::text(3, "Espace").height(Length::Px(1000.)),
                    widgets::button(4, "En bas", Msg::Click(1)),
                ],
            ),
        )
    });
    let mut m = Measure::default();
    prepare(&mut ui, &mut m);
    let snapshot = ui.accessibility();
    let bottom = snapshot.iter().find(|n| n.id == ElementId(4)).unwrap();
    assert!(bottom.offscreen);
    assert_eq!(bottom.parent, Some(ElementId(1)));
    ui.accessibility_action(Action::Focus(ElementId(4)))
        .unwrap();
    prepare(&mut ui, &mut m);
    let snapshot = ui.accessibility();
    let bottom = snapshot.iter().find(|n| n.id == ElementId(4)).unwrap();
    assert!(bottom.focused);
    assert!(!bottom.offscreen);
}

/// Séparateur d'un `Split` redimensionnable dont un volet est masqué : pas de séparateur
/// dessiné, donc ni place dans l'ordre de tabulation ni nœud d'accessibilité.
#[test]
fn a_split_with_a_hidden_pane_has_no_separator_to_reach() {
    let shown = Rc::new(Cell::new(false));
    let toggle = shown.clone();
    let mut ui = view_runtime(move || {
        let mut first = Element::column(2, vec![]);
        first.layout_rules().hidden = !toggle.get();
        Element::column(
            10,
            vec![
                widgets::button(5, "Avant", Msg::Click(5)),
                Element::split(
                    1,
                    Split::horizontal(0.5).resizable(true),
                    first,
                    Element::column(3, vec![widgets::button(4, "Après", Msg::Click(4))]),
                ),
            ],
        )
    });
    let mut m = Measure::default();
    let tour = |ui: &mut UiRuntime, m: &mut Measure| {
        let mut seen = Vec::new();
        for _ in 0..3 {
            key(ui, Key::Tab);
            prepare(ui, m);
            seen.push(ui.focused().unwrap().0);
        }
        let separator = ui.accessibility().iter().any(|n| n.role == Role::Separator);
        (seen, separator)
    };
    prepare(&mut ui, &mut m);
    assert_eq!(tour(&mut ui, &mut m), (vec![5, 4, 5], false));
    shown.set(true);
    ui.invalidate();
    prepare(&mut ui, &mut m);
    assert_eq!(tour(&mut ui, &mut m), (vec![1, 4, 5], true));
}
