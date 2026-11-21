# Entrées brutes : les trois boutons de la souris et toutes les touches

Les contrôles ordinaires (boutons, champs, glisser) suffisent à une interface. Un outil
de dessin, une vue 3D ou un jeu ont besoin de plus : le bouton droit ou celui du milieu
tenus pendant qu'on glisse, une lettre par outil, Ctrl+Z. Deux portes les donnent, en
plus de ce qui existe et sans rien en retirer :

- **`Element::on_pointer`** : les trois boutons de la souris sur un élément ;
- **`Application::key`** : toutes les touches du clavier.

Code : `src/ui/runtime/raw_input/` (types et acheminement, indépendants du système),
`src/platform/windows/events/raw_input.rs` (Windows). Tests :
`src/ui/tests/runtime/raw_input.rs`.

## `on_pointer`

```rust
use karbon_ui::ui::runtime::{PointerButton, PointerEvent, PointerPhase};

element.on_pointer(|event: PointerEvent| match (event.phase, event.button) {
    (PointerPhase::Down, Some(PointerButton::Secondary)) => Some(Msg::Grab(event.x, event.y)),
    (PointerPhase::Move, _) => Some(Msg::Drag(event.x, event.y)),
    (PointerPhase::Up, Some(PointerButton::Secondary)) => Some(Msg::Release),
    _ => None,
})
```

- `PointerEvent` : la phase (`Down`, `Move`, `Up`), le bouton (`Primary`, `Secondary`,
  `Middle` ; aucun pour un mouvement), la position dans la fenêtre et dans l'élément, et
  Maj, Ctrl, Alt.
- Le gestionnaire rend le message à envoyer, ou `None` : l'événement passe alors aux
  ancêtres, comme `onwheel`.
- **Le geste reste à l'élément qui a pris l'appui** : les mouvements et les
  relâchements lui reviennent, même hors de lui et hors de la fenêtre (la fenêtre garde
  la souris), jusqu'au dernier bouton relâché. Sans geste, un mouvement va aux éléments
  sous le pointeur.
- Le bouton gauche garde ses effets ordinaires (clic, glisser, sélection) ; le bouton
  droit relâché amène toujours le menu contextuel.

## `Application::key`

```rust
fn key(&mut self, press: KeyPress) -> bool {
    if press.pressed && press.control && press.key == KeyCode::Z {
        self.undo();
        return true; // reconstruire la vue
    }
    false
}
```

- `KeyCode` nomme une touche par sa place sur un clavier américain : `KeyCode::W` est la
  touche Z d'un clavier français. Lettres, chiffres, F1 à F12, Échap, Entrée, Tab, Espace,
  retour arrière, Suppr, Inser, Début, Fin, pages, flèches, Maj, Ctrl, Alt.
- `KeyPress` : la touche, enfoncée ou relâchée, répétée ou non, et Maj, Ctrl, Alt.
- **Rien n'arrive pendant qu'un champ de saisie a le clavier** : ce qu'on y tape est à
  lui (Ctrl+Z y annule la saisie).
- La touche garde son effet ordinaire : Tab déplace le focus, Entrée active le contrôle
  focalisé, Échap ferme un menu. L'application la voit avant ce traitement.
- Par défaut, `key` ne fait rien : une application qui ne l'écrit pas ne voit aucune
  différence.

## Ce qui n'est pas fait

- **Windows seulement** : sous macOS et Linux, ces entrées n'arrivent pas encore (les
  applications existantes n'y voient aucun changement). Les touches tenues avec Alt
  (`WM_SYSKEYDOWN`) ne sont pas données.
- Ni la molette dans `PointerEvent` (elle a `onwheel`), ni les boutons 4 et 5, ni le
  pavé numérique.

## Contrôles faits

- Les fichiers existants ne reçoivent que des lignes ajoutées (aucune retirée ni
  changée) : `git diff --numstat` le montre.
- Exports de référence identiques octet pour octet (`rustmusic_html` en 1280×900 et
  960×700, `bootstrap_demo` deux fois).
- Tests : un appui pris garde le geste jusqu'au dernier bouton relâché ; un appui non pris
  passe aux ancêtres ; les touches vont à l'application sauf quand un champ de saisie a le
  clavier. Contrôle négatif fait (sans ces règles, trois tests échouent).
- Un test de l'exemple `rustmusic_html` (`title_bar_button_collapses_and_expands_the_sidebar`)
  échoue, **et échouait déjà avant ces ajouts**, de la même façon (vérifié sur une copie
  au dernier commit) : centrage de la barre latérale repliée à 960×700.
