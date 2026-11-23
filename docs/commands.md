# Commandes, raccourcis et palette (`ui::commands`)

KUI-044 (registre et raccourcis configurables) et KUI-042 (palette de commandes) du
cahier de l'éditeur de karbon-engine. Module neuf, rien d'existant ne change.

## Raccourcis : `Shortcut`

- Une touche, par ce qui est écrit dessus (`ShortcutKey::Key(KeyCode)`) ou par sa place
  (`ShortcutKey::Place(PhysicalKey)`), avec Ctrl, Maj, Alt et la touche Windows.
- Lus et écrits en clair, en anglais ou en français, sans tenir compte des capitales :
  « Ctrl+Shift+D », « Ctrl+Maj+D », « Suppr », « 7 du pavé », « Num7 », « Ctrl+[ »,
  « Win+E » ; une place s'écrit entre crochets d'après son nom du web (« [KeyW] »).
- `Shortcut::matches(&KeyPress)` : la touche enfoncée (ou répétée), exactement ces
  modificateurs.

## Registre : `Keymap`

```rust
let mut keymap = Keymap::new();
keymap.add(Command::new("object.duplicate", "Dupliquer").shortcut(Shortcut::key(KeyCode::D).ctrl()));
keymap.add(Command::new("object.delete", "Supprimer").context("view").shortcut(Shortcut::key(KeyCode::Delete)));
keymap.load(&storage, "raccourcis");                     // les changements de l'utilisateur
let command = keymap.resolve(&press, &["view", GLOBAL]); // le contexte le plus étroit d'abord
let menu_text = keymap.shortcut_text("object.duplicate", ShortcutNames::French);
```

- `Command` : identifiant, nom, description, contexte (`GLOBAL` : partout), raccourcis de
  départ.
- `set_shortcuts`, `reset`, `reset_all`, `is_changed` ; `changes_text` et
  `apply_changes` (une ligne par commande, « aucun » ou « none », `#` en commentaire ;
  les lignes illisibles sont rendues) ; `save` et `load` dans un `LocalStorage`.
- `conflicts()` : deux commandes du même contexte sur la même touche ; `users_of` pour
  prévenir avant de valider une touche.
- Ce texte alimente les menus (`MenuEntry::shortcut`), les bulles (`editor::tip`) et la
  barre d'outils (`ToolItem::shortcut`).

## Palette : `CommandPalette`

- `PaletteItem { id, name, detail }`, `PaletteState` (`open`, `close`, `query`,
  `matches`, `update`) ; la vue est un popover posé à `at`, au champ de recherche qui
  prend le clavier (`TextInput::autofocus`, flèches par `TextInput::on_key`, Entrée par
  `on_submit`) ; Échap ou un clic dehors la ferment, un clic choisit.
- `fuzzy_score` et `rank` : lettres dans l'ordre, capitales et accents ignorés, débuts de
  mots et lettres collées comptés plus. 1 000 commandes : moins d'une milliseconde par
  frappe (test, version optimisée).

## L'éditeur de karbon-engine

Il garde son registre (`commands/keymap.rs`, `raccourcis.txt`) et lit par `Shortcut` les
noms de touches qu'il ne connaît pas (F13 à F24, pavé, ponctuation). Sa palette
(Ctrl+Maj+P, F3) est cette `CommandPalette`, remplie de ses commandes.

## Contrôles

`src/ui/commands/tests.rs` : lecture et écriture dans les deux langues (témoins :
touche, modificateur, place inconnus) ; correspondance par nom et par place ;
contextes ; Ctrl+D passé à Ctrl+Maj+D, gardé et relu « au lancement suivant » ;
conflits ; recherche floue et son temps ; palette entièrement au clavier, Échap, clic,
recherche sans résultat.
