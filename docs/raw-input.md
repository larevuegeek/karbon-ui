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

- `KeyCode` nomme une touche par ce qui est marqué dessus, dans la disposition active :
  sous Windows, il vient du code virtuel, qui suit la disposition. Sur un clavier
  français, la touche marquée Z donne `KeyCode::Z`, comme sur un clavier américain :
  Ctrl+Z annule pour tout le monde. Pour la place de la touche, lire `physical` (plus
  bas). Lettres, chiffres, F1 à F12, Échap, Entrée, Tab, Espace,
  retour arrière, Suppr, Inser, Début, Fin, pages, flèches, Maj, Ctrl, Alt.
- `KeyPress` : la touche, enfoncée ou relâchée, répétée ou non, et Maj, Ctrl, Alt.
- `KeyPress::physical` : la place de la touche sur le clavier, quelle que soit la
  disposition (`PhysicalKey`, nommée comme les codes physiques du web :
  `PhysicalKey::KeyW` est la touche marquée Z sur un clavier français, W sur un
  américain). Une caméra en ZQSD ou WASD lie `KeyW`, `KeyA`, `KeyS`, `KeyD` et marche
  sur les deux. `KeyPress::scancode` : le code brut du système (sous Windows, le code
  de balayage de lparam, avec `0xE000` pour les touches étendues).
- F10 et les touches tenues avec Alt (`WM_SYSKEYDOWN`, `WM_SYSKEYUP`) arrivent aussi,
  depuis 0.3.22 ; Windows garde ensuite leur traitement (Alt+F4 ferme la fenêtre, Alt
  seul et F10 ouvrent le menu de la fenêtre).
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

## Ajouts de 0.3.22 : place physique et touches système

Code : `src/ui/runtime/raw_input/physical.rs` (`PhysicalKey`),
`src/platform/windows/events/scancode.rs` (table des codes de balayage de Windows),
`WM_SYSKEYDOWN` et `WM_SYSKEYUP` dans `events/dispatch.rs`.

- La phrase « Les touches tenues avec Alt (`WM_SYSKEYDOWN`) ne sont pas données », plus
  haut, valait pour 0.3.21 : elles le sont depuis 0.3.22, avec F10. Une application qui
  écrit `Application::key` reçoit donc de nouvelles touches : Alt seul, Alt+lettre, F10.
- `physical` n'est donné que pour les 66 touches de `KeyCode` : une touche sans
  `KeyCode` (ponctuation, pavé numérique avec Verr. Num.) n'arrive toujours pas.
  La table connaît pourtant les 120 places de `PhysicalKey`, prête pour la suite.
- Le relâchement (`WM_KEYUP`) passe par `keyboard::key_up`, qui ne reçoit pas lparam :
  la table de routage dépose lparam avant de l'appeler (`scancode::remember_release`),
  pour ne pas changer la signature d'une fonction existante.
- Sous Linux et macOS, ces entrées n'arrivent pas encore ; `physical` y sera `None` et
  `scancode` 0 tant qu'elles n'y sont pas branchées.

Contrôles faits :

- tests de la table (codes vers places, bit étendu qui sépare flèches et pavé numérique,
  une seule place par touche) ;
- disposition française chargée par `LoadKeyboardLayoutW` dans un fil d'essai, sans
  l'activer : la touche à la place du W américain y donne `VK_Z`, donc `KeyCode::Z`, et
  sa place reste `PhysicalKey::KeyW` (et la disposition est rendue si l'essai l'a
  ajoutée) ;
- sur une fenêtre cachée : F10, Alt+Q enfoncé et relâché, Alt de droite arrivent à
  `Application::key` avec leur place ; un Ctrl de droite relâché garde sa place.
  Contrôle négatif fait : sans les lignes ajoutées à `dispatch.rs`, les deux tests
  échouent ;
- **non vérifié par un test** : Alt+F4 qui ferme la fenêtre. Envoyé à une fenêtre
  cachée, Windows ne poste pas la fermeture (il ne le fait que pour une vraie frappe) ;
  le code rend à `DefWindowProcW` le message comme avant. À essayer à la main.

## Ajouts de 0.3.23 : rang du clic, molette avec modificateurs

- `PointerEvent::clicks` : rang de l'appui dans une série d'appuis rapprochés du même
  bouton (1, 2 pour un double-clic, 3…), compté par le runtime à 500 ms et 4 pixels
  logiques (les valeurs par défaut de Windows) ; le relâchement porte le rang de son
  appui, un mouvement 0. Test : `src/ui/tests/runtime/raw_input_additions.rs`.
- `Element::on_pointer_wheel` : la molette avec Maj, Ctrl et Alt, phase
  `PointerPhase::Wheel`, tour en crans dans `PointerEvent::wheel`. Gestionnaire à part :
  les `on_pointer` existants ne la voient jamais. Le premier qui rend un message la prend
  (ni `on_wheel` ni défilement) ; sinon tout se passe comme avant. Sous Windows, ligne
  ajoutée à `events/pointer.rs` (`wheel`) avant le traitement existant.
- Voir aussi [les composants d'éditeur](editor.md) : fermeture annulable
  (`Application::close_requested`), `TextInput::autofocus`, `LiveImage`.

## Ajouts pour l'éditeur de karbon-engine (KUI-012, KUI-013, KUI-014, KUI-046)

Tout est fait par ajout ; une application qui ne se sert pas de ces gestionnaires ne voit
rien changer (tests de `raw_input` et exports de référence inchangés).

### Les touches manquantes (KUI-012)

- `KeyCode` gagne la ponctuation (`Minus`, `Equal`, `BracketLeft`, `BracketRight`,
  `Backslash`, `Semicolon`, `Quote`, `Backquote`, `Comma`, `Period`, `Slash`), le pavé
  numérique (`Numpad0` à `Numpad9`, `NumpadAdd`, `NumpadSubtract`, `NumpadMultiply`,
  `NumpadDivide`, `NumpadDecimal`), F13 à F24, `Meta` (touche Windows), `CapsLock`,
  `NumLock`, `ScrollLock`, `PrintScreen`, `Pause`, `ContextMenu`, et `Unidentified`.
- Une touche de ponctuation est nommée **d'après son caractère dans la disposition
  active** (`MapVirtualKeyW`) : sur un clavier français, la touche à la place du « ; »
  américain donne « , », donc `Comma`. Une touche dont le caractère n'a pas de nom (« ^ »,
  « $ », « ù », « < » d'un AZERTY) arrive comme `Unidentified`, avec sa place dans
  `physical` (`BracketLeft`, `BracketRight`…) : c'est par elle qu'on la lie.
- Les touches nommées avant gardent leur nom : la table ajoutée
  (`platform/windows/events/more_keys.rs`) n'est consultée que si l'ancienne ne connaît
  pas le code. **Ces touches arrivent maintenant à `Application::key`**, comme Alt+lettre
  depuis 0.3.22 : une application qui écrit `key` les reçoit en plus.
- Gauche et droite (Maj, Ctrl, Alt) : `physical` les distingue (`ShiftLeft`,
  `ShiftRight`…).
- `KeyPress::meta` et `PointerEvent::meta` : la touche Windows tenue.

### Le clavier du panneau qui a le focus (KUI-013) : `Element::on_key`

```rust
let view = column("view", children).on_key(|press: KeyPress| {
    (press.pressed && press.key == KeyCode::Delete).then_some(Msg::RemoveObject)
});
let tree = column("tree", rows).on_key(|press: KeyPress| {
    (press.pressed && press.key == KeyCode::Delete).then_some(Msg::RemoveLine)
});
```

- Une touche part de l'élément qui a le focus clavier s'il a changé depuis le dernier
  appui (Tab), sinon de l'élément où l'on a appuyé en dernier (n'importe quel bouton de
  la souris), et remonte ses ancêtres jusqu'au premier `on_key` qui rend un message ;
  sinon elle va à `Application::key`, comme avant.
- Rien n'y arrive pendant qu'un champ de saisie a le clavier (comme pour `key`).
- Code : `src/ui/runtime/raw_input/panel_keys.rs`.

### Pointeur verrouillé et mouvement relatif (KUI-014)

```rust
column("view", [])
    .on_pointer(|e: PointerEvent| match e.phase {
        PointerPhase::Move => Some(Msg::Turn(e.delta.0, e.delta.1)),
        _ => e.button.map(|_| Msg::Grab),
    })
    .pointer_lock(self.flying)
```

- `PointerEvent::delta` : l'écart depuis l'événement précédent (pour tout mouvement).
- `Element::pointer_lock(true)` : pendant un geste pris par cet élément, Windows cache le
  pointeur (`ShowCursor`), le tient dans la fenêtre (`ClipCursor`) et le ramène à la
  place de l'appui après chaque mouvement (`SetCursorPos`) ; `delta` est alors le
  mouvement propre de la souris, qui ne bute jamais au bord de l'écran. À la fin du
  geste, le pointeur est rendu, visible, à sa place.
- La fenêtre perd le clavier (Alt+Tab, `WM_KILLFOCUS`) ou la souris
  (`WM_CAPTURECHANGED`) : l'élément reçoit le relâchement des boutons tenus et le geste
  finit (`UiRuntime::pointer_lock_lost`). Sans verrou, rien ne change.
- Le drapeau est lu sur l'arbre en cours : une application le pose d'après son état
  (l'éditeur : `pointer_lock(self.camera.is_some())`), la vue étant refaite après l'appui.
- Code : `src/ui/runtime/raw_input/lock.rs`, `src/platform/windows/events/pointer_lock.rs`.

### Fichiers déposés depuis l'Explorateur (KUI-046) : `Element::on_file_drop`

- `FileDropEvent` : `phase` (`Over` répété, `Leave`, `Drop`), `paths` (chemins complets),
  position dans la fenêtre et dans l'élément. Le gestionnaire rend un message pour
  accepter (curseur « copier »), `None` pour refuser (les ancêtres, sinon le curseur
  interdit) ; l'élément quitté reçoit `Leave`.
- Sous Windows, la fenêtre s'inscrit comme cible OLE (`RegisterDragDrop`, `IDropTarget`
  en `#[implement]`, chemins lus de `CF_HDROP`) **seulement** quand un tel gestionnaire a
  été posé sur son fil : une application qui n'en pose pas ne s'inscrit jamais. Code :
  `src/ui/runtime/raw_input/file_drop.rs`, `src/platform/windows/events/file_drop.rs`.
  Le crate `windows` (déjà présent) gagne les fonctions `Win32_Graphics_Gdi`,
  `Win32_System_Com_StructuredStorage` et `Win32_System_SystemServices`.

### Zoom de l'interface choisi par l'application

`Application::interface_zoom(&self) -> f32` (1 par défaut) : multiplié à l'échelle de
l'écran sous Windows (`ffi::scale`, par fenêtre) et par la fenêtre simulée ; la vue reçoit
moins d'unités, plus grandes (`ViewContext::width` divisé, `scale` multiplié). Tenu entre
0,5 et 3 (`UiRuntime::interface_zoom`). Un changement refait toute l'image suivante.

### Pour les tests : la fenêtre simulée

`HeadlessApp::raw_pointer(phase, bouton, place, RawModifiers)`, `raw_locked_move(place,
mouvement)`, `pointer_locked()`, `raw_key(KeyPress)`, `drag_files(phase, chemins, place)`
donnent ces entrées comme une fenêtre (`src/platform/headless/app/raw.rs`).

### Contrôles

Tests sans fenêtre (`src/ui/tests/runtime/raw_input_panels.rs`, `headless/app/raw.rs`,
`more_keys.rs`, `pointer_lock.rs`, `ffi.rs`) : touche au panneau appuyé puis remontée,
Tab, champ de saisie, sans `on_key` rien ne change ; 50 mouvements verrouillés de 40
pixels (2 000 pixels) sans butée ; perte du verrou qui relâche les boutons (témoin : sans
verrou, le geste reste) ; table des touches ajoutées (pavé distinct des chiffres,
ponctuation d'un clavier américain et d'un clavier français) ; fichiers acceptés,
refusés, quittés ; zoom de 1,25 sur un écran à 150 % (largeur 640 au lieu de 800, échelle
1,875 ; bornes). Contrôles négatifs faits (sans `panel_key`, sans le mouvement de la
souris : les essais échouent). **Non vérifié à la main** : le verrou dans une vraie
fenêtre (les essais ne bougent pas le vrai pointeur), un vrai glisser depuis
l'Explorateur, le zoom dans une vraie fenêtre.
