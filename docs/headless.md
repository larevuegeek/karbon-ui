# Tester une application sans fenêtre

Le backend headless (`karbon_ui::platform::headless`) est compilé sur toutes
les cibles, en Rust pur et sans dépendance. Il permet de piloter une
`Application` dans un test unitaire, sous Windows comme sous Linux (CI) ou
macOS, sans ouvrir de fenêtre ni dépendre des polices installées.

## Premier test

```rust
use karbon_ui::{platform::headless::HeadlessApp, ui::{prelude::*, runtime::Key}};

#[test]
fn le_compteur_augmente() -> karbon_ui::Result<()> {
    let mut app = HeadlessApp::new(Counter::default())?; // 800 × 600, thème sombre
    app.click("increment")?;               // survol, appui, relâchement au centre
    assert_eq!(app.app().count, 1);        // état de l’application
    assert!(app.contains_text("Count: 1")); // texte peint dans la Scene
    app.press(Key::Tab)?;                  // navigation clavier
    assert_eq!(app.focused(), Some(ElementId::named("increment")));
    Ok(())
}
```

Chaque méthode d’entrée envoie les mêmes `UiEvent` qu’une fenêtre native,
livre les messages à `update`, puis reconstruit, met en page et peint la vue :
l’état se relit immédiatement. Les identités acceptent les noms donnés aux
constructeurs (`"save"`) ou des nombres (`10`).

## Ce que le harnais sait faire

| Besoin | Méthode |
| --- | --- |
| Créer | `HeadlessApp::new(app)`, `with_options(app, HeadlessOptions { width, height, theme, title_bar, transparent, reduced_motion, scale })` |
| Pointeur | `click(id)`, `click_at(point)`, `hover(id)`, `scroll(id, dx, dy)` |
| Clavier | `type_text("abc")` (un caractère à la fois), `press(Key::Enter)`, `press_with_shift(Key::Tab)`, `edit(EditCommand::Copy)` |
| Focus et accessibilité | `focus(id)`, `accessibility_action(action)`, `focused()`, `accessibility()`, `accessible(id)` |
| Messages et état | `send(message)`, `app()`, `update_app(\|app\| …)` |
| Fenêtre | `resize(w, h)`, `set_theme(theme)`, `take_window_commands()` (jamais exécutées) |
| Temps | `advance(Duration)` : horloge virtuelle, animations reproductibles |
| Tâches | `run_tasks()` : exécute les tâches demandées et celles qu’elles enchaînent |
| Relecture | `scene()`, `texts()`, `contains_text(…)`, `find(id)`, `bounds(id)`, `input_value(id)` |
| Presse-papiers | simulé : `clipboard()`, `set_clipboard(…)` ; copier/couper/coller passent par lui |
| Pixels | `render(scale)` : image BGRA prémultipliée du rastériseur logiciel ; `HeadlessOptions::scale` simule l'échelle de l'écran (bordures et boîtes des feuilles de frameworks calées sur ses pixels), à passer aussi à `render` |
| Accès bas niveau | `event(UiEvent)`, `runtime()`, `runtime_mut()` puis `prepare()` |

Les tâches de fond ne tournent que lors d’un appel explicite à `run_tasks()`,
ce qui permet de vérifier l’état « chargement » avant les résultats. Les délais
des tooltips suivent l’horloge système et ne sont pas pilotés par `advance`.

## Métriques déterministes

`HeadlessTextMeasurer` remplace DirectWrite par des métriques fixes :

- chaque caractère (valeur scalaire Unicode, espaces compris) avance de
  `0,6 ×` la taille de police, quelles que soient la famille et la graisse ;
- une ligne mesure `1,25 ×` la taille, ou la hauteur de ligne explicite du style ;
- `\n` ouvre une ligne ; au-delà de la largeur disponible, le texte se coupe
  après le dernier espace qui tient, et un mot trop long est coupé entre deux
  caractères ; l’espace de coupure n’est pas compté ;
- la largeur mesurée est celle de la ligne la plus longue ; un texte vide fait
  zéro de large et une ligne de haut.

`"Hello"` en 20 px mesure donc 60 × 25 ; `"Hello world"` limité à 80 px tient
sur deux lignes, 60 × 50. Les positions des tests restent identiques sur toutes
les plateformes, mais diffèrent de celles du rendu Windows : vérifier des
relations (ordre, largeur relative, débordement) plutôt que des pixels Windows.

## Images et rendu

`Resources::decode_image` ne décode pas : l’en-tête PNG/JPEG est validé, puis
un aplat gris de mêmes dimensions est renvoyé (le décodeur des tâches renvoie
un aplat de la taille de la vignette). Les applications qui chargent des images
dans `load` fonctionnent donc sans décodeur natif.

`render(scale)` rastérise la scène avec un échantillon par pixel, sans
anticrénelage : rectangles arrondis pleins ou tracés, lignes, chemins, images
(plus proche voisin), clips, transformations et opacité de groupe sont exacts.
Les dégradés sont peints avec la moyenne de leurs arrêts, les filtres sont
ignorés, les icônes `Vector` ne sont pas dessinées ; les dessins SVG compilés sont
remplis (non nul ou pair-impair) et tracés (bouts plats, ronds ou carrés, jonctions
arrondies) après aplatissement des courbes ; chaque glyphe est une
boîte aux métriques ci-dessus. Le résultat sert aux vérifications de couleurs ou
de recouvrement, pas à la comparaison visuelle avec Windows.

## Hors Windows

`platform::run` et `run_windows` renvoient toujours `Error::Unsupported` hors
Windows : le backend headless n’a pas de boucle d’événements. En revanche,
`diagnostics::render_preview` et `render_benchmark` y utilisent le rastériseur
headless (compteurs mémoire et CPU à zéro), et `diagnostics::write_png` un
encodeur PNG intégré (blocs non compressés). Sous Windows, ces fonctions gardent
Direct2D et GDI+.

Pour vérifier la compilation Linux depuis Windows (bibliothèque, tests et
exemples) :

```powershell
cargo check --all-targets --target x86_64-unknown-linux-gnu
cargo clippy --all-targets --target x86_64-unknown-linux-gnu -- -D warnings
```

`cargo test --target x86_64-unknown-linux-gnu --no-run` compile aussi, mais
l’édition de liens demande un éditeur de liens et une sysroot Linux (absents
d’un poste Windows standard) : lancer les tests sur une machine ou une CI Linux,
avec `cargo test --workspace --all-targets`.
