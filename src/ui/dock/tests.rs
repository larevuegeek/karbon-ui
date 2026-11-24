//! Zone ancrée : arbre (poser, retirer, groupes vidés), dix déplacements puis sauvegarde
//! et relecture, onglet tiré dans la fenêtre simulée sur le côté d'un autre groupe,
//! marque du dépôt, clic sur un onglet, croix, séparation réglée.
use super::*;
use crate::{
    platform::headless::{HeadlessApp, RawModifiers},
    ui::{
        element::{Axis, ElementId},
        prelude::*,
        runtime::{PointerButton, PointerPhase, UiEvent},
        scene::{DrawCommand, PointF},
    },
};

/// 1 | (2 / 3) : un groupe à gauche, deux empilés à droite.
fn three() -> DockTree {
    let mut tree = DockTree::single(1);
    assert!(tree.dock(2, 1, DockSide::Right));
    assert!(tree.dock(3, 2, DockSide::Bottom));
    tree
}

#[test]
fn docking_builds_splits_and_an_emptied_group_disappears() {
    let mut tree = three();
    assert_eq!(tree.panels(), [1, 2, 3]);
    let Some(DockNode::Split { axis, second, .. }) = tree.root() else {
        panic!("split at the root");
    };
    assert_eq!(*axis, Axis::Horizontal);
    assert!(matches!(
        **second,
        DockNode::Split {
            axis: Axis::Vertical,
            ..
        }
    ));
    // 3 parmi les onglets de 1 : son groupe vidé disparaît, avec sa séparation.
    assert!(tree.dock(3, 1, DockSide::Center));
    assert_eq!(tree.active_of(1), Some(3));
    let Some(DockNode::Split { second, .. }) = tree.root() else {
        panic!("split at the root");
    };
    assert_eq!(**second, DockNode::tab(2));
    // Témoins : un onglet seul ne se pose pas à côté de lui-même ; une cible absente.
    assert!(!tree.dock(2, 2, DockSide::Left));
    assert!(!tree.dock(4, 9, DockSide::Left));
    // Un onglet tiré de son groupe vers un côté de ce groupe le partage.
    assert!(tree.dock(3, 3, DockSide::Top));
    assert_eq!(tree.panels(), [3, 1, 2]);
    assert!(tree.remove(1) && tree.remove(2) && tree.remove(3));
    assert_eq!(tree.root(), None);
    assert_eq!(DockTree::from_json(&tree.to_json()).unwrap(), tree);
}

#[test]
fn ten_moves_then_saving_and_reading_give_the_same_tree() {
    let mut tree = DockTree::single(1);
    for panel in 2..=5 {
        tree.dock(panel, panel - 1, DockSide::Right);
    }
    let moves = [
        (1, 5, DockSide::Bottom),
        (2, 1, DockSide::Center),
        (3, 2, DockSide::Left),
        (4, 3, DockSide::Top),
        (5, 4, DockSide::Center),
        (1, 4, DockSide::Right),
        (2, 2, DockSide::Bottom),
        (3, 1, DockSide::Center),
        (4, 5, DockSide::Left),
        (5, 3, DockSide::Top),
    ];
    for (panel, target, side) in moves {
        tree.dock(panel, target, side);
        let mut panels = tree.panels();
        panels.sort_unstable();
        assert_eq!(panels, [1, 2, 3, 4, 5], "after {panel} → {target}");
    }
    assert!(tree.set_fraction(split_key(&tree), 0.3125));
    let text = tree.to_json();
    let read = DockTree::from_json(&text).unwrap();
    assert_eq!(read, tree);
    assert_eq!(read.to_json(), text);
    // Témoins : un texte abîmé, un panneau en double.
    assert!(DockTree::from_json("{\"tabs\":[]}").is_err());
    assert!(DockTree::from_json(
        "{\"split\":\"h\",\"fraction\":0.5,\"first\":{\"tabs\":[1],\"active\":1},\"second\":{\"tabs\":[1],\"active\":1}}"
    )
    .is_err());
}

/// La clé de la séparation à la racine.
fn split_key(tree: &DockTree) -> (u64, u64) {
    match tree.root() {
        Some(DockNode::Split { first, second, .. }) => (first.first_panel(), second.first_panel()),
        _ => panic!("split at the root"),
    }
}

struct Editor {
    dock: DockState,
    events: Vec<DockEvent>,
}

impl Application for Editor {
    type Message = DockMsg;
    fn view(&self, _: &ViewContext) -> Element {
        DockArea::new(
            "dock",
            &self.dock,
            |m| m,
            |panel| format!("Panneau {panel}"),
            |panel| column(ElementId::named("panel").item(panel), []),
        )
        .closable(true)
        .element()
    }
    fn update(&mut self, message: DockMsg) -> bool {
        let reply = self.dock.update(message);
        self.events.extend(reply.event);
        reply.rebuild
    }
}

fn editor(tree: DockTree) -> HeadlessApp<Editor> {
    HeadlessApp::with_options(
        Editor {
            dock: DockState::new(tree),
            events: Vec::new(),
        },
        crate::platform::headless::HeadlessOptions {
            width: 800.,
            height: 600.,
            ..Default::default()
        },
    )
    .unwrap()
}

fn mouse(app: &mut HeadlessApp<Editor>, phase: PointerPhase, p: PointF) {
    let event = match phase {
        PointerPhase::Down => UiEvent::PointerDown(p),
        PointerPhase::Up => UiEvent::PointerUp(p),
        _ => UiEvent::PointerMove(p),
    };
    app.event(event).unwrap();
    let button = (phase != PointerPhase::Move).then_some(PointerButton::Primary);
    app.raw_pointer(phase, button, p, RawModifiers::default())
        .unwrap();
}

fn tab(panel: u64) -> ElementId {
    ElementId::from("dock").child("tab").item(panel)
}

fn point_in(app: &HeadlessApp<Editor>, id: ElementId, fx: f32, fy: f32) -> PointF {
    let r = app.bounds(id).expect("on screen");
    PointF {
        x: r.left + (r.right - r.left) * fx,
        y: r.top + (r.bottom - r.top) * fy,
    }
}

fn marks(app: &HeadlessApp<Editor>) -> usize {
    app.scene()
        .commands
        .iter()
        .filter(|c| matches!(c, DrawCommand::Stroke { width, radius, .. } if *width == 2. && *radius == 2.))
        .count()
}

#[test]
fn a_tab_dragged_onto_the_left_quarter_of_another_group_docks_there() {
    let mut app = editor(three());
    let panel3 = ElementId::named("panel").item(3);
    let start = point_in(&app, tab(1), 0.3, 0.5);
    mouse(&mut app, PointerPhase::Down, start);
    let aim = point_in(&app, panel3, 0.1, 0.5);
    mouse(&mut app, PointerPhase::Move, aim);
    assert_eq!(app.app().dock.aim(), Some((3, DockSide::Left)));
    assert_eq!(marks(&app), 1, "the drop zone is shown");
    mouse(&mut app, PointerPhase::Up, aim);
    assert_eq!(
        app.app().events.last(),
        Some(&DockEvent::Moved {
            panel: 1,
            target: 3,
            side: DockSide::Left
        })
    );
    assert_eq!(app.app().dock.tree().panels(), [2, 1, 3]);
    assert_eq!(app.app().dock.aim(), None);
    assert_eq!(marks(&app), 0);
    // Les panneaux sont à leur place : 1 à gauche de 3, sous 2.
    let (one, three) = (
        app.bounds(ElementId::named("panel").item(1)).unwrap(),
        app.bounds(panel3).unwrap(),
    );
    assert!(one.right <= three.left && (one.top - three.top).abs() < 1.);
}

#[test]
fn a_tab_released_where_it_started_does_not_move_and_a_click_activates() {
    let mut app = editor(DockTree::from_node(DockNode::tabs(vec![1, 2], 1)));
    // Clic visé par la position : l'onglet 2 se montre.
    let point = point_in(&app, tab(2), 0.3, 0.5);
    app.click_at(point).unwrap();
    assert_eq!(app.app().dock.tree().active_of(1), Some(2));
    assert!(app.find(ElementId::named("panel").item(2)).is_some());
    // Témoin : un glisser lâché sur sa propre rangée d'onglets ne change rien.
    let before = app.app().dock.tree().clone();
    mouse(&mut app, PointerPhase::Down, point);
    let beside = PointF {
        x: point.x + 30.,
        ..point
    };
    mouse(&mut app, PointerPhase::Move, beside);
    mouse(&mut app, PointerPhase::Up, beside);
    assert_eq!(app.app().dock.tree(), &before);
    // La croix ferme le panneau.
    let close = point_in(&app, tab(2).child("close"), 0.5, 0.5);
    app.click_at(close).unwrap();
    assert_eq!(app.app().events.last(), Some(&DockEvent::Closed(2)));
    assert_eq!(app.app().dock.tree().panels(), [1]);
}

#[test]
fn moving_a_divider_reports_its_share_to_the_tree() {
    let mut app = editor(three());
    let left = app.bounds(ElementId::named("panel").item(1)).unwrap();
    let right = app.bounds(ElementId::named("panel").item(2)).unwrap();
    let x = (left.right + right.left) / 2.;
    let y = (left.top + left.bottom) / 2.;
    app.event(UiEvent::PointerMove(PointF { x, y })).unwrap();
    app.event(UiEvent::PointerDown(PointF { x, y })).unwrap();
    app.event(UiEvent::PointerMove(PointF { x: 200., y }))
        .unwrap();
    app.event(UiEvent::PointerUp(PointF { x: 200., y }))
        .unwrap();
    let Some(DockNode::Split { fraction, .. }) = app.app().dock.tree().root() else {
        panic!("split at the root");
    };
    assert!((*fraction - 0.25).abs() < 0.02, "share {fraction}");
    assert!(app.app().events.contains(&DockEvent::Resized));
}
