//! Raccourcis lus et écrits, registre (contextes, conflits, changements gardés), recherche
//! floue et palette de commandes dans une fenêtre simulée.
use super::*;
use crate::{
    application::Application,
    platform::headless::{HeadlessApp, HeadlessOptions},
    storage::LocalStorage,
    ui::{
        element::{Element, ElementId},
        prelude::{ViewContext, column, layers},
        runtime::{Key, KeyCode, KeyPress, PhysicalKey},
        scene::PointF,
    },
};

#[test]
fn shortcuts_are_read_and_written_in_english_and_in_french() {
    let s = Shortcut::parse("Ctrl+Maj+D").unwrap();
    assert_eq!(s, Shortcut::key(KeyCode::D).ctrl().shift());
    assert_eq!(s.text(ShortcutNames::English), "Ctrl+Shift+D");
    assert_eq!(s.text(ShortcutNames::French), "Ctrl+Maj+D");
    for (text, expected) in [
        (
            "ctrl+shift+delete",
            Shortcut::key(KeyCode::Delete).ctrl().shift(),
        ),
        ("Suppr", Shortcut::key(KeyCode::Delete)),
        ("Alt+F4", Shortcut::key(KeyCode::F4).alt()),
        ("F24", Shortcut::key(KeyCode::F24)),
        ("7 du pavé", Shortcut::key(KeyCode::Numpad7)),
        ("Num7", Shortcut::key(KeyCode::Numpad7)),
        ("Ctrl+[", Shortcut::key(KeyCode::BracketLeft).ctrl()),
        ("[KeyW]", Shortcut::place(PhysicalKey::KeyW)),
        ("Pg préc", Shortcut::key(KeyCode::PageUp)),
        ("Win+E", Shortcut::key(KeyCode::E).meta()),
    ] {
        let read = Shortcut::parse(text).unwrap_or_else(|e| panic!("{text}: {e}"));
        assert_eq!(read, expected, "{text}");
        // Écrit puis relu : le même raccourci, dans les deux langues.
        for names in [ShortcutNames::English, ShortcutNames::French] {
            assert_eq!(Shortcut::parse(&read.text(names)).unwrap(), read, "{text}");
        }
    }
    // Témoins : une touche ou un modificateur inconnus, un texte vide.
    assert!(Shortcut::parse("Ctrl+Truc").is_err());
    assert!(Shortcut::parse("Hyper+A").is_err());
    assert!(Shortcut::parse("").is_err());
    assert!(Shortcut::parse("[Nulle]").is_err());
}

#[test]
fn a_shortcut_matches_its_key_by_name_or_by_place_with_exactly_its_modifiers() {
    let ctrl_z = Shortcut::key(KeyCode::Z).ctrl();
    let press = KeyPress {
        control: true,
        ..KeyPress::new(KeyCode::Z, true)
    };
    assert!(ctrl_z.matches(&press));
    // Ni relâchée, ni avec Maj en plus.
    assert!(!ctrl_z.matches(&KeyPress {
        pressed: false,
        ..press
    }));
    assert!(!ctrl_z.matches(&KeyPress {
        shift: true,
        ..press
    }));
    // Par sa place : la touche marquée Z d'un AZERTY est à la place de W.
    let forward = Shortcut::place(PhysicalKey::KeyW);
    let azerty_z = KeyPress {
        physical: Some(PhysicalKey::KeyW),
        ..KeyPress::new(KeyCode::Z, true)
    };
    assert!(forward.matches(&azerty_z));
    assert!(!Shortcut::key(KeyCode::W).matches(&azerty_z));
}

fn editor_keymap() -> Keymap {
    let mut keymap = Keymap::new();
    keymap.add(
        Command::new("object.duplicate", "Dupliquer")
            .description("Copie l'objet à côté de lui.")
            .shortcut(Shortcut::key(KeyCode::D).ctrl()),
    );
    keymap.add(
        Command::new("object.delete", "Supprimer l'objet")
            .context("view")
            .shortcut(Shortcut::key(KeyCode::Delete)),
    );
    keymap.add(
        Command::new("tree.rename", "Renommer la ligne")
            .context("tree")
            .shortcut(Shortcut::key(KeyCode::F2))
            .shortcut(Shortcut::key(KeyCode::Delete).shift()),
    );
    keymap.add(Command::new("file.save", "Enregistrer").shortcut(Shortcut::key(KeyCode::S).ctrl()));
    keymap
}

#[test]
fn a_key_leads_to_the_command_of_the_first_active_context_that_has_one() {
    let keymap = editor_keymap();
    let delete = KeyPress::new(KeyCode::Delete, true);
    assert_eq!(
        keymap.resolve(&delete, &["view", GLOBAL]),
        Some("object.delete")
    );
    // Dans l'arbre, Suppr seule n'est à personne.
    assert_eq!(keymap.resolve(&delete, &["tree", GLOBAL]), None);
    let save = KeyPress {
        control: true,
        ..KeyPress::new(KeyCode::S, true)
    };
    assert_eq!(keymap.resolve(&save, &["tree", GLOBAL]), Some("file.save"));
    assert_eq!(
        keymap.shortcut_text("tree.rename", ShortcutNames::French),
        "F2"
    );
}

#[test]
fn ctrl_d_moved_to_ctrl_shift_d_is_kept_and_read_again_at_the_next_launch() {
    let storage = LocalStorage::in_memory();
    let mut keymap = editor_keymap();
    let moved = vec![Shortcut::parse("Ctrl+Maj+D").unwrap()];
    keymap.set_shortcuts("object.duplicate", moved);
    keymap.set_shortcuts("file.save", Vec::new());
    assert!(keymap.is_changed("object.duplicate"));
    keymap.save(&storage, "keys").unwrap();
    // « Relancer » : un registre neuf relit les changements.
    let mut again = editor_keymap();
    assert!(again.load(&storage, "keys").is_empty());
    let new_key = KeyPress {
        control: true,
        shift: true,
        ..KeyPress::new(KeyCode::D, true)
    };
    let old_key = KeyPress {
        control: true,
        ..KeyPress::new(KeyCode::D, true)
    };
    assert_eq!(again.resolve(&new_key, &[GLOBAL]), Some("object.duplicate"));
    assert_eq!(again.resolve(&old_key, &[GLOBAL]), None);
    let shown = again.shortcut_text("object.duplicate", ShortcutNames::French);
    assert_eq!(shown, "Ctrl+Maj+D");
    assert_eq!(again.shortcut_text("file.save", ShortcutNames::French), "");
    // Remis par défaut : plus aucun changement à garder.
    again.reset_all();
    assert_eq!(again.changes_text(ShortcutNames::English), "");
    // Des lignes illisibles sont dites, les autres appliquées.
    let text = "# note\nnope = F2\nobject.duplicate = Ctrl+Truc\nfile.save = aucun\n";
    let errors = again.apply_changes(text);
    assert_eq!(errors.len(), 2, "{errors:?}");
    assert!(again.shortcuts("file.save").is_empty());
}

#[test]
fn two_commands_on_the_same_key_in_the_same_context_are_reported() {
    let mut keymap = editor_keymap();
    assert!(keymap.conflicts().is_empty());
    // Même touche dans deux contextes différents : pas un conflit.
    keymap.set_shortcuts("tree.rename", vec![Shortcut::key(KeyCode::Delete)]);
    assert!(keymap.conflicts().is_empty());
    // Même touche dans le même contexte : un conflit.
    keymap.set_shortcuts("file.save", vec![Shortcut::key(KeyCode::D).ctrl()]);
    let conflicts = keymap.conflicts();
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].commands, ["object.duplicate", "file.save"]);
    let users = keymap.users_of(&Shortcut::key(KeyCode::D).ctrl(), GLOBAL, "file.save");
    assert_eq!(users, ["object.duplicate"]);
}

#[test]
fn the_fuzzy_search_ranks_starts_of_words_first_and_stays_fast() {
    let names = ["Robe", "Objet nouveau", "Ouvrir le bâtiment"];
    // Deux débuts de mots (« Ouvrir le bâtiment ») passent devant des lettres collées au
    // milieu d'un mot (« Robe »).
    assert_eq!(rank("ob", names, 10), [1, 2, 0]);
    assert_eq!(rank("bat", names, 10), [2]);
    assert_eq!(rank("ÉLÉ", ["Supprimer l'élément"], 10), [0]);
    // 1 000 commandes : une frappe en moins d'une milliseconde (en version optimisée ;
    // la version de mise au point a plus de marge).
    let many: Vec<String> = (0..1000)
        .map(|i| format!("Commande numéro {i} du menu Affichage"))
        .collect();
    let start = std::time::Instant::now();
    let found = rank("99 aff", many.iter().map(String::as_str), 12);
    let spent = start.elapsed();
    assert!(!found.is_empty());
    let budget = if cfg!(debug_assertions) { 20 } else { 1 };
    assert!(spent.as_millis() < budget, "{spent:?}");
}

/// Une application qui ouvre la palette et note la commande choisie.
#[derive(Default)]
struct Palette {
    state: PaletteState,
    items: Vec<PaletteItem>,
    chosen: Vec<String>,
}

impl Application for Palette {
    type Message = PaletteMsg;
    fn view(&self, _: &ViewContext) -> Element {
        let mut overlays = Vec::new();
        if self.state.is_open() {
            overlays.push(
                CommandPalette::new("palette", &self.state, &self.items, |m| m)
                    .at(PointF { x: 20., y: 20. })
                    .build(),
            );
        }
        layers("root", column("page", []), overlays)
    }
    fn update(&mut self, message: PaletteMsg) -> bool {
        let reply = self.state.update(message, &self.items);
        self.chosen.extend(reply.chosen);
        reply.rebuild
    }
}

fn palette_window() -> HeadlessApp<Palette> {
    let keymap = editor_keymap();
    let items = keymap
        .commands()
        .iter()
        .map(|c| PaletteItem {
            id: c.id.clone(),
            name: c.name.clone(),
            detail: keymap.shortcut_text(&c.id, ShortcutNames::French),
        })
        .collect();
    let mut state = PaletteState::default();
    state.open();
    let app = Palette {
        state,
        items,
        chosen: Vec::new(),
    };
    let options = HeadlessOptions {
        width: 800.,
        height: 600.,
        ..Default::default()
    };
    HeadlessApp::with_options(app, options).unwrap()
}

fn line(index: u64) -> ElementId {
    ElementId::named("palette").child("line").item(index)
}

#[test]
fn the_palette_is_used_entirely_from_the_keyboard() {
    let mut w = palette_window();
    // Le champ a le clavier dès l'ouverture.
    let search = ElementId::named("palette").child("search");
    assert_eq!(w.focused(), Some(search));
    w.type_text("sup").unwrap();
    assert_eq!(w.app().state.query(), "sup");
    assert!(w.find(line(0)).is_some() && w.find(line(1)).is_none());
    // Une recherche plus large, la flèche du bas, puis Entrée.
    for _ in 0..3 {
        w.press(Key::Backspace).unwrap();
    }
    w.type_text("e").unwrap();
    w.press(Key::Down).unwrap();
    assert_eq!(w.app().state.selected(), 1);
    let expected = {
        let shown = w.app().state.matches(&w.app().items);
        w.app().items[shown[1]].id.clone()
    };
    w.press(Key::Enter).unwrap();
    assert_eq!(w.app().chosen, [expected]);
    assert!(!w.app().state.is_open());
}

#[test]
fn the_palette_closes_on_escape_and_a_click_chooses_a_line() {
    let mut w = palette_window();
    w.press(Key::Escape).unwrap();
    assert!(!w.app().state.is_open() && w.app().chosen.is_empty());
    let mut w = palette_window();
    w.type_text("enreg").unwrap();
    assert!(w.find(line(0)).is_some());
    w.click(line(0)).unwrap();
    assert_eq!(w.app().chosen, ["file.save"]);
    // Témoin : rien ne correspond, la ligne le dit et Entrée ne choisit rien.
    let mut w = palette_window();
    w.type_text("zzz").unwrap();
    assert!(w.contains_text("Aucune commande ne correspond."));
    w.press(Key::Enter).unwrap();
    assert!(w.app().chosen.is_empty());
}
