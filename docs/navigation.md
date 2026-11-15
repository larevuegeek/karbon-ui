# Historique de navigation (`karbon_ui::navigation`)

`History<R>` tient les piles Retour et Suivant d'un onglet de navigateur, pour n'importe
quel type de route (en général l'énumération des pages de l'application).

```rust
use karbon_ui::navigation::History;

#[derive(Clone, PartialEq)]
enum Route { Home, Albums, Album(String), Settings(u8) }

let mut history = History::new(Route::Home);
history.push(Route::Albums);           // la page courante passe derrière
history.push(Route::Album("42".into()));
history.back();                        // Albums ; l'album est devant
history.forward();                     // l'album de nouveau
history.push(Route::Settings(0));      // ouvrir une page efface les suivantes
history.back_past(|r| matches!(r, Route::Settings(_)), Route::Home); // sortir des réglages

history.current();                     // &Route
history.can_go_back();                 // pour griser les boutons de la barre de titre
history.can_go_forward();
```

- `push` rend `false` (et ne change rien) quand la route est déjà la page courante.
- `replace` change la page courante sans ajouter d'entrée (redirection, paramètres
  modifiés).
- `back_past(skip, repli)` revient à la dernière page que `skip` ne retient pas, en
  écartant celles d'entre-deux (quitter d'un coup un groupe de pages de réglages) ; sans
  elle, ouvre `repli`. La page quittée est alors la seule devant.
- `behind()` et `ahead()` exposent les piles (la plus récente en dernier), par exemple
  pour un menu d'historique.
