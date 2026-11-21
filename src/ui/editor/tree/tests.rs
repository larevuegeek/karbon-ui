//! Arbre virtualisé : lignes visibles, pli, sélection par clé (Ctrl, Maj), clavier,
//! double-clic, glisser-déposer, et le nombre de lignes construites sur 100 000 nœuds.
use super::*;
use crate::ui::{
    editor::{
        EditorPalette, label,
        testing::{CONTROL, NONE, SHIFT, at, click, key, pointer, window},
    },
    element::{Element, ElementId},
    prelude::{Application, ViewContext},
    runtime::{KeyCode, KeyPress, PointerButton, PointerPhase},
};
use std::collections::HashMap;

/// Un arbre de l'application : racines et enfants par clé.
#[derive(Default)]
struct Forest {
    roots: Vec<u64>,
    children: HashMap<u64, Vec<u64>>,
}

impl TreeSource for Forest {
    fn children(&self, parent: Option<u64>) -> &[u64] {
        match parent {
            None => &self.roots,
            Some(key) => self.children.get(&key).map_or(&[], Vec::as_slice),
        }
    }
}

/// 1 ─ 2, 3 ─ 4 ; 5 ; 6 : trois racines, 1 a deux enfants, 3 a un enfant.
fn small() -> Forest {
    Forest {
        roots: vec![1, 5, 6],
        children: HashMap::from([(1, vec![2, 3]), (3, vec![4])]),
    }
}

/// `levels` niveaux : `branching` racines, et `branching` enfants par nœud.
fn large(levels: u32, branching: u64) -> Forest {
    let mut forest = Forest::default();
    let mut next = 1;
    let mut fresh = |count: u64| -> Vec<u64> {
        (0..count)
            .map(|_| {
                next += 1;
                next
            })
            .collect()
    };
    let mut level = fresh(branching);
    forest.roots = level.clone();
    for _ in 1..levels {
        let mut below = Vec::new();
        for &parent in &level {
            let kids = fresh(branching);
            below.extend(&kids);
            forest.children.insert(parent, kids);
        }
        level = below;
    }
    forest
}

fn keys(state: &TreeState) -> Vec<u64> {
    state.rows().iter().map(|row| row.key).collect()
}

fn sorted_selection(state: &TreeState) -> Vec<u64> {
    let mut selected: Vec<u64> = state.selection().iter().copied().collect();
    selected.sort_unstable();
    selected
}

#[test]
fn only_the_expanded_branches_are_rows() {
    let forest = small();
    let mut tree = TreeState::new(&forest);
    assert_eq!(keys(&tree), [1, 5, 6]);
    tree.set_expanded(1, true, &forest);
    assert_eq!(keys(&tree), [1, 2, 3, 5, 6]);
    tree.set_expanded(3, true, &forest);
    assert_eq!(keys(&tree), [1, 2, 3, 4, 5, 6]);
    assert_eq!(tree.rows()[3].depth, 2);
    tree.set_expanded(1, false, &forest);
    assert_eq!(keys(&tree), [1, 5, 6]);
    // Déplié de nouveau, 1 retrouve 3 déplié.
    tree.set_expanded(1, true, &forest);
    assert_eq!(keys(&tree), [1, 2, 3, 4, 5, 6]);
}

#[test]
fn the_keyboard_moves_folds_and_extends_the_selection() {
    let forest = small();
    let mut tree = TreeState::new(&forest);
    let press = |k| KeyPress::new(k, true);
    tree.select(1);
    // Droite déplie, puis descend sur le premier enfant.
    assert_eq!(
        tree.key(press(KeyCode::Right), &forest).event,
        Some(TreeEvent::Expanded(1, true))
    );
    tree.key(press(KeyCode::Right), &forest);
    assert_eq!(tree.cursor(), Some(2));
    // Maj+Bas étend la sélection depuis l'ancre (2).
    let shift_down = KeyPress {
        shift: true,
        ..press(KeyCode::Down)
    };
    tree.key(shift_down, &forest);
    tree.key(shift_down, &forest);
    assert_eq!(tree.cursor(), Some(5));
    assert_eq!(sorted_selection(&tree), [2, 3, 5]);
    // Ctrl+Haut déplace le curseur seul.
    let control = |k| KeyPress {
        control: true,
        ..press(k)
    };
    tree.key(control(KeyCode::Up), &forest);
    tree.key(control(KeyCode::Up), &forest);
    assert_eq!(tree.cursor(), Some(2));
    assert_eq!(tree.selection().len(), 3);
    // Ctrl+Espace retire la ligne du curseur, puis la remet.
    tree.key(control(KeyCode::Space), &forest);
    assert_eq!(sorted_selection(&tree), [3, 5]);
    tree.key(control(KeyCode::Space), &forest);
    assert_eq!(sorted_selection(&tree), [2, 3, 5]);
    // Gauche sur un nœud sans enfant remonte au parent et le choisit seul.
    tree.key(press(KeyCode::Left), &forest);
    assert_eq!(tree.cursor(), Some(1));
    assert_eq!(sorted_selection(&tree), [1]);
    // Gauche sur un nœud déplié le plie.
    tree.key(press(KeyCode::Left), &forest);
    assert_eq!(keys(&tree), [1, 5, 6]);
    // Fin, Début, Entrée, Ctrl+A.
    tree.key(press(KeyCode::End), &forest);
    assert_eq!(tree.cursor(), Some(6));
    tree.key(press(KeyCode::Home), &forest);
    assert_eq!(
        tree.key(press(KeyCode::Enter), &forest).event,
        Some(TreeEvent::Activated(1))
    );
    tree.key(control(KeyCode::A), &forest);
    assert_eq!(sorted_selection(&tree), [1, 5, 6]);
    // Un relâchement ne change rien.
    assert!(
        !tree
            .key(KeyPress::new(KeyCode::Down, false), &forest)
            .rebuild
    );
}

#[test]
fn folding_a_branch_brings_the_cursor_back_to_it() {
    let forest = small();
    let mut tree = TreeState::new(&forest);
    tree.set_expanded(1, true, &forest);
    tree.set_expanded(3, true, &forest);
    tree.select(4);
    let index = tree.index_of(1).unwrap();
    tree.collapse_at(index, &forest);
    assert_eq!(keys(&tree), [1, 5, 6]);
    assert_eq!(tree.cursor(), Some(1));
}

#[test]
fn the_selection_is_kept_by_key_when_the_nodes_are_sorted() {
    let mut forest = small();
    let mut tree = TreeState::new(&forest);
    tree.select(5);
    tree.key(
        KeyPress {
            shift: true,
            ..KeyPress::new(KeyCode::Down, true)
        },
        &forest,
    );
    forest.roots = vec![6, 5, 1];
    tree.refresh(&forest);
    assert_eq!(keys(&tree), [6, 5, 1]);
    assert_eq!(sorted_selection(&tree), [5, 6]);
    assert_eq!(tree.cursor(), Some(6));
}

#[test]
fn a_row_cannot_be_dropped_into_itself_or_its_descendants() {
    let forest = small();
    let mut tree = TreeState::new(&forest);
    tree.set_expanded(1, true, &forest);
    tree.set_expanded(3, true, &forest);
    tree.select(1);
    // Depuis la ligne 1 (indice 0) : 1, 2, 3, 4 sont à elle ; 5 (indice 4) est libre.
    for rows_down in 0..4 {
        let y = rows_down as f32 * 22. + 11.;
        assert_eq!(tree.drop_target(1, y), None, "row {rows_down}");
    }
    assert_eq!(
        tree.drop_target(1, 4. * 22. + 11.),
        Some((4, DropPlace::Into))
    );
    // Hors des lignes.
    assert_eq!(tree.drop_target(1, -30.), None);
    assert_eq!(tree.drop_target(1, 40. * 22.), None);
}

/// Une fenêtre avec l'arbre de `forest`, qui note les événements de l'arbre.
struct Outline {
    forest: Forest,
    tree: TreeState,
    events: Vec<TreeEvent>,
}

impl Outline {
    fn new(forest: Forest) -> Self {
        let tree = TreeState::new(&forest);
        Self {
            forest,
            tree,
            events: Vec::new(),
        }
    }
}

const TREE: &str = "outline";

fn row(key: u64) -> ElementId {
    ElementId::named(TREE).item(key)
}

impl Application for Outline {
    type Message = TreeMsg;
    fn view(&self, _: &ViewContext) -> Element {
        TreeView::new(
            TREE,
            &self.tree,
            |m| m,
            |item| {
                let name = format!("Objet {}", item.key);
                label(item.scope.child("name"), name, &EditorPalette::DARK)
            },
        )
        .focused(true)
        .element()
    }
    fn update(&mut self, message: TreeMsg) -> bool {
        let reply = self.tree.update(message, &self.forest);
        self.events.extend(reply.event);
        reply.rebuild
    }
    fn key(&mut self, press: KeyPress) -> bool {
        let reply = self.tree.key(press, &self.forest);
        self.events.extend(reply.event);
        reply.rebuild
    }
}

#[test]
fn clicks_select_one_row_ctrl_adds_and_shift_selects_a_range() {
    let mut w = window(Outline::new(small()), 300., 200.);
    // Clic sur la flèche de 1 : déplie sans choisir.
    let p = at(&w, row(1), 7., 11.);
    click(&mut w, p, NONE);
    assert_eq!(keys(&w.app().tree), [1, 2, 3, 5, 6]);
    assert!(w.app().tree.selection().is_empty());
    let p = at(&w, row(2), 60., 11.);
    click(&mut w, p, NONE);
    assert_eq!(sorted_selection(&w.app().tree), [2]);
    let p = at(&w, row(5), 60., 11.);
    click(&mut w, p, CONTROL);
    assert_eq!(sorted_selection(&w.app().tree), [2, 5]);
    // Maj : de l'ancre (5) à 1.
    let p = at(&w, row(1), 60., 11.);
    click(&mut w, p, SHIFT);
    assert_eq!(sorted_selection(&w.app().tree), [1, 2, 3, 5]);
    // Un clic simple sur une ligne choisie parmi d'autres ne garde qu'elle.
    std::thread::sleep(std::time::Duration::from_millis(600));
    let p = at(&w, row(3), 60., 11.);
    click(&mut w, p, NONE);
    assert_eq!(sorted_selection(&w.app().tree), [3]);
    // Ctrl sur une ligne choisie la retire.
    std::thread::sleep(std::time::Duration::from_millis(600));
    let p = at(&w, row(3), 60., 11.);
    click(&mut w, p, CONTROL);
    assert!(w.app().tree.selection().is_empty());
}

#[test]
fn a_double_click_activates_the_row() {
    let mut w = window(Outline::new(small()), 300., 200.);
    let point = at(&w, row(5), 60., 11.);
    click(&mut w, point, NONE);
    click(&mut w, point, NONE);
    assert_eq!(w.app().events.last(), Some(&TreeEvent::Activated(5)));
    // Un troisième appui n'active pas de nouveau.
    click(&mut w, point, NONE);
    assert_eq!(w.app().events.last(), Some(&TreeEvent::Selected));
}

#[test]
fn rows_dragged_onto_another_row_are_dropped_into_it() {
    let mut forest = small();
    forest.roots.push(7);
    let mut w = window(Outline::new(forest), 300., 300.);
    let p = at(&w, row(5), 60., 11.);
    click(&mut w, p, NONE);
    let p = at(&w, row(6), 60., 11.);
    click(&mut w, p, CONTROL);
    // Glisser 5 (choisie avec 6) au milieu de 7 : dépôt dans 7.
    let left = Some(PointerButton::Primary);
    std::thread::sleep(std::time::Duration::from_millis(600));
    let p = at(&w, row(5), 60., 11.);
    pointer(&mut w, PointerPhase::Down, left, p, NONE);
    let target = at(&w, row(7), 60., 11.);
    pointer(&mut w, PointerPhase::Move, None, target, NONE);
    let target_of = |w: &crate::platform::headless::HeadlessApp<Outline>| {
        w.app().tree.drag.and_then(|d| d.target)
    };
    assert_eq!(target_of(&w), Some((3, DropPlace::Into)));
    pointer(&mut w, PointerPhase::Up, left, target, NONE);
    let expected = TreeDrop {
        keys: vec![5, 6],
        target: 7,
        place: DropPlace::Into,
    };
    assert_eq!(w.app().events.last(), Some(&TreeEvent::Dropped(expected)));
    // Le haut d'une ligne : avant elle ; le bas : après.
    std::thread::sleep(std::time::Duration::from_millis(600));
    let p = at(&w, row(7), 60., 11.);
    pointer(&mut w, PointerPhase::Down, left, p, NONE);
    let p = at(&w, row(1), 60., 2.);
    pointer(&mut w, PointerPhase::Move, None, p, NONE);
    assert_eq!(target_of(&w), Some((0, DropPlace::Before)));
    let p = at(&w, row(1), 60., 20.);
    pointer(&mut w, PointerPhase::Move, None, p, NONE);
    assert_eq!(target_of(&w), Some((0, DropPlace::After)));
    // Échap abandonne le glisser : rien n'est déposé.
    key(&mut w, KeyCode::Escape, NONE);
    let p = at(&w, row(1), 60., 20.);
    pointer(&mut w, PointerPhase::Up, left, p, NONE);
    assert!(!matches!(
        w.app().events.last(),
        Some(TreeEvent::Dropped(_))
    ));
}

#[test]
fn the_keys_of_the_application_drive_the_tree_and_scroll_to_the_cursor() {
    let forest = large(1, 300);
    let mut w = window(Outline::new(forest), 300., 220.);
    let first = w.app().forest.roots[0];
    let p = at(&w, row(first), 60., 11.);
    click(&mut w, p, NONE);
    key(&mut w, KeyCode::End, NONE);
    let last = *w.app().forest.roots.last().unwrap();
    assert_eq!(w.app().tree.cursor(), Some(last));
    // La dernière ligne est construite et visible dans la liste.
    let list = w.bounds(TREE).unwrap();
    let shown = w.bounds(row(last)).expect("last row built");
    assert!(shown.bottom <= list.bottom + 0.5 && shown.top >= list.top - 0.5);
    // Un défilement à la molette fait oublier la ligne à montrer, et donne la page.
    w.scroll(TREE, 0., -44.).unwrap();
    assert_eq!(w.app().tree.reveal, None);
    assert_eq!(w.app().tree.page, 10);
    key(&mut w, KeyCode::PageUp, NONE);
    let index = w
        .app()
        .tree
        .index_of(w.app().tree.cursor().unwrap())
        .unwrap();
    assert_eq!(index, 299 - 10);
}

#[test]
fn a_hundred_thousand_nodes_build_only_the_visible_rows() {
    // 8 niveaux de 4 enfants : 87 380 nœuds, tous dépliés.
    let forest = large(8, 4);
    let mut outline = Outline::new(forest);
    let all: Vec<u64> = outline.forest.children.keys().copied().collect();
    outline.tree.expanded.extend(all);
    outline.tree.refresh(&outline.forest);
    let count = outline.tree.rows().len();
    assert!(count > 80_000, "{count}");
    let mut w = window(outline, 400., 1080.);
    let realized = |w: &crate::platform::headless::HeadlessApp<Outline>| {
        let metrics = w.find(TREE).unwrap().virtual_metrics().unwrap();
        (metrics.realized_rows, metrics.realized_range.start)
    };
    assert!(realized(&w).0 <= 60, "{:?}", realized(&w));
    for _ in 0..50 {
        w.scroll(TREE, 0., 2_000.).unwrap();
        assert!(realized(&w).0 <= 60, "{:?}", realized(&w));
    }
    assert!(realized(&w).1 > 4_000, "{:?}", realized(&w));
}

/// Mesure à la demande (`cargo test --release --lib tree_timings -- --ignored
/// --nocapture`) : déplier un nœud de 10 000 enfants dans un arbre de 87 000 lignes.
#[test]
#[ignore]
fn tree_timings() {
    let mut forest = large(8, 4);
    let hub = 1;
    forest.roots.push(hub);
    forest
        .children
        .insert(hub, (10_000_000..10_010_000).collect());
    let mut tree = TreeState::new(&forest);
    let all: Vec<u64> = forest
        .children
        .keys()
        .copied()
        .filter(|&k| k != hub)
        .collect();
    tree.expanded.extend(all);
    tree.refresh(&forest);
    let mut times = Vec::new();
    for _ in 0..20 {
        let start = std::time::Instant::now();
        tree.set_expanded(hub, true, &forest);
        times.push(start.elapsed());
        tree.set_expanded(hub, false, &forest);
    }
    times.sort();
    tree.set_expanded(hub, true, &forest);
    println!(
        "déplier 10 000 enfants ({} lignes visibles après) : médiane {:?}, pire {:?}",
        tree.rows().len(),
        times[times.len() / 2],
        times.last().unwrap()
    );
}
