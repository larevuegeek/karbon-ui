//! Ce qu'on apporte à un arbre par un glisser libre (`TreeView::on_payload`) : la ligne
//! visée et la place (au-dessus, dedans, au-dessous), la marque dessinée ; et la
//! sélection posée par l'application (`TreeState::set_selection`).
use super::*;
use crate::{
    platform::headless::{HeadlessApp, RawModifiers},
    ui::{
        drag::{DragEvent, DragPhase, Payload},
        editor::{EditorPalette, label},
        prelude::*,
        runtime::{PointerButton, PointerPhase, UiEvent},
        scene::{DrawCommand, PointF},
    },
};

/// Trois racines, 1, 2, 3, sans enfants.
struct Flat;

impl TreeSource for Flat {
    fn children(&self, parent: Option<u64>) -> &[u64] {
        if parent.is_none() { &[1, 2, 3] } else { &[] }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Tree(TreeMsg),
    Aim(Option<(u64, DropPlace)>),
    Dropped(u64, u64, DropPlace),
}

struct Outline {
    tree: TreeState,
    accepts: bool,
    aim: Option<(u64, DropPlace)>,
    dropped: Vec<(u64, u64, DropPlace)>,
}

impl Application for Outline {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        let thumb = column("thumb", [])
            .width(Length::Px(60.))
            .height(Length::Px(40.))
            .drag_payload(Payload::new("model", "Chaise", 77_u64));
        let mut tree = TreeView::new("tree", &self.tree, Msg::Tree, |item| {
            label(item.scope.child("name"), "objet", &EditorPalette::DARK)
        })
        .payload_target(self.aim);
        if self.accepts {
            tree = tree.on_payload(|e: &DragEvent, key, place| match e.phase {
                DragPhase::Over if e.payload.is("model") => Some(Msg::Aim(Some((key, place)))),
                DragPhase::Leave => Some(Msg::Aim(None)),
                DragPhase::Drop => Some(Msg::Dropped(*e.payload.get::<u64>()?, key, place)),
                _ => None,
            });
        }
        column("page", [thumb, tree.element().height(Length::Px(200.))])
    }
    fn update(&mut self, message: Msg) -> bool {
        match message {
            Msg::Tree(m) => self.tree.update(m, &Flat).rebuild,
            Msg::Aim(aim) => {
                self.aim = aim;
                true
            }
            Msg::Dropped(payload, key, place) => {
                self.dropped.push((payload, key, place));
                self.aim = None;
                true
            }
        }
    }
}

fn outline(accepts: bool) -> HeadlessApp<Outline> {
    HeadlessApp::new(Outline {
        tree: TreeState::new(&Flat),
        accepts,
        aim: None,
        dropped: Vec::new(),
    })
    .unwrap()
}

fn mouse(app: &mut HeadlessApp<Outline>, phase: PointerPhase, p: PointF) {
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

/// Point de la ligne de `key` à `fraction` de sa hauteur.
fn on_row(app: &HeadlessApp<Outline>, key: u64, fraction: f32) -> PointF {
    let r = app.bounds(ElementId::from("tree").item(key)).unwrap();
    PointF {
        x: (r.left + r.right) / 2.,
        y: r.top + (r.bottom - r.top) * fraction,
    }
}

fn drag_to(app: &mut HeadlessApp<Outline>, points: &[PointF]) {
    mouse(app, PointerPhase::Down, PointF { x: 20., y: 20. });
    for &p in points {
        mouse(app, PointerPhase::Move, p);
    }
    mouse(app, PointerPhase::Up, *points.last().unwrap());
}

#[test]
fn a_payload_dropped_on_a_row_names_the_row_and_the_place() {
    let mut app = outline(true);
    // Le milieu de la ligne 2 : dedans ; la marque (un cadre) est dessinée pendant le survol.
    mouse(&mut app, PointerPhase::Down, PointF { x: 20., y: 20. });
    let middle = on_row(&app, 2, 0.5);
    mouse(&mut app, PointerPhase::Move, middle);
    assert_eq!(app.app().aim, Some((2, DropPlace::Into)));
    let framed = app
        .scene()
        .commands
        .iter()
        .any(|c| matches!(c, DrawCommand::Stroke { width, .. } if *width == 2.));
    assert!(framed);
    mouse(&mut app, PointerPhase::Up, middle);
    // Le haut de la ligne 3 : au-dessus ; le bas de la ligne 1 : au-dessous.
    let top = on_row(&app, 3, 0.1);
    drag_to(&mut app, &[top]);
    let bottom = on_row(&app, 1, 0.9);
    drag_to(&mut app, &[bottom]);
    assert_eq!(
        app.app().dropped,
        [
            (77, 2, DropPlace::Into),
            (77, 3, DropPlace::Before),
            (77, 1, DropPlace::After)
        ]
    );
    assert_eq!(app.app().aim, None);
}

#[test]
fn a_tree_without_on_payload_refuses_the_payload() {
    // Témoin : sans `on_payload`, les lignes ne sont pas des cibles.
    let mut app = outline(false);
    let middle = on_row(&app, 2, 0.5);
    drag_to(&mut app, &[middle]);
    assert!(app.app().dropped.is_empty());
    assert_eq!(app.app().aim, None);
}

#[test]
fn set_selection_mirrors_a_selection_made_elsewhere() {
    let mut tree = TreeState::new(&Flat);
    tree.set_selection([1, 3], Some(3));
    let mut selected: Vec<u64> = tree.selection().iter().copied().collect();
    selected.sort_unstable();
    assert_eq!(selected, [1, 3]);
    assert_eq!(tree.cursor(), Some(3));
    // Maj+flèche part de l'ancre posée : 3 puis 2.
    let up = crate::ui::runtime::KeyPress {
        shift: true,
        ..crate::ui::runtime::KeyPress::new(crate::ui::runtime::KeyCode::Up, true)
    };
    tree.key(up, &Flat);
    let mut selected: Vec<u64> = tree.selection().iter().copied().collect();
    selected.sort_unstable();
    assert_eq!(selected, [2, 3]);
    tree.set_selection([], None);
    assert!(tree.selection().is_empty() && tree.cursor().is_none());
}

#[test]
fn narrator_reads_the_name_level_place_and_fold_of_a_row() {
    struct Two;
    impl TreeSource for Two {
        fn children(&self, parent: Option<u64>) -> &[u64] {
            match parent {
                None => &[1, 5],
                Some(1) => &[2, 3],
                Some(_) => &[],
            }
        }
    }
    struct Named {
        tree: TreeState,
        names: bool,
    }
    impl Application for Named {
        type Message = TreeMsg;
        fn view(&self, _: &ViewContext) -> Element {
            let mut view = TreeView::new(
                "tree",
                &self.tree,
                |m| m,
                |item| label(item.scope.child("name"), "objet", &EditorPalette::DARK),
            );
            if self.names {
                view = view.accessible_names(|key| format!("Objet {key}"));
            }
            view.element()
        }
        fn update(&mut self, message: TreeMsg) -> bool {
            self.tree.update(message, &Two).rebuild
        }
    }
    let mut tree = TreeState::new(&Two);
    tree.set_expanded(1, true, &Two);
    let app = HeadlessApp::new(Named { tree, names: true }).unwrap();
    let row = |key: u64| app.accessible(ElementId::from("tree").item(key)).unwrap();
    let first = row(1);
    assert_eq!(first.name, "Objet 1");
    assert_eq!(first.value.as_deref(), Some("niveau 1, ligne 1 sur 4"));
    assert_eq!(first.expanded, Some(true));
    let child = row(3);
    assert_eq!(child.value.as_deref(), Some("niveau 2, ligne 3 sur 4"));
    assert_eq!(
        child.expanded, None,
        "a leaf is neither expanded nor collapsed"
    );
    // Témoin : sans noms donnés, la ligne n'a pas de nom.
    let mut tree = TreeState::new(&Two);
    tree.set_expanded(1, true, &Two);
    let bare = HeadlessApp::new(Named { tree, names: false }).unwrap();
    let node = bare.accessible(ElementId::from("tree").item(1)).unwrap();
    assert_eq!(node.name, "");
}
