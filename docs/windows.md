# Plusieurs fenêtres et cycle de vie

Cette étape prépare Karbon UI avec un premier gestionnaire de fenêtres
applicatives. L’API est commune ; seul le backend Windows est implémenté.
Le nom Cargo est `karbon-ui`.

## Exemple minimal

```powershell
cargo run --offline --example multi_window --target-dir target/library-check
```

Cet exemple ouvre deux fenêtres indépendantes avec le même socle public.
L’application choisit ses commandes et sa politique de fermeture.

## Une identité et une application par fenêtre

`WindowId` est l’identité logique choisie par l’application. Elle ne contient
aucun handle Windows. Deux fenêtres vivantes doivent avoir des identifiants
différents ; un identifiant peut être réutilisé après sa fermeture.

`WindowDescriptor` rassemble cet ID, les options et l’application qui possède
le contenu. Voici le principe, avec un type applicatif `MyWindow` :

```rust,ignore
let first = WindowDescriptor::new(
    WindowId(1),
    WindowOptions::DEFAULT,
    MyWindow::default(),
);
let second = WindowDescriptor::new(
    WindowId(2),
    WindowOptions { x: 400, light: true, ..WindowOptions::DEFAULT },
    Inspector::default(),
);
platform::run_windows(vec![first, second], ExitPolicy::LastWindowClosed)?;
```

`WindowDescriptor::new` reçoit l’application par valeur. Chaque application
déclare son propre `type Message` et reçoit les messages de sa vue dans
`update` : ici, `MyWindow` et `Inspector` peuvent utiliser des types de message
différents. Elle construit sa vue avec `view`. Les modèles métier peuvent être partagés explicitement, par
exemple avec `Rc<RefCell<Model>>` sur le thread UI ; le moteur ne les partage
pas automatiquement.

L’entrée `platform::run(options, application)` reste disponible et reçoit elle
aussi l’application par valeur. Elle crée une fenêtre avec `WindowId::MAIN` et
utilise `LastWindowClosed`.

## Ouvrir et fermer depuis un bouton

Une application conserve ses demandes dans `Vec<WindowCommand>`. Dans sa
méthode `update`, elle ajoute une commande :

```rust,ignore
self.commands.push(WindowCommand::Open(WindowDescriptor::new(
    WindowId(2),
    options,
    MyWindow::default(),
)));
```

Elle rend puis vide sa file avec le nouveau callback :

```rust,ignore
fn take_window_commands(&mut self) -> Vec<WindowCommand> {
    std::mem::take(&mut self.commands)
}
```

Le moteur extrait les demandes après les callbacks et les exécute depuis la
boucle principale. Les appels natifs n’ont donc pas lieu pendant que le
callback applicatif manipule son état. Un message de réveil permet aussi de
traiter une demande provenant d’un message Windows envoyé pendant l’attente
de la boucle. `update` peut renvoyer `false` si la vue n’a pas changé : cela
n’empêche pas la commande de fenêtre d’être traitée.

Les commandes disponibles sont :

| Commande | Effet |
| --- | --- |
| `Open(descriptor)` | Créer, initialiser, peindre puis afficher une fenêtre. |
| `Action(id, action)` | Réduire, agrandir/restaurer ou fermer via `WindowAction`, notamment depuis une barre de titre personnalisée. |
| `Change(id, change)` | Changer la taille, la position ou l’état d’une fenêtre ouverte (voir ci-dessous). |
| `Close(id)` | Détruire la fenêtre visée ; ne fait rien si elle est déjà fermée. |
| `Exit` | Fermer toutes les fenêtres et abandonner les demandes suivantes du lot. |

L’ordre des demandes d’une même file est conservé. Il n’y a pas de garantie
d’ordre global entre deux applications de fenêtre. Le callback doit rendre
les demandes une seule fois, et non recréer la même ouverture à chaque appel.

## Changer une fenêtre ouverte et suivre son état

`WindowCommand::Change(id, WindowChange)` agit sur une fenêtre déjà ouverte ; une
fenêtre fermée l’ignore. Les tailles sont en pixels logiques de l’écran de la fenêtre,
comme `WindowOptions::width` ; les positions sont celles de `WindowOptions::x` (pixels
logiques à l’échelle de l’écran principal, y vers le bas).

| `WindowChange` | Effet |
| --- | --- |
| `Size { width, height }` | Taille de la zone client, coin haut-gauche gardé, bornée par les tailles minimale et maximale. |
| `MinSize(Option<(w, h)>)`, `MaxSize(…)` | Bornes du redimensionnement ; `None` les lève. La taille courante y est ramenée. |
| `AspectRatio(Option<(w, h)>)` | Proportions gardées pendant un redimensionnement à la souris (le carré du micro-lecteur). |
| `Resizable(bool)` | Bords à tirer et agrandissement (double clic, bouton) permis ou non. |
| `Topmost(bool)` | Au-dessus des fenêtres ordinaires, comme `WindowOptions::topmost` à l’ouverture. |
| `Position { x, y }` | Coin haut-gauche du cadre. |
| `Center`, `KeepOnScreen` | Centrer dans la zone de travail de l’écran, ou y ramener la fenêtre en la déplaçant le moins possible. |
| `Maximized(bool)`, `Fullscreen(bool)` | Agrandir ou restaurer ; couvrir tout l’écran ou en sortir (le rectangle d’avant est rétabli). |
| `Visible(bool)`, `Focus` | Masquer ou montrer sans fermer ; montrer, restaurer et passer au premier plan. |

Un changement incohérent (taille nulle ou négative, proportion nulle) est une erreur de
programmation : `Error::InvalidWindowOptions`, comme des options invalides à l’ouverture.

`Application::window_changed(WindowState)` reçoit l’état de la fenêtre : une fois à
l’ouverture, puis après chaque changement de position, de taille, d’échelle, d’état
(agrandie, réduite, plein écran, premier plan, redimensionnable) et au début et à la fin
d’un déplacement ou d’un redimensionnement à la souris (`dragging`). Réduite, la fenêtre
garde la position et la taille d’avant. `x`/`y` se rendent tels quels à `Position` ou à
`WindowOptions::x`, `width`/`height` à `Size`.

```rust,ignore
fn window_changed(&mut self, state: WindowState) -> bool {
    // Fin d'un geste sur la poignée : la taille est retenue.
    if self.window.dragging && !state.dragging {
        self.save_size(state.width);
    }
    self.window = state;
    false
}
```

Un élément `data-resize-region="bottom-right"` (`Element::resize_region`) est une
poignée : l’appuyer redimensionne la fenêtre par ce coin, même quand elle n’est pas
redimensionnable (`Resizable(false)` : ses bords restent fixes), sans que l’application
suive le pointeur. Sous Windows et macOS, karbon-ui suit le geste lui-même : la souris
est capturée à l’appui, chaque mouvement donne la taille « taille de départ +
déplacement du pointeur » en coordonnées de l’écran, bornée par `MinSize`/`MaxSize`,
appliquée et peinte aussitôt (une taille par image) ; le bord opposé reste en place.
Avec `AspectRatio`, un coin suit le plus grand des deux déplacements (comptés vers
l’extérieur) : le carré du micro-lecteur grandit de `max(dx, dy)`, comme
`resizeMicroPlayer` de RustMusic. `dragging` vaut `true` pendant le geste ; l’état remis
au relâchement porte la taille finale, que l’application retient (et ramène dans l’écran
par `KeepOnScreen` si besoin). La poignée l’emporte sur une zone de déplacement
(`data-drag-region`) qui la contient ; elle porte son curseur (`cursor: nwse-resize`),
le moteur n’en impose pas. Une fenêtre agrandie ou en plein écran ne se redimensionne pas
par sa poignée.

Ce que chaque plateforme honore (un changement impossible est ignoré) :

| Changement | Windows | macOS | X11 | Wayland |
| --- | --- | --- | --- | --- |
| `Size`, `MinSize`, `MaxSize`, `Resizable`, `Maximized`, `Fullscreen` | oui | oui | oui (gestionnaire) | oui (fenêtre flottante pour `Size`) |
| `AspectRatio` | oui (`WM_SIZING`) | oui | oui (`WM_NORMAL_HINTS`) | non (aucun protocole) |
| `Topmost` | oui | oui (niveau flottant) | oui (`_NET_WM_STATE_ABOVE`) | non |
| `Position`, `Center`, `KeepOnScreen` | oui | oui | oui (`_NET_WORKAREA`) | non : le compositeur place les fenêtres |
| `Visible`, `Focus` | oui | oui | oui (`_NET_ACTIVE_WINDOW`) | non (activation sans jeton refusée) |
| `WindowState::x`, `y` | oui | oui | oui | toujours 0 |
| `WindowState::dragging` | oui (`WM_ENTERSIZEMOVE`, poignée) | oui (poignée, redimensionnement) | non | oui (`resizing`) |
| Poignée `data-resize-region` | oui (suivie à la main) | oui (suivie à la main) | oui (`_NET_WM_MOVERESIZE`, règles du gestionnaire) | oui (`xdg_toplevel.resize`, règles du compositeur) |

Sous Windows, une fenêtre non redimensionnable perd `WS_THICKFRAME` et
`WS_MAXIMIZEBOX` (ni bords, ni Aero Snap, ni agrandissement au double clic) ; le plein
écran retire aussi les coins arrondis et l’ombre. `WM_NCHITTEST` ne rend alors aucun bord :
la fenêtre ne se redimensionne que par ses poignées. Une poignée vaut `HTCLIENT`, avant
les bords et les zones de déplacement : son geste ne passe pas par la boucle modale de
Windows (qui exigerait `WS_THICKFRAME`) mais par `WM_LBUTTONDOWN`, `SetCapture`,
`WM_MOUSEMOVE` (`SetWindowPos` puis `UpdateWindow`) et `WM_LBUTTONUP` ou
`WM_CAPTURECHANGED` (`platform/windows/events/grip.rs`). Les bords d’une fenêtre
redimensionnable restent ceux de Windows (`WM_SIZING` applique les mêmes proportions).
Code : `window/change.rs` (types et géométrie communs, règle de la poignée
`grip_size`), `platform/windows/window/change.rs`, `platform/windows/events/grip.rs`,
`platform/macos/window/change.rs`, `platform/linux/shell/facts.rs`,
`platform/linux/{wayland,x11}/window/change.rs`. Tests :
`platform/windows/tests/change.rs` (fenêtre cachée, geste d’une poignée sur une fenêtre
fixe et test de position), `window/change.rs` (géométrie, règle de la poignée),
`ui/tests/runtime/basics.rs` (poignée, poignée dans une zone de déplacement), `platform/headless/tests/window.rs`
(`HeadlessApp::window_changed`).

Une ouverture avec un ID déjà utilisé renvoie `Error::DuplicateWindowId`, des
options invalides `Error::InvalidWindowOptions`. Les erreurs de création, de
chargement et de rendu arrêtent actuellement la session entière ; le
`karbon_ui::Result` renvoyé par `run`/`run_windows` permet de les traiter :

```rust,ignore
match platform::run_windows(windows, ExitPolicy::LastWindowClosed) {
    Ok(()) => {}
    Err(karbon_ui::Error::DuplicateWindowId(id)) => eprintln!("ID réutilisé : {id:?}"),
    Err(karbon_ui::Error::Panicked { message }) => eprintln!("panique : {message}"),
    Err(error) => return Err(error),
}
```

Les fenêtres déjà créées sont nettoyées avant le retour.

## Dialogues de fichiers

`WindowCommand::FileDialog(id, TaskId, FileDialog)` montre le sélecteur natif du
système, modal à la fenêtre `id` : ouvrir un fichier (`FileDialog::open_file`), plusieurs
(`open_files`), choisir un dossier (`pick_folder`) ou enregistrer sous (`save_file`),
avec un titre, des types de fichiers (`filter("Audio", &["mp3", "flac"])`), un nom
proposé et un dossier de départ. Le choix revient comme le résultat d'une tâche, à
`Application::task_completed`, sous le `TaskId` donné : `FileDialogResult { paths }`
(vide si l'utilisateur annule), ou un échec si le dialogue n'a pas pu s'ouvrir. Rien
n'est livré si la fenêtre est fermée entre-temps.

```rust,ignore
self.commands.push(WindowCommand::FileDialog(
    WindowId::MAIN,
    TaskId(900),
    FileDialog::pick_folder().title("Ajouter un dossier"),
));
// …
fn task_completed(&mut self, result: TaskResult) -> bool {
    if result.id == TaskId(900) {
        if let Ok(choice) = result.into_value::<FileDialogResult>() {
            self.import(choice.paths);
        }
    }
    true
}
```

| Backend | Dialogue | Notes |
| --- | --- | --- |
| Windows | `IFileOpenDialog`, `IFileSaveDialog` | Boucle modale montrée hors des emprunts de l'application ; la fenêtre continue de se dessiner. Extension du premier type ajoutée au nom tapé sans extension. |
| macOS | `NSOpenPanel`, `NSSavePanel` en feuille | Types par `UTType` (macOS 11 et plus) ; non essayé sur un Mac. |
| Linux | Portail XDG `org.freedesktop.portal.FileChooser` | Appel et attente sur un thread (libdbus), choix rendu par le réveil des tâches ; parent `x11:<XID>` sous X11, `wayland:<poignée>` sous Wayland (`xdg-foreign` v2, sinon v1 comme GNOME 42) ; à essayer à la main (exemple `system_dialogs`). |

## Icône de la fenêtre et zone de notification

Deux choses distinctes :

- l'**icône de la fenêtre** (`WindowOptions::icon`), celle de la barre des tâches,
  d'Alt+Tab et du Dock ;
- l'**icône de la zone de notification** (`WindowCommand::Tray`), avec son menu : la
  zone de notification de Windows, la barre des menus de macOS, un `StatusNotifierItem`
  sous Linux. Elle permet de garder l'application ouverte fenêtre masquée.

Exemple complet : `cargo run --release --example tray_icon`
([examples/tray_icon.rs](../examples/tray_icon.rs)).

### En cinq étapes

```rust,ignore
use karbon_ui::window::{TrayEvent, TrayIcon, WindowChange, WindowCommand, WindowId, WindowOptions};

const ICON: &[u8] = include_bytes!("../assets/icon.png");
const SHOW: u32 = 1; // identifiants des entrées, choisis par l'application
const QUIT: u32 = 2;

// 1. L'icône de la fenêtre, à l'ouverture.
platform::run(WindowOptions { icon: Some(ICON), ..WindowOptions::DEFAULT }, app)

// 2. L'icône de la zone de notification : une commande, comme toutes les commandes de
//    fenêtre (rendue par `take_window_commands`). La pousser dès la construction de
//    l'application suffit : elle est lue après le chargement de la fenêtre.
self.commands.push(WindowCommand::Tray(
    WindowId::MAIN,
    Some(TrayIcon::new(ICON).tooltip("Lecteur")
        .item(SHOW, "Afficher")
        .separator()
        .item(QUIT, "Quitter")),
));

// 3. Les clics et les choix du menu arrivent ici, même fenêtre masquée.
fn tray_event(&mut self, event: TrayEvent) -> bool {
    match event {
        TrayEvent::Activated | TrayEvent::MenuItem(SHOW) => self.commands.push(
            WindowCommand::Change(WindowId::MAIN, WindowChange::Focus), // montrer, premier plan
        ),
        TrayEvent::MenuItem(QUIT) => self.commands.push(WindowCommand::Exit),
        TrayEvent::MenuItem(_) => {}
    }
    false // `true` reconstruit la vue
}

// 4. « Fermer » masque au lieu de quitter (bouton de la barre de titre de l'application) :
self.commands.push(WindowCommand::Change(WindowId::MAIN, WindowChange::Visible(false)));

// 5. Changer un texte du menu ou l'infobulle : renvoyer la commande, l'icône est mise à
//    jour sur place. La retirer : `WindowCommand::Tray(WindowId::MAIN, None)`.
```

### Ce que fait chaque plateforme

Les règles sont les mêmes partout, pour qu'une application n'ait qu'un comportement à
écrire : le **clic principal** remet `TrayEvent::Activated`, le **clic secondaire**
ouvre le menu, le **choix d'une entrée** remet `TrayEvent::MenuItem(id)`. Une entrée
grisée (`disabled_item`) ou un séparateur ne remettent rien.

| | Windows | macOS | Linux |
| --- | --- | --- | --- |
| Où | Zone de notification (Windows 11 range les nouvelles icônes sous « ^ » ; l'utilisateur peut l'en sortir) | Barre des menus, à droite | Zone de l'hôte `StatusNotifierItem` : KDE Plasma, Xfce, Cinnamon, MATE, LXQt, Budgie ; GNOME avec l'extension AppIndicator (présente sous Ubuntu) |
| Clic principal | `Activated` (aussi Entrée sur l'icône sélectionnée au clavier) | `Activated` | `Activated` (méthode `Activate`) |
| Menu | Clic droit ; menu Win32 au thème du système (sombre si Windows l'est) | Clic droit ou Contrôle + clic, sous l'icône | Clic droit, dessiné par l'hôte (`com.canonical.dbusmenu`), à son thème |
| Infobulle | 127 caractères au plus | Infobulle du bouton | Titre et infobulle de l'élément (le nom de l'application si vide) |
| `template` | ignoré | Image « modèle » : seule la forme compte, teinte comme la barre des menus | ignoré |
| Taille de l'image | 16 px à 100 %, 24 à 150 %, 32 à 200 % | 18 points (36 px sur Retina) | L'hôte choisit parmi 16, 22, 24, 32, 48, 64 px |
| Explorateur ou hôte redémarré | Icône reposée (`TaskbarCreated`) | sans objet | Icône réinscrite quand le gardien `org.kde.StatusNotifierWatcher` revient |

| | Windows | macOS | Linux X11 | Linux Wayland |
| --- | --- | --- | --- | --- |
| `WindowOptions::icon` | Barre des tâches, Alt+Tab, menu de la fenêtre (`WM_SETICON`, deux tailles au DPI de la fenêtre) | Icône du Dock hors paquet `.app` (un paquet garde la sienne) | `_NET_WM_ICON` (16 à 128 px) | sans effet : le compositeur prend l'icône de l'entrée `.desktop` dont le nom, ou `StartupWMClass`, est le nom de l'exécutable (`app_id`) |

### Images

- **Icône de la fenêtre** : une image carrée d'au moins 256 px. Sous macOS, l'icône du
  Dock suit le dessin des icônes macOS (carré arrondi avec marge) : donner une image
  propre à macOS (`#[cfg(target_os = "macos")]`) si celle des autres systèmes n'en a pas.
- **Zone de notification** : une forme simple sur fond transparent, d'au moins 64 px,
  lisible à 16 px. Un fond sombre ou clair plein se perd dans une barre de l'autre
  couleur. Sous macOS, `.template(true)` sur une forme d'une seule couleur donne une
  icône comme les autres de la barre des menus.
- Formats : ceux du décodeur de la plateforme (PNG, JPEG, BMP partout). Une image
  illisible est signalée sur la sortie d'erreur (`karbon-ui: tray icon unavailable`,
  `karbon-ui: window icon unavailable`) et ignorée : l'icône précédente reste.

### Icône de l'exécutable et paquets

karbon-ui pose les icônes au lancement ; celle du fichier lui-même vient de la
compilation ou du paquet :

- **Windows** : une ressource `ICON` dans l'exécutable (Explorateur, raccourcis,
  épinglage). Avec la crate `embed-resource` dans `build.rs` :

  ```rust,ignore
  // build.rs ; Cargo.toml : [build-dependencies] embed-resource = "3"
  fn main() {
      if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
          // app.rc : `1 ICON "assets/icon.ico"` (et un bloc VERSIONINFO, nom affiché
          // dans le Gestionnaire des tâches).
          embed_resource::compile("app.rc", embed_resource::NONE).manifest_optional().unwrap();
      }
  }
  ```

- **macOS** : `CFBundleIconFile` du paquet `.app` (fichier `.icns`).
- **Linux** : une entrée `.desktop` (`Icon=`, `StartupWMClass=` nom de l'exécutable)
  et l'icône installée dans `share/icons/hicolor/`. Sans entrée sous Wayland, la fenêtre
  a l'icône générique.

### Limites

- Une icône par fenêtre. Plusieurs fenêtres peuvent en avoir chacune une.
- Pas de sous-menus, de cases à cocher ni d'icônes dans le menu ; pas de notification
  en bulle (voir les notifications du système, à venir).
- Linux : sans hôte `StatusNotifierItem` (GNOME sans l'extension AppIndicator), rien ne
  s'affiche ; un message l'indique sur la sortie d'erreur et l'icône apparaît dès qu'un
  hôte démarre.
- Windows : le menu sombre passe par trois fonctions non documentées d'`uxtheme.dll`
  (celles de l'Explorateur, du Bloc-notes et de Windows Terminal) ; absentes, il reste
  clair.

### Fonctionnement et code

- Types publics : `window/tray.rs` ; `WindowOptions::icon` dans `window/config.rs`.
- Windows : `platform/windows/window/icon.rs` (`HICON` à alpha droit depuis l'image
  décodée en BGRA prémultiplié), `tray.rs` (`Shell_NotifyIconW`, comportement de la
  version 4, infobulle `NIF_SHOWTIP`, menu `TrackPopupMenuEx` de la fenêtre propriétaire,
  `TaskbarCreated`), `menu_theme.rs` (`SetPreferredAppMode(AllowDark)`,
  `AllowDarkModeForWindow`, `FlushMenuThemes`, ordinaux 135, 133 et 136).
- macOS : `platform/macos/window/tray.rs` : `NSStatusItem` carré, image `NSImage`
  (18 points, `template`), bouton qui envoie son action aux deux relâchements ; au clic
  secondaire, le menu est attaché le temps d'un `performClick:` puis détaché. Icône du
  Dock par `NSApplication.applicationIconImage` quand `NSBundle.mainBundle` n'a pas
  d'identifiant.
- Linux : `platform/linux/tray/` : un thread et une connexion privée au bus de session
  par icône (`service.rs`) ; nom `org.kde.StatusNotifierItem-<pid>-<n>`, objets
  `/StatusNotifierItem` (`item.rs`) et `/MenuBar` (`dbusmenu.rs`, version 3), inscription
  auprès du gardien et de nouveau à son retour (`NameOwnerChanged`). Clics remis par un
  canal et le réveil de la boucle, livrés par `Session::tasks`. Images en ARGB à alpha
  droit (`pixmap.rs`), aussi pour `_NET_WM_ICON` (`x11/window/top_level.rs`).
- Tests : `platform/windows/tests/tray.rs` (fenêtre cachée : icônes posées aux deux
  tailles, conversion des pixels, clics remis à l'application ; l'icône n'est pas ajoutée
  au vrai Shell), `platform/linux/tray/tests.rs` (pixels, identifiants du menu).
  Essayé à la main sous Windows 11 (exemple `tray_icon`, RustMusic). macOS et Linux :
  compilés et vérifiés par clippy depuis Windows, **pas encore essayés** sur un Mac ni sur
  un bureau Linux ; à faire avec l'exemple `tray_icon` (clic principal, clic secondaire,
  « Pause » qui change le texte du menu, « Quitter »).

## Choisir quand l’application s’arrête

| Politique | Comportement |
| --- | --- |
| `LastWindowClosed` | Une fenêtre peut survivre à celle qui l’a ouverte ; sortie après la dernière fermeture. |
| `MainWindowClosed(WindowId(1))` | Sortie à la fermeture de cette fenêtre, avec destruction des fenêtres restantes. |

La fenêtre désignée comme principale doit être présente dans la liste
initiale. Une liste initiale vide ou contenant des IDs en double est refusée.
Une demande `Exit` arrête la session quelle que soit la politique.

## État et ressources sous Windows

Une seule boucle attend les événements de toutes les fenêtres du thread UI.
Chaque fenêtre possède son runtime, son renderer, sa surface, son thème, ses
états de barre de titre, son ombre, ses timers, sa saisie UTF-16/IME et son pont
UI Automation. L’OS conserve ses services globaux, comme le presse-papiers.

`state.rs` associe le handle natif au contexte de la fenêtre. Un scope choisit
le contexte au début du message et restaure le précédent à la fin : si Windows
appelle B pendant un traitement de A, le retour reprend bien le contexte A.
Le registre n’est pas emprunté pendant les callbacks. Le contexte de création
couvre aussi les messages synchrones émis par `CreateWindowExW`.

`WM_DESTROY` désarme les timers et déconnecte l’accessibilité et l’ombre de la
fenêtre concernée. À `WM_NCDESTROY`, le contexte est retiré du registre et ses
ressources sont libérées après la fin des scopes qui l’utilisent encore. La
fermeture réveille la boucle sans déposer de `WM_QUIT` pour chaque fenêtre.
Cela permet à une deuxième fenêtre de continuer et évite un `WM_QUIT` résiduel
si l’application relance une session sur le même thread.

Le décodeur GDI+ et l’appartement COM restent vivants jusqu’au nettoyage de la
session. Une seconde boucle imbriquée sur le même thread est refusée. Appeler
l’entrée du moteur sur le thread principal prépare la portabilité future ;
les applications de fenêtre ne sont pas tenues d’implémenter `Send`.

Les règles natives utilisées ici sont documentées par Microsoft :
[destruction finale](https://learn.microsoft.com/en-us/windows/win32/winmsg/wm-ncdestroy),
[messages envoyés pendant GetMessage](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getmessagew)
et [réveil par PostMessageW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-postmessagew).

## Validation et limites

Neuf tests natifs créent des fenêtres cachées et vérifient l’isolation des
compteurs, des thèmes et du décodage UTF-16, la réentrance entre fenêtres, les
ouvertures/fermetures différées, les politiques de sortie, les erreurs et la
libération des applications. Deux scénarios envoient réellement un message
depuis un autre thread pendant la boucle : fermeture de la dernière fenêtre
et demande de sortie sans reconstruction de la vue. Un secours borné détecte
une régression du réveil au lieu de suspendre la suite.

Un test de la démo valide les arbres et les identifiants des fenêtres produites.
La suite compte 154 tests au total : 153 dans la bibliothèque et un dans la démo.
La recette visuelle simultanée des fenêtres, de leurs ombres, de leurs champs
de saisie et du lecteur d’écran reste à faire ; les tests cachés ne la remplacent
pas. Aucun gain de RAM ou de temps d’image global n’est annoncé à cette étape.

Le [service de tâches de fond](tasks.md) fournit maintenant des résultats
associés à chaque fenêtre, avec annulation à la fermeture. L’API de fenêtres
ne fournit pas encore la liste des écrans, les relations propriétaire/dialogue natif, l’annulation d’une fermeture pour
confirmer une sauvegarde, le docking ou les backends macOS/Linux. Chaque fenêtre
possède actuellement son renderer ; le partage de ressources graphiques entre
fenêtres reste une optimisation à mesurer.

Les tâches préparent la validation d’un premier écran RustMusic et du chargement
de ses ressources, avec des mesures de mémoire et de temps d’affichage.
