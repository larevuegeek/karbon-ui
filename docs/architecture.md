# Organisation du projet

Le projet est une bibliothèque Rust, publiée sous le nom Cargo `karbon-ui`
(anciennement `window-engine`). Les petits programmes de `examples/` utilisent
uniquement l’API publique.

## Carte des modules

Règles communes : un dossier par thème ; un module qui a des enfants est un
dossier `foo/mod.rs`, qui ne contient que la documentation du module, les `mod`, les `pub use` et la sélection par `cfg` (la logique va dans un fichier nommé d’après son rôle) ; les tests unitaires d’un module sont dans `foo/tests.rs`
(ou `foo/tests/<thème>.rs`) ; les fichiers restent sous ~500 lignes. Le moteur
(`application`, `window`, `task`, `resources`, `ui`) ne dépend d’aucun OS ; tout
ce qui touche un OS est sous `platform/`.

```text
src/
├── lib.rs                  Modules publics et macros ui!, ui_file!, styles!, css_file!
├── error.rs                Error, ImageError, Result
├── application/            Trait Application, ViewContext, ressources, actions et messages effacés
├── window/                 WindowOptions, identités, commandes, validation des descripteurs
├── task/                   Task, annulation, résultats ; executor/ : pool de threads borné
├── resources/              Cache CPU d’images ; image_loader/ : limites et lecture commune
├── diagnostics.rs          API publique des aperçus, mesures et export PNG
├── ui/                     Moteur indépendant de l’OS
│   ├── prelude.rs          Imports usuels pour écrire une vue
│   ├── core/               Element, contrat Widget, géométrie, unités, thème, recettes de style, hachage
│   ├── view/               Constructeurs de composition (row, column…) et ViewStyle
│   ├── template/           Support des macros : Node (node/), contexte, slots, HTML
│   ├── styling/            Cascade CSS : properties/, resolve/, tokens/, color/, texte, variables
│   ├── layout/             Mesure et placement : flex/, grilles, pistes, positions, scroll, mémoire des mesures, containment.rs (`content-visibility`)
│   ├── animation/          Effets, préréglages, courbes, étapes par propriété, minutage ; timeline/ : horloges et pistes
│   ├── components/         Contrôles ; un dossier par contrôle complexe (select/, slider/…)
│   ├── overlays/           Modales, popovers et menus, tooltips, couches
│   ├── runtime/            UiRuntime : événements, focus, accessibilité, glisser-déposer, painting/ (dont cull.rs : sous-arbres hors de leur découpe), text_cache/
│   ├── rendering/          Scene (scene/), pinceaux, filtres, ombres, SVG, transformations
│   ├── kit/                Composants stylés prêts à l'emploi, thème et feuille du kit (voir kit.md)
│   ├── bootstrap.rs        Actions des composants Bootstrap (code généré par ui-macros)
│   └── tests/              Tests de contrat du moteur, par thème (voir tests.md)
└── platform/
    ├── mod.rs              Entrées publiques, sélection unique du backend (`Native`)
    ├── backend.rs          Contrat d’un backend : traits `Backend`, `Offscreen` et obligations
    ├── offscreen.rs        Protocole commun : aperçus, mesures et tâches hors écran
    ├── headless/           Backend sans fenêtre (toutes cibles) et `HeadlessApp`
    │   ├── app/            Harnais HeadlessApp : pilotage, entrées, inspection
    │   ├── raster/         Rastériseur logiciel d’une Scene
    │   ├── text/           Métriques de texte déterministes
    │   └── tests/          Tests du harnais (contrat « application headless »)
    ├── macos/              Backend macOS (Core Graphics, Core Text, ImageIO, AppKit)
    │   ├── window/         Boucle AppKit, sessions, vue et délégué, entrées et IME, menus
    │   ├── accessibility/  Pont NSAccessibility (VoiceOver)
    │   ├── graphics/       Renderer Core Graphics/Core Text : canvas, texte, couches, filtres CPU
    │   ├── metal/          Renderer GPU Metal des fenêtres : shaders, encodage, filtres, atlas, textures
    │   ├── fonts.rs        Polices embarquées et familles système Core Text
    │   ├── imaging.rs      Décodage d’images et export PNG par ImageIO
    │   └── tests/          Suite minimale du backend
    └── windows/            Backend Windows (modèle des futurs backends)
        ├── window/         Boucle, création et état des fenêtres, minuteries, tâches, ombre, DWM
        ├── events/         Procédure de fenêtre : pointeur, clavier, zone non cliente, texte/IME, presse-papiers
        ├── present/        État de dessin, peinture, composition DirectComposition ou surface layered
        ├── graphics/       Renderer Direct2D/DirectWrite : canvas, typographie, filtres, chaîne d’échange
        ├── accessibility/  Pont UI Automation : arbre, synchronisation, fournisseurs COM, patterns
        ├── offscreen.rs    Implémentation d’Offscreen et compteurs du processus
        ├── imaging.rs      Décodage d’images et export PNG par GDI+
        ├── fonts.rs        Collection DirectWrite des polices embarquées
        ├── ffi.rs          Types Win32 partagés, appartement COM
        └── tests/          Suite minimale du backend (fenêtres cachées)
examples/                   rustmusic_html, bootstrap_demo et trois exemples d’API
crates/ui-macros/           Compilateur .ui et CSS, sans dépendance tierce
tests/                      Contrat des templates et des styles : css/, templates/,
                            frameworks/, interaction/ (une crate par thème)
docs/archive/               Anciennes recettes et mesures, non exécutables ici
```

Le [plan de src/ui](../src/ui/README.md) détaille les composants et le moteur ;
l’[organisation des tests](tests.md) décrit le contrat testé. Les anciens chemins
d’import (`ui::element`, `ui::widgets`, `ui::scene`…) restent réexportés dans
`ui/mod.rs`, sans dupliquer leur code : le code généré par `ui-macros` les utilise.

## Organiser une application qui utilise le socle

Une petite application peut tenir dans un fichier, comme
[`examples/simple_ui.rs`](../examples/simple_ui.rs). Pour une application plus
importante, on peut séparer les responsabilités ainsi :

```text
mon_application/
├── mod.rs          Déclarations du module
├── app.rs          Implémentation d’Application
├── view.rs         Structure de la page
├── state.rs        Données et actions
├── styles.rs       Recettes et palette
├── components/     Compositions réutilisables et widgets personnalisés
└── assets/         Ressources propres à cette application
```

Pour changer une disposition, modifier la vue. Pour un espacement ou une
couleur partagés, modifier les styles. Pour le résultat d’un clic, modifier
l’état applicatif. Pour un dessin spécifique, implémenter `Widget`.

## Messages : des widgets à `update`

Une application déclare `type Message` et le reçoit dans
`update(&mut self, message) -> bool` (`true` reconstruit la vue). Chaque
élément possède ses propres gestionnaires :

- `Action` (public, dans le prélude) pour les activations simples : clic,
  Entrée/Espace, commandes d’accessibilité *Invoke*, *Toggle* et *Select*. Il
  clone le message donné au constructeur (`Button::new("Ok", Msg::Save)`).
- `Handler<T>` (interne) pour les widgets porteurs de valeur : fonction
  `Fn(T) -> M` partagée par `Rc` (`TextInput` : `String`, `Slider` : `f64`,
  `Select` : `u64`, case et interrupteur : `bool`, `DropTarget` : `DropEvent`,
  menu contextuel : `ContextMenuRequest`).

Les deux produisent un `Message` effacé (`Box<dyn Any>` et nom du type émis).
Rust ne permet pas qu’un même paramètre accepte à la fois une valeur et une
fermeture (cohérence des traits) : les actions simples prennent donc une valeur,
les widgets porteurs de valeur une fonction.

Lorsqu’un événement arrive, le runtime lit le gestionnaire dans l’arbre
**courant** (widget, slider retrouvé par son ID, cible de dépôt, couche
ouverte…), construit le message, puis le passe à un adaptateur objet interne,
`DynApplication`. L’implémentation générique pour chaque `A: Application`
convertit le message en `A::Message` et appelle `update`. Le backend ne
manipule que `Box<dyn DynApplication>` : `platform::run`, `WindowDescriptor::new`,
`render_benchmark`, `render_preview` et `UiRuntime::new` reçoivent l’application
par valeur, et chaque fenêtre peut utiliser un type de message différent.

Remplacer l’arbre remplace aussi ses gestionnaires : pas de registre global,
pas d’identifiant périmé, et les lignes de listes virtualisées construites
tardivement ou les couches (modales, popovers, menus) portent leurs propres
gestionnaires. Un élément construit hors d’une vue reste valide. Les
interactions en cours d’un slider ou d’un Select ne comparent plus leurs
gestionnaires entre deux reconstructions : seules la configuration et la
présence du contrôle comptent.

Un message d’un autre type que `Application::Message` est une erreur de
programmation : l’adaptateur panique avec un message explicite (type émis et
type attendu). Le backend Windows transforme cette panique en
`Error::Panicked` renvoyée par la boucle de messages.

## De l’application aux pixels

Les templates `ui!` / `ui_file!` sont du HTML compilé (balises natives, blocs
`{#if}`/`{#each}`, Rust entre accolades) traduit à la compilation.
Ils produisent le même arbre que les appels Rust ; aucun interpréteur ou backend
graphique supplémentaire. Les styles `styles!` sont des fonctions typées.
Le [guide des templates](templates.md) décrit les composants, slots, includes
et les limites actuelles.

1. `Application::view()` compose un arbre d’`Element`.
2. Le runtime résout les styles hérités, états et media queries, puis prépare
   le layout lorsque sa géométrie change. Les changements de couleur ne le relancent pas.
3. Les widgets peignent dans une `Scene` indépendante de l’OS.
4. Le backend (Windows, ou le rastériseur headless) dessine la surface.
5. Les événements reviennent au runtime, qui construit un message et le
   transmet à `Application::update`.

La composition utilise le même arbre et le même moteur de layout que les
constructeurs historiques. Le backend macOS a ses services hors écran, une boucle
AppKit (IME, menus, accessibilité) et un renderer GPU Metal pour ses fenêtres ; le backend
Linux a ses services hors écran (Cairo, Pango), deux boucles de fenêtres (Wayland, X11)
sur un socle commun et un renderer GPU Vulkan. La
[frontière des backends](#frontière-des-backends) décrit leur place.

Le compilateur CSS est dans `crates/ui-macros/src/css/`. Les expressions sont
des arbres statiques, les sélecteurs conditionnels des prédicats natifs. Le layout
sépare flex, pistes de grille, positionnement et clips. Le backend Windows utilise
Direct2D 1.1 : les filtres conservent leurs listes de commandes et graphes natifs ;
les ombres de boîte conservent un masque CPU. Voir [la couverture CSS](css.md).

### Mémoires de la préparation

`UiRuntime::prepare_at` (`runtime/prepare.rs`) garde trois mémoires, sans effet sur le
résultat (vérifié par les tests d’équivalence, voir [tests](tests.md)) :

- **Mesures d’une passe de layout** (`layout/memo.rs`, active entre
  `LayoutPass::begin` et sa fin) : un appel de `layout::measure` déjà fait avec les
  mêmes contraintes (place offerte, dimensions et limites que le parent modifie le temps
  d’une mesure, marges) rend sa taille et remet sur l’élément sa taille naturelle et ses
  limites relatives résolues. Ne sont jamais mémoïsés les éléments dont un descendant
  porte un état écrit par la mesure et relu sans la refaire (limites relatives,
  révélation, widget conteneur, feuille relue par la ligne de base) ni les lignes des
  listes virtuelles.
- **Mesures de texte d’une image à l’autre** (`runtime/text_cache/`) : le mesureur du
  backend est enveloppé (`Cached`) ; une mesure faite à la largeur `W` dont la ligne la
  plus large mesure `w` sert pour toute largeur entre `w + 0,5` et `W` (coupures
  gloutonnes), un texte justifié seulement à la même largeur. Vidée quand l’échelle
  d’affichage change.
- **Faits du pointeur** (`styling/resolve/matching.rs`, `pointer_facts`) : la cascade ne
  lit le pointeur qu’à travers deux faits par élément (pointeur rogné, survol) ; quand
  seul le pointeur a bougé et que ces faits n’ont pas changé, elle n’est pas relancée
  (décision dans `runtime/restyle.rs`, qui résout aussi les styles de la vue et reporte
  sur une vue reconstruite les ombres et tailles de conteneurs de l’ancienne).

S'y ajoute la **reprise des sous-arbres mémorisés** (`view/lazy.rs`, attribut `memo`
des gabarits) : la vue pose un élément vide à la place d'un sous-arbre dont la clé n'a
pas changé, le runtime y remet l'ancien sous-arbre (`transplant`) ; ses mesures de
l'image précédente (`layout/memo.rs`, `carried`) et son placement (`layout/lazy.rs`,
translaté s'il a seulement bougé) sont repris tant que la résolution des styles n'y
change aucune métrique (`StyleChanges::layout` sous l'élément).

**Layout incrémental** (`core/element/memo.rs` `LayoutCache`, `layout/memo.rs`
`carried`, `layout/lazy.rs`) : chaque conteneur retient ses mesures et son placement ;
une image suivante qui le mesure avec les mêmes contraintes et le place avec les mêmes
entrées les reprend (translaté s'il a seulement bougé), tant que la résolution des
styles n'a changé aucune métrique dans son sous-arbre (`settled`, remis à `false` en
remontant comme les bits « sale » des navigateurs). Seuls les sous-arbres dont la mesure
ne dépend que des contraintes en profitent (pas de limites relatives, de révélation, de
ligne de base relue ni de liste virtuelle dessous) ; avant de placer un conteneur
repris, `stale_descendants` le remesure si sa dernière mesure de la passe n'est pas
celle qui a laissé ses descendants dans leur état. Un sous-arbre mémorisé explicitement
garde ses mesures sans cette garantie. Un séparateur déplacé oublie tout (`unsettle`).

**Résolution partielle des styles** (`styling/resolve/partial.rs`) : quand seuls le
pointeur et des animations ont changé depuis la dernière résolution (même arbre, même
signature hors pointeur, sans conteneur de requêtes changé), la cascade ne refait que
les éléments dont les faits du pointeur ont changé, ceux dont une transition ou une
animation de style est en cours, leurs sous-arbres et les frères suivants qui lisent
un frère précédent, en passant par leurs ancêtres. Après une reconstruction, un
sous-arbre mémorisé repris saute la cascade tant que l'empreinte de ses entrées
(typographie héritée, variables, position, faits des ancêtres) est la même et qu'aucun
de ses éléments n'a changé d'état (`MemoState::style_dirty`). Enfin, si le layout change
ce que le pointeur touche (éléments d'une vue reconstruite, sans position lors de la
résolution), les éléments touchés sont restylés dans la même image : le survol ne
disparaît plus à chaque reconstruction.

`UiRuntime::set_incremental(false)` les désactive (et refait la seconde construction de
la vue à l’ouverture d’une couche) : c’est la référence des tests d’équivalence. Les
tables internes du moteur sont hachées par `core/hash.rs` (`FastMap`).

## Frontière des backends

Tout ce qui précède l’étape 4 est indépendant de l’OS : `UiRuntime` reçoit des
`UiEvent`, mesure le texte par le trait `TextMeasurer` et peint une `Scene`.
Le contrat complet d’un backend est écrit en un seul endroit,
`src/platform/backend.rs` : deux traits internes pour les services typés, et le
tableau des obligations de la boucle d’événements.

| Service | Trait | Windows | macOS | Linux | Headless |
| --- | --- | --- | --- | --- | --- |
| Boucle d’événements et fenêtres | `Backend::run_windows` | `window/`, `events/`, `present/`, `accessibility/` | AppKit (`window/`) | Wayland (`wayland/`) ou X11 (`x11/`) sur le socle `shell/` | `Error::Unsupported` |
| Mesure du texte | `Offscreen: TextMeasurer` | DirectWrite (`graphics/measure.rs`) | Core Text (`graphics/text/`) | Pango (`graphics/text/`) | métriques fixes (`HeadlessTextMeasurer`) |
| Décodage d’images (`Resources`, `ImageDecoder`) | `Offscreen::resources` | GDI+ (`imaging.rs`) | ImageIO (`imaging.rs`) | gdk-pixbuf (`imaging.rs`) | en-tête validé, aplat gris de même taille |
| Polices embarquées | `Offscreen::load_fonts` | collection DirectWrite privée (`fonts.rs`) | descripteurs Core Text privés, WOFF2 compris (`fonts.rs`) | configuration fontconfig privée, WOFF2 décodé par FreeType (`fonts.rs`) | validées, sans effet sur les métriques |
| Scène → pixels (aperçus, mesures) | `Offscreen::render` | Direct2D (GPU ou WARP) + relecture | Core Graphics sur bitmap, filtres CPU (`graphics/`) ; fenêtres : Metal (`metal/`) | Cairo sur surface image, filtres CPU (`graphics/`) ; mode `Gpu` : Vulkan (`vulkan/`), fenêtres : Vulkan (`vulkan/`) | rastériseur logiciel (`raster/`) |
| Compteurs du processus | `Backend::process_sample` | `GetProcessMemoryInfo`, `GetProcessTimes` | `task_info`, `proc_pid_rusage`, `getrusage` | `/proc/self/status`, `getrusage` | zéros |
| Export PNG | `Backend::write_png` | GDI+ | ImageIO | Cairo | encodeur intégré (deflate non compressé) |

`platform/mod.rs` choisit l’implémentation **en un seul endroit**
(`type Native = windows::Windows` sous Windows, `macos::MacOs` sous macOS,
`linux::Linux` sous Linux, `headless::Headless` ailleurs) ;
`platform::run`, `run_windows` et `diagnostics::{render_preview,
render_benchmark, write_png}` sont écrits une fois et délèguent à `Native`. Le
protocole hors écran (chargement, tâches de fond, chauffe, phases de mesure) est
commun : `platform/offscreen.rs`, générique sur `Backend`. Le harnais
[`HeadlessApp`](headless.md) réutilise le même moteur headless et la même boucle
de tâches sur toutes les cibles.

Le presse-papiers, les minuteries, le réveil du thread UI et l’accessibilité ne
sont pas des traits : ce sont des obligations internes de `run_windows`, écrites
contre l’API publique du runtime (tableau détaillé dans `backend.rs`) :

- valider les descripteurs (`window::validation`, commun à tous les backends) ;
- traduire les entrées natives en `UiEvent` (positions logiques, DPI appliqué) ;
- honorer `EventResult` : capture et libération du pointeur, requêtes de
  presse-papiers (`ClipboardRequest::Write` puis `EditCommand::DeleteSelection`
  pour couper, `Read` renvoyé en `UiEvent::Text`), curseur (`UiRuntime::cursor`) ;
- programmer une minuterie à `UiRuntime::next_wake` et envoyer `UiEvent::Timer` ;
- créer un `TaskScope` par fenêtre avec une fonction de réveil qui poste un
  message à la boucle, puis livrer les résultats sur le thread UI ;
- exposer `UiRuntime::accessibility` et appliquer `accessibility_action` au
  lecteur d’écran de la plateforme ;
- dessiner chaque `DrawCommand` de la `Scene` et présenter la surface ; une
  fenêtre transparente sans test alpha natif peut interroger
  `ui::rendering::scene::coverage`.

### Services partagés des renderers macOS et Linux

`src/platform/shared/` (compilé sous macOS et Linux) réunit ce que Core Graphics, Metal et
Cairo font de la même façon, sans dépendance à l’OS : filtres CSS sur CPU et facteur de
réduction des groupes à grand flou (`filters.rs`), structure des portées d’une scène
(`scopes.rs` : fin d’une portée, portée d’un seul élément peint), rectangles arrondis
normalisés et extrémités des dégradés (`geometry.rs`), empreintes des caches
(`keys.rs`) et cache des mises en page de texte (`text_cache.rs` : clé, tailles
mesurées, ellipse de début), générique sur la pile de texte du backend (trait
`Typesetter`, implémenté par les polices Core Text et Pango).

### Backend Linux

Pile choisie par le propriétaire le 2026-11-05 : Wayland natif d’abord puis X11, Pango
(HarfBuzz, fontconfig, FreeType) pour le texte, Cairo pour le rendu CPU (hors écran,
exports, repli), Vulkan pour le GPU des fenêtres. Liaisons C écrites à la main
(`linux/ffi/`), liées dynamiquement ; `libc` pour `memfd_create` et `getrusage`.

- Compilation : paquets de développement de `cairo`, `pango` (`pangocairo`, `pangoft2`),
  `fontconfig`, `freetype2`, `gdk-pixbuf-2.0`, `glib-2.0`, `wayland-client`,
  `wayland-cursor`, `xkbcommon`, `dbus-1` (Debian/Ubuntu : `libcairo2-dev
  libpango1.0-dev libgdk-pixbuf-2.0-dev libwayland-dev libxkbcommon-dev libdbus-1-dev`,
  qui tirent les autres).
- Exécution : les bibliothèques partagées correspondantes, présentes sur tout bureau
  GNOME, KDE ou Xfce ; FreeType compilé avec brotli pour les polices WOFF2.
- Texte : `PangoLayout` coupe les lignes (mots puis caractères), met en forme par HarfBuzz
  et justifie ; le placement des lignes (hauteur naturelle = ascendante + descendante +
  interligne de la police de base), l’alignement, les ellipses par ligne et les
  décorations sont faits par le backend, comme sous macOS. Pango retire de la largeur
  d’une ligne la demi-espace de lettres de ses bords, mais coupe en la comptant : la
  largeur rendue la reprend (`letter_spacing` par ligne non vide, largeur de DirectWrite).
  Mise en page sans hinting ni arrondi des positions : la mesure ne dépend pas de
  l’échelle du dessin.
- Polices embarquées : chaque fichier est écrit dans un `memfd` ouvert par
  `/proc/self/fd/N`, enregistré dans une configuration fontconfig propre au renderer sous
  une famille privée (`karbon-embedded-<famille>`) : une police installée du même nom ne la
  remplace pas. Les WOFF et WOFF2 sont décodés par FreeType (HarfBuzz ne lit que les
  polices brutes). `Segoe UI`, `system-ui`… → `system-ui` ; `Consolas`, `monospace`… →
  `monospace` (sans traduction, fontconfig proposait une famille au nom voisin).
- Réglages du bureau (`desktop.rs`, GSettings par GIO, schéma du bureau en cours :
  GNOME et ses dérivés, MATE ; aucun ailleurs) : dans les fenêtres, `system-ui` et
  `monospace` désignent les polices du bureau (GNOME : « Inter 10 » → Inter) quand elles
  sont installées ; aperçus et exports gardent les génériques de fontconfig
  (reproductibles : l’export 1280 × 900 de RustMusic changerait sinon). Animations
  désactivées par le bureau (`enable-animations`) → mouvement réduit, relu quand une
  fenêtre reprend le focus, comme sous macOS et Windows.
- Rendu Cairo : surface ARGB32 (BGRA prémultiplié en mémoire, celui du moteur), images du
  moteur lues sans copie. Cairo n’a pas d’opacité globale : l’opacité d’un groupe d’un
  seul élément multiplie chaque source, un groupe de plusieurs éléments est un groupe
  Cairo borné au contenu ; `mask-image` est un groupe recomposé par `cairo_mask`,
  `background-clip: text` un masque d’alpha des glyphes.

#### Boucles de fenêtres (`linux/shell/`)

`platform::run_windows` choisit Wayland s’il y a un compositeur (`WAYLAND_DISPLAY`),
sinon X11 (`DISPLAY`) ; `KARBON_BACKEND=wayland|x11` impose l’un des deux (X11 sous
XWayland pour l’essayer sur un bureau Wayland). Ce que les deux boucles partagent :
une seule boucle par thread (`RunGuard`), réveil des tâches de fond (`eventfd`),
clavier xkbcommon (`keyboard.rs`), et la fenêtre vue du moteur (`session.rs` :
application, renderers Cairo et Vulkan, taille logique et échelle, barre de titre du
moteur, tâches, réveils, allègement du GPU, accès AT-SPI) que chaque fenêtre de backend
embarque. Sonde commune des deux boucles : `probe.rs`.

#### Boucle X11 (`linux/x11/`, en cours)

- XCB chargé au lancement (`ffi/xcb/`, `dlopen` de `libxcb.so.1`, types transcrits de
  `xcb.h` avec une assertion de taille par structure, d’après les en-têtes) : un système
  sans X11 démarre en Wayland, et aucun paquet de développement n’est requis.
- Connexion (`connection.rs`) : visuel de la racine (TrueColor 24 bits aux masques BGRA
  : pixels du moteur sans conversion) et visuel ARGB 32 bits des fenêtres transparentes,
  atomes ICCCM/EWMH/Motif, échelle du bureau `Xft.dpi` / 96 (ressources X de la racine).
- Fenêtres (`window/`) : titre (`WM_NAME`, `_NET_WM_NAME`), classe, PID, taille minimale
  (`WM_NORMAL_HINTS`), fermeture (`WM_DELETE_WINDOW`) et `_NET_WM_PING` ; avec
  `title_bar`, le gestionnaire dessine la barre de titre, sans, aucune décoration
  (`_MOTIF_WM_HINTS`). Taille reçue en pixels (`ConfigureNotify`), états EWMH (agrandie,
  plein écran, réduite → ressources libérées), agrandir et réduire par messages à la
  racine.
- Image : Vulkan par défaut (surface `VK_KHR_xcb_surface`, mode « fifo » : X11 n’a pas de
  rappel de trame, la présentation suit le balayage de l’écran), Cairo en repli (zones
  modifiées envoyées par `xcb_put_image` en bandes sous la taille maximale d’une
  requête). Sous XWayland : RustMusic 1440 × 1040, 4,2 ms par image Vulkan ; images
  Vulkan et Cairo équivalentes (écart moyen 0,17).
- Entrées (`input.rs`) : pointeur (positions logiques, molette par boutons 4 à 7, Maj
  pour l’horizontal), barre de titre et bords du moteur et zones de déplacement de
  l’application (`shell/chrome.rs`, commun avec Wayland) confiés au gestionnaire par
  `_NET_WM_MOVERESIZE` ; clavier sans extension XKB ni `xkbcommon-x11` : disposition
  compilée par xkbcommon d’après `_XKB_RULES_NAMES`, modificateurs et groupe lus dans
  chaque événement, répétitions du serveur reconnues (relâchement et appui au même
  instant) ; curseurs du thème par `libxcb-cursor` (chargée au lancement). Vérifié sous
  XWayland par des événements synthétiques (clic dans le champ de recherche de
  RustMusic, frappe).
- Presse-papiers (`clipboard.rs`, sélection `CLIPBOARD`) : une fenêtre invisible du
  backend possède la sélection quand l’application copie (`TARGETS`, `UTF8_STRING`,
  `STRING`, `TEXT`, au plus une requête : pas d’envoi par morceaux) ; une lecture attend
  la conversion en `UTF8_STRING` une seconde au plus, en gardant pour la boucle les
  autres événements reçus. Vérifié sous XWayland avec un client XCB (collage, copie).
- Défilement fin (`xinput.rs`, XInput 2.1 par `libxcb-xinput`, chargée au lancement) :
  chaque fenêtre demande en XI2 les déplacements et les boutons des pointeurs maîtres
  (le serveur ne lui envoie plus les équivalents classiques, rejoués par les chemins
  communs de `input.rs`) ; les axes de défilement d’un périphérique (`XIQueryDevice`) sont
  lus à sa première rencontre, une variation d’un incrément vaut un cran (48 px vertical,
  96 px horizontal, comme une molette) ; les crans émulés (`XIPointerEmulated`) sont
  ignorés. Sans XInput 2.1, événements classiques. Décodage testé sur des octets
  construits d’après le protocole (en-têtes `xinput.h` absents) ; XWayland annonce les
  axes 2 et 3, incrément 1.
- Méthode d’entrée (`linux/ime/`, client IBus sur D-Bus, et `x11/ime.rs`) : un contexte
  de saisie par boucle, sur le bus privé d’IBus (`IBUS_ADDRESS`, sinon le fichier
  `~/.config/ibus/bus/<machine-id>-<hôte>-<écran>` comme libibus), à défaut par le portail
  IBus du bus de session. Quand un champ modifiable a le focus, chaque touche va d’abord
  au moteur (`ProcessKeyEvent`, 200 ms au plus) ; ses signaux deviennent les événements
  de composition du moteur (comme sous Wayland) ou du texte, une touche renvoyée
  (`ForwardKeyEvent`) est traitée par l’application. Caret en pixels de l’écran (origine
  de la fenêtre gardée jusqu’au `ConfigureNotify` suivant) ; un clic dans le contenu ou
  la perte du focus valide la composition. Vérifié sous XWayland : touche morte, et
  saisie Unicode Ctrl+Maj+U (composition soulignée puis « é » validé). XIM n’est pas
  géré (choix du 2026-11-06) ; Fcitx (D-Bus) reste à faire.
- `KARBON_XINPUT=0` : événements X classiques plutôt que XInput 2.
- Restent : Fcitx, menu de la fenêtre au clic droit sur la barre, transferts `INCR`.

#### Boucle Wayland (`linux/wayland/`)

- Protocoles : les interfaces de base viennent de libwayland-client ; celles des
  extensions (`xdg-shell`, `viewporter`, `fractional-scale-v1`, `xdg-decoration`,
  `text-input-v3`) sont
  transcrites des XML de wayland-protocols dans `wayland/protocol/` (rôle de
  `wayland-scanner`, sans outil à la compilation ; les tables sont complètes, les numéros
  des messages en dépendent). Requêtes par `wl_proxy_marshal_flags` (macros `request!`,
  `create!`, `destroy!`), événements par un répartiteur unique
  (`wl_proxy_add_dispatcher`) qui décode les arguments d’après la signature du message
  et les met en file ; la boucle les traite hors de tout appel à libwayland.
- Attente : `poll` sur la connexion et un `eventfd` écrit par les tâches de fond, jusqu’au
  prochain réveil (runtime, répétition d’une touche).
- Fenêtres : `xdg_toplevel` ; taille et états (agrandie, collée, redimensionnement en
  cours → `UiRuntime::set_live_resize`) appliqués au `configure` qui les confirme. Image
  rendue par Vulkan dans la chaîne d’échange de la surface (chemin par défaut, voir le
  renderer Vulkan), ou dessinée par Cairo sur une surface du renderer et copiée dans un
  tampon `wl_shm` libre (trois au plus) ; envoyée avec un rappel de trame : l’image
  suivante attend ce rappel, sauf après une configuration ; une fenêtre cachée n’en
  reçoit pas et ne peint pas.
  Régions opaque (fond du thème) et d’entrée (vide pour `click_through`).
- Échelle : `wp_fractional_scale_v1` (tampon à la taille physique, `wp_viewport` à la
  taille logique), sinon échelle entière (`wl_surface.preferred_buffer_scale`, ou la plus
  grande des sorties de la surface).
- Décorations : avec `xdg-decoration` (KDE…), barre du compositeur pour `title_bar` ; sans
  (GNOME), barre et boutons du moteur (`ChromeState`, mêmes zones que sous Windows :
  `title_bar::hit_test`), déplacement (`xdg_toplevel.move`, double clic : agrandir), menu
  de la fenêtre au clic droit, redimensionnement par les bords (8 px, curseurs des bords).
  Une zone `data-drag-region` déplace aussi la fenêtre.
- Entrées : position et défilement en unités logiques (cran de molette `axis_value120` =
  48 px comme sous Windows, pavé tactile en pixels, Maj : horizontal) ; clavier par
  xkbcommon (touches du moteur, raccourcis Ctrl+A/C/X/V/Z/Y, Ctrl+←/→, Maj+Inser,
  Ctrl+Inser, Maj+Suppr), touches mortes par la table `Compose` de la locale, répétition
  côté client (`repeat_info`). Curseurs du thème (libwayland-cursor, `XCURSOR_THEME`,
  `XCURSOR_SIZE`, taille × échelle entière).
- Méthode d’entrée (`zwp_text_input_v3`, IBus sous GNOME) : activée tant qu’un champ
  modifiable a le focus dans la fenêtre qui a le clavier, avec le rectangle du caret et
  le texte environnant (4000 octets au plus) ; événements appliqués au `done` dans l’ordre
  du protocole (suppression autour du caret rejouée en Retour arrière et Suppr, texte
  validé, texte en composition) et traduits en `CompositionStart`/`Update`/`Commit`/`End`.
  Un clic ou la perte du focus valide la composition, comme sous macOS. Les touches que
  la méthode d’entrée ne prend pas arrivent par `wl_keyboard` (touches mortes par
  `Compose` sans méthode d’entrée).
- Presse-papiers : source de l’application (texte servi depuis un thread), lecture d’une
  autre application par un tube en 2 s au plus ; le texte de l’application est relu
  directement tant qu’elle est la source.
- Non disponibles sous Wayland : position initiale (`x`, `y`), `topmost`, `activate`.

#### Renderer Vulkan (`linux/vulkan/`)

Même conception que Metal (voir [présentation macOS](#présentation-à-lécran-macos)) : un
quad instancié par élément, couverture par distance signée ou texture ; structures
communes `platform/shared/gpu.rs` (instances, arrêts, maillons de découpe). Étape 1 :

- `libvulkan.so.1` chargée au lancement (`ffi/vulkan/` : types transcrits de
  `vulkan_core.h`, avec assertions de taille et d’alignement tirées du compilateur C ;
  fonctions par `vkGetInstanceProcAddr`/`vkGetDeviceProcAddr`). Aucun paquet de
  développement pour compiler.
- Vulkan 1.3 (rendu dynamique, synchronization2) et `VK_KHR_push_descriptor` exigés ;
  GPU intégré préféré, puis dédié ; llvmpipe seulement sur demande (tests).
  `KARBON_VULKAN_DEVICE=<texte>` impose un GPU, `KARBON_VULKAN_VALIDATION` active la
  couche de validation de Khronos.
- Shaders : `vulkan/shaders/*.vert|frag` (GLSL, port de `macos/metal/shaders.metal`,
  structures dans `common.glsl`), compilés en SPIR-V versionné par `shaders/build.sh`
  (`glslangValidator`, paquet glslang-tools, outil des développeurs seulement).
- Pipeline : bande de deux triangles par instance, descripteurs poussés (instances,
  arrêts, maillons, texture), constantes de push du repère de la cible, mélange
  prémultiplié ; rendu hors écran en BGRA 8 bits non linéaire, relu pour les tests.

Étape 2 (traduction des scènes, copie propre à Linux de `macos/metal/encode/` : choix du
2026-11-05, sans toucher au code macOS), par paliers :

- a (fait) : formes, traits, segments, dégradés linéaires et radiaux, découpes
  rectangulaires et arrondies (chaîne), transformations, opacité d’un élément et des
  groupes (couches intermédiaires dans une réserve par taille, `submit.rs`). Écarts
  Vulkan/Cairo (moyenne par canal sur 255, 1x / 1,5x / 2x) : formes et dégradés
  0,086 / 0,088 / 0,069, découpes, transformations et groupes 0,024 / 0,022 / 0,016 ;
  mêmes valeurs sur RADV et llvmpipe. Les commandes pas encore traduites rendent
  `Error::Unsupported` (l’image passera par Cairo) ;
- b (fait) : texte. Mise en page Pango reprise du renderer Cairo (`graphics/gpu.rs`,
  `text_glyphs` : mêmes runs, positions et couleurs par portion que le dessin) ; glyphes
  rastérisés seuls par Cairo et rangés dans un atlas (`vulkan/atlas.rs` : pages
  1024 × 1024 par étagères, R8 teinté ou BGRA pour les émojis, copiées en tête d’image
  par un tampon intermédiaire, page neuve effacée). Cairo 1.18 pose un glyphe au quart de
  pixel sur les deux axes (pixel `floor(v + 1/8)`, phase rastérisée) et un émoji au pixel
  le plus proche ; la position suit son trajet (départ du run arrondi au 1/256 de pixel
  du point fixe, ramené au repère logique). Le texte droit sort donc identique au
  pixel près, au mélange près : écarts 0,024 / 0,017 / 0,014, émojis 0, groupe
  translucide 0,014 / 0,009 / 0,007 ; sous rotation, le glyphe droit est tourné par le
  shader (1,35 / 0,97 / 0,68, bords seulement). Texte rempli (`background-clip: text`)
  avec le palier d ;
- c (fait) : images et rastérisations. Une image droite reprend la décision du dessin
  Cairo (`graphics/gpu.rs`, `display_image` : même rectangle arrondi au pixel, mêmes
  pixels réduits à la taille d’affichage, même repérage des tailles animées) et sort
  identique au mélange près (0,024 / 0,011 / 0,027) ; étirée (taille animée, très grande)
  ou tournée, l’image d’origine est échantillonnée avec ses mipmaps (réductions par
  `vkCmdBlitImage`), proche du filtre `GOOD` de Cairo (0 à 1,06 ; tournée 0,64 / 0,56 /
  0,52). Icônes, SVG et polygones sont rastérisés seuls par Cairo à leur taille
  d’affichage (`rasterize`) et posés pixel pour pixel : identiques (0 / 0 / 0,001) ;
  sous rotation, rastérisés dans le repère local et échantillonnés (0,77 / 0,44 / 0,36).
  Textures gardées d’une image à l’autre (`vulkan/textures.rs`, bornes de Metal :
  60 images d’inactivité, budgets de 64 Mo pour les images et 32 Mo pour les
  rastérisations) ; envois de l’atlas et des textures par un tampon intermédiaire commun
  (`vulkan/staging.rs`), rendu au-delà de 4 Mo ;
- d (fait) : effets. Filtres CSS en passages (`vulkan/filter/`, `shaders/filter.frag`,
  cibles RGBA 16 bits flottants) qui suivent exactement les filtres CPU du chemin Cairo
  (`platform/shared/filters.rs`) : réduction par blocs au-delà de 6 px d’écart type,
  trois flous en boîte par axe (Metal fait une vraie gaussienne, comme Core Graphics),
  agrandissement bilinéaire, matrices enchaînées, ombre portée ; groupes filtrés gardés
  d’une image à l’autre par signature et place (`resources.rs`). Flou 0,12 / 0,05 /
  0,04, grand flou réduit 0,56 / 0,42 / 0,42, ombre 0,06, matrices ≤ 0,05,
  enchaînements ≤ 0,62. `backdrop-filter` : instances de la cible finale rejouées dans
  une couche de la zone relue, filtrée, recomposée sous la forme arrondie (≤ 0,16).
  `mask-image` : contenu composé dans une couche couvrant la découpe, recomposée par un
  maillon de dégradé (0,14 / 0,12 / 0,12) ; Metal masque chaque élément
  (`CGContextClipToMask`), Cairo et CSS le groupe composé, ce qui diffère sous un
  contenu translucide superposé. Texte rempli : couverture des glyphes masquant le
  remplissage (0,014), émojis par leur alpha comme le groupe d’alpha de Cairo (0,010).

Toute scène est traduite : l’interface RustMusic exportée par Vulkan
(`--renderer=gpu --export`, mode `Gpu` des rendus hors écran, `linux/offscreen.rs`) ne
s’écarte du rendu Cairo que de 0,121 en moyenne (1920 × 1350, 0,006 % des canaux
au-delà de 16), sans erreur de la couche de validation, synchronisation comprise. Les
modes `Auto` et `Software` des rendus hors écran restent sur Cairo (exports de
référence).

Étape 3 (faite) : présentation des fenêtres Wayland, chemin par défaut (`Auto` et
`Gpu` ; `Software` reste sur Cairo), comme Metal sous macOS.

- `vulkan/surface.rs` : surface `VK_KHR_wayland_surface` de la `wl_surface` et chaîne
  d’échange BGRA 8 bits à la taille en pixels (échelle fractionnaire : destination du
  viewport posée par la fenêtre), refaite quand la taille change ou que la chaîne est
  périmée ; mode « mailbox » s’il existe (sinon « fifo ») : le rythme vient du rappel de
  trame de la fenêtre, une fenêtre cachée n’envoie rien et rien ne bloque. Sémaphore
  d’acquisition attendu par la soumission, sémaphore de fin de rendu par image de la
  chaîne attendu par la présentation.
- `vulkan/present.rs` : traduction (renderer Cairo de la fenêtre emprunté pour le texte et
  les images, comme Metal emprunte celui de Core Graphics), image acquise (au plus
  50 ms d’attente, sinon reprise au tour suivant), rendu, présentation, puis la main est
  rendue sans attendre le GPU : une image en vol, dont la fin est attendue au début de
  l’image suivante (`finish`) avant de réutiliser tampons, réserves et ressources.
  RustMusic : traduction 0,5 ms, enregistrement 0,4 ms, soumission et présentation
  0,2 ms.
- `wayland/window/paint.rs` : rappel de trame, échelle et destination du viewport demandés
  juste avant `vkQueuePresentKHR`, dont l’envoi de la surface les applique. Une erreur du
  GPU désactive Vulkan pour la fenêtre (Cairo dessine l’image en entier) ; une image
  présentée par Vulkan fait oublier à Cairo son image précédente. `--profile` : GPU et
  mode de présentation à l’ouverture, puis préparation, traduction, enregistrement, GPU
  et présentation de chaque image. RustMusic 2160 × 1560 sur Radeon 760M : 6,7 ms par
  image entière (traduction 0,5 ms, GPU et présentation 5,5 ms), contre 19,5 ms de dessin
  Cairo ; aucune erreur de la couche de validation, synchronisation comprise.

- Contexte Vulkan (instance, appareil, file) partagé par les fenêtres de la boucle, créé
  à la première fenêtre qui le demande ; chaque fenêtre garde son renderer, ses réserves
  et ses caches (atlas, textures, groupes filtrés), comme sous macOS.
- Allègement (`Renderer::trim`, comme `metal::Trim`) : une seconde après la dernière
  image, réserves des couches et des cibles de filtre, tampon intermédiaire et textures
  que la dernière image n’a pas employées ; fenêtre réduite ou masquée (état
  `suspended` de xdg-shell 6), tout, surfaces et tampons Cairo compris.

Sous Wayland, le client ne choisit pas la position de ses fenêtres (`x`, `y` ignorés) :
GNOME peut poser deux fenêtres de même taille l’une sur l’autre.

#### Accessibilité AT-SPI (`linux/accessibility/`)

- libdbus-1 en liaison directe (`ffi/dbus.rs`) : connexions privées sans boucle propre ;
  les boucles Wayland et X11 surveillent le descripteur et lisent les messages sans
  attendre. Seul le thread de la boucle touche libdbus (pas de
  `dbus_threads_init_default`).
- Surface exposée : comme pour UIA sous Windows et GTK/Qt, tout processus de la session
  connecté au bus d’accessibilité (socket limité à l’utilisateur) peut lire l’arbre,
  appuyer sur les boutons, régler les curseurs et modifier le texte des champs
  (`EditableText`) ; le moteur applique ses propres bornes (lecture seule, désactivé,
  `max_chars`).
- Enregistrement au démarrage (comme GTK, sans attendre qu’un lecteur d’écran soit
  lancé) : adresse du bus d’accessibilité (`org.a11y.Bus.GetAddress` sur le bus de
  session), puis `org.a11y.atspi.Socket.Embed` auprès du registre. Sans session D-Bus
  ou sans at-spi2, pas de pont.
- Objets : application (`/org/a11y/atspi/accessible/root`), une fenêtre `frame` par
  fenêtre du moteur (`…/w<id>`), les éléments de l’instantané accessible
  (`…/w<id>/e<élément>`), renouvelé quand `UiRuntime::accessibility_version` change.
  Interfaces : Accessible (rôles, états, attributs `xml-roles` des repères, enfants,
  parent), Application, Component (repère logique de la fenêtre pour tous les types de
  coordonnées : Wayland ne donne pas la position des fenêtres), Action (appui, bascule,
  focus), Value (curseurs, `CurrentValue` modifiable), Text et EditableText (offsets en
  caractères, frontières de caractère, de mot et de ligne visuelle, sélection, étendues
  par clusters), propriétés D-Bus, introspection minimale, cache vide (`GetItems`).
  Interrupteur annoncé comme bouton bascule (pas de rôle `switch` dans at-spi2-core 2.52).
- Événements seulement si un client écoute (`Registry.GetRegisteredEvents` et signaux
  `EventListenerRegistered`/`Deregistered`) : fenêtre activée ou désactivée, focus (la
  perte du focus d’un élément sorti de l’arbre n’est pas annoncée), texte inséré ou
  supprimé, caret, valeur d’un curseur focalisé.

### Ajouter un backend

1. Créer `src/platform/<os>/`,
   compilé sous `#[cfg(target_os = …)]`, avec ses dépendances dans une section
   `[target.'cfg(…)'.dependencies]` de `Cargo.toml`. Reprendre le découpage de
   `platform/windows/` : `window/`, `events/`, `present/`, `graphics/`,
   `accessibility/`, `offscreen.rs`, `tests/`.
2. Implémenter `Offscreen` (texte, images, rastérisation) puis `Backend`
   (`run_windows` en suivant les obligations ci-dessus, compteurs, PNG). Le module
   `windows/offscreen.rs` montre l’adaptateur hors écran complet.
3. Ajouter la ligne `type Native = …` correspondante dans `platform/mod.rs`
   et restreindre la ligne headless aux cibles restantes. Rien d’autre ne change :
   aperçus, mesures et tests headless fonctionnent déjà.
4. Faire passer les tests de contrat (`src/ui/tests/`, `platform/headless/tests/`,
   puis les crates de `tests/`), écrire la suite minimale du backend sur le modèle
   de `windows/tests/` et comparer les exports de référence (voir
   [le contrat d’un nouveau backend](tests.md#contrat-dun-nouveau-backend)).

## Présentation à l’écran (macOS)

Une fenêtre a deux chemins de dessin, choisis à chaque image par
`window/session/paint.rs` :

- **Metal** (`metal/`), chemin par défaut : en `RenderMode::Auto` (valeur par défaut des
  options) et `RenderMode::Gpu` dès qu’un périphérique Metal existe. Toutes les commandes
  de la scène, filtres, `backdrop-filter`, masques et texte rempli compris (un émoji sous
  `background-clip: text` sert de masque par sa luminance, comme sous Core Graphics).
  Image entière à chaque fois, sans suivi des zones. Un `CAMetalLayer` (BGRA8 non
  linéaire, espace sRGB comme la fenêtre, `framebufferOnly`, `contentsScale` et taille du
  drawable suivant la vue, trois drawables) est un sous-calque du calque de la vue ; il
  présente dans la transaction Core Animation de la session (`presentsWithTransaction`).
- **Core Graphics** (`graphics/`), repli : en `RenderMode::Software` (`--renderer=software`)
  pour toutes les images ; sans périphérique Metal (machine virtuelle) ou si les shaders ne
  compilent pas (message écrit en `Gpu` ou sous `--profile`) ; pour le reste de la vie de
  la fenêtre après une erreur du GPU (création d’un tampon de commandes, image précédente
  terminée en erreur : GPU retiré ou réinitialisé) ; pour une image sans drawable, ou une
  scène refusée par l’encodage (`Error::Unsupported`, plus aucun cas aujourd’hui ;
  `metal::Presented::Fallback`, raison écrite par `--profile`). Redessin partiel,
  demi-résolution pendant les mouvements ; l’image devient le contenu du calque de la vue.
  `KARBON_METAL_FALLBACK` (diagnostic, lu à chaque image) confie les images à Core
  Graphics tant que la variable existe : la sonde native vérifie ainsi le passage.

Le passage d’un chemin à l’autre se fait dans une seule transaction : le calque Metal est
montré et le contenu de la vue vidé, ou l’inverse. Une image Metal libère ce qui ne sert
qu’à Core Graphics (surface de la fenêtre, surface de mouvement, images converties et
redimensionnées, caches des groupes filtrés et des `backdrop-filter`, inventaires des
zones : `graphics::Renderer::release`) ; la suivante par Core Graphics est dessinée en
entier. Les mises en page Core Text, partagées par les deux chemins, restent.
`ViewContext::backend` vaut le chemin de la dernière image (`GPU · Metal` ou
`CPU · Core Graphics`). Le rendu hors écran (aperçus, `--export`, mesures) reste Core
Graphics : il est la référence.

Renderer Metal :

| Fichier | Rôle |
| --- | --- |
| `device.rs` | GPU, file, bibliothèque compilée au lancement (`newLibraryWithSource`, sources `shaders.metal` et `filter/filters.metal` embarquées), pipeline (mélange prémultiplié « source-over ») et modes de stockage |
| `encode/` | scène → couches d’instances et étapes : pile des portées et groupes (`scopes.rs`), formes (`shapes.rs`), texte et texte rempli (`text.rs`), images et rastérisations (`raster.rs`), groupes filtrés, `backdrop-filter` et masques (`effects.rs`) |
| `filter/` | filtres CSS en passages GPU (`filters.metal` : réduction, flou gaussien séparable, agrandissement, matrices de couleur, ombre portée) |
| `compare.rs` | diagnostic `KARBON_METAL_COMPARE` (Metal relu contre Core Graphics sur une vraie fenêtre) |
| `instance.rs` | structures partagées avec les shaders (instance de 208 octets, arrêts, maillons de découpe) |
| `atlas.rs` | atlas de glyphes en pages de 1024² : masques R8 teintés, BGRA pour les émojis |
| `resources.rs` | textures gardées et leurs bornes : images (par `Arc`, mipmaps), rastérisations, réserve des cibles par taille et format, résultats des groupes filtrés (par signature) ; allègement au repos et fenêtre masquée (`Trim`) |
| `submit.rs` | étapes de l’image dans l’ordre (passages de rendu des couches, passages de filtre), anneau de trois jeux de tampons, relecture |
| `surface.rs` | `CAMetalLayer` de la fenêtre |

Un seul pipeline dessine un quad instancié par élément ; le fragment calcule la couverture
(aire exacte le long des côtés, distance à l’arc dans les coins ; anneau entre deux boîtes
pour un trait, jonctions en onglet ; segment orienté), la multiplie par la découpe
rectangulaire (scissor du lot + couverture anticrénelée des bords fractionnaires) et par
la chaîne des découpes arrondies ou tournées et des masques en dégradé (maillons évalués en distance signée dans le
repère de chaque découpe), puis peint : couleur, dégradé linéaire ou radial (arrêts
interpolés dans le shader, couleurs extrêmes prolongées comme Core Graphics), texture
(images, rastérisations) ou masque teinté (glyphes). L’opacité d’un groupe de plusieurs
éléments passe par une texture intermédiaire bornée au contenu et à la découpe,
recomposée sous la découpe du parent ; un élément seul reçoit l’opacité directement ; un
contenu d’opacité nulle est sauté (même logique que `graphics/canvas.rs`).

Effets (`encode/effects.rs`, `filter/`), calqués sur `graphics/layers.rs` et
`graphics/filters.rs` :

- **Groupe filtré** (`PushFilter`) : zone des filtres (bornes élargies du flou et de
  l’ombre, entière tant qu’elle reste sous 4 Mpx, sinon bornée à la découpe élargie),
  contenu rendu dans une couche réduite d’un facteur k pour un grand flou
  (`graphics::reduction`, comme Core Graphics), puis passages de filtre dans des textures
  RGBA 16 bits flottants (pas d’arrondi entre deux filtres) : flou gaussien séparable en
  deux passages, calculé sur une image réduite d’un facteur ⌊σ/3⌋ ≤ 8 (moyenne des blocs,
  agrandissement bilinéaire, même échantillonnage que le CPU) ; ombre portée (alpha
  décalé d’un nombre entier de pixels, teinté, flouté, contenu posé dessus) ; matrices de
  couleur consécutives en un passage (couleurs non prémultipliées, bornées après
  chacune). La sortie est recomposée étirée sur la zone, sous la découpe du parent. Elle
  est gardée d’une image à l’autre par élément tant que la `Signature` du contenu et la
  place de la couche (translation au quart de pixel près) ne changent pas : le contenu
  n’est alors ni encodé ni filtré.
- **`backdrop-filter`** (`Backdrop`) : ce que la cible finale a reçu jusque-là dans la
  zone relue (forme élargie de trois écarts types) est redessiné dans une couche de cette
  taille (mêmes instances, même ordre, mêmes textures), filtré, puis recomposé sous une
  découpe à la forme arrondie. La cible finale reste dessinée en un seul passage : couper
  son passage pour la relire coûterait une écriture et une relecture de toute la fenêtre
  par carte. Comme dans Core Graphics, un `backdrop-filter` dans un groupe à opacité relit
  la cible finale (le contenu du groupe n’en fait pas partie) et il est ignoré dans un
  groupe filtré. Les zones relues ne sont pas gardées d’une image à l’autre : leur
  redessin et leur flou (réduit) coûtent peu sur le GPU.
- **`mask-image`** (`PushMask`) : maillon de la chaîne des découpes évalué par le shader
  (alpha du dégradé, couleurs extrêmes prolongées), multiplié à chaque élément de la
  portée, comme `clip_to_mask` ; les masques s’imbriquent.
- **`background-clip: text`** (`PushTextFill`) : chaque glyphe sert de masque au
  remplissage (couleur, dégradé linéaire ou radial posé sur la zone de la portée, hors de
  laquelle rien n’est peint) ; les décorations sont remplies de même.

Ordre des étapes (`encode::Step`) : une couche (groupe à opacité, contenu d’un groupe
filtré, zone relue par un `backdrop-filter`) est dessinée quand elle est complète, puis
viennent ses passages de filtre ; la cible finale est dessinée en dernier.

Texte : les mises en page Core Text du chemin Core Graphics sont réutilisées (lignes,
runs, positions, couleurs par portion, décorations en rectangles). Chaque glyphe est
rastérisé une fois par `CTFontDrawGlyphs` à l’échelle d’affichage ; sa position est
transmise au douzième de pixel (au sixième au-delà de 16 px) et Core Graphics y applique
sa propre quantification (tiers, demi ou pixel entier selon la taille), ligne de base sur
un pixel entier : sous un repère droit, le texte est identique à celui du chemin Core
Graphics. Un glyphe en couleur (émoji) est rangé en BGRA ; sous `background-clip: text`,
il est dessiné dans un masque en niveaux de gris (`atlas::Kind::Luminance`), comme le
masque de texte de Core Graphics. Écart connu : Core Graphics pose l’image d’un émoji à
sa position verticale fractionnaire, Metal sur le pixel entier de la ligne de base
(jusqu’à un pixel d’écart vertical sur les émojis).

Icônes, SVG et polygones (`Path`) sont rastérisés par Core Graphics à leur
taille d’affichage et posés pixel pour pixel (sous une rotation : dans le repère local,
échantillonnés), en cache par contenu et position sous-pixel.

GPU et mémoire : sur un Mac à deux GPU (Mac Intel à graphiques commutables), le GPU
économe (`isLowPower`) est retenu et `MTLCreateSystemDefaultDevice`, qui forcerait le
GPU dédié, n’est pas appelé. Une application empaquetée déclare
`NSSupportsAutomaticGraphicsSwitching` à `true` dans son `Info.plist` pour que le système
garde la main sur la commutation. Stockage (`device::Storage`) : tampons d’instances en
`Shared` en mémoire unifiée (Apple Silicon, GPU Intel intégrés), en `Managed` sur un GPU
dédié (copie signalée par `didModifyRange`) ; textures remplies par le CPU en `Shared`
seulement sur un GPU Apple (macOS refuse les textures `Shared` sur les GPU Intel et AMD,
même intégrés), en `Managed` sinon ; cibles intermédiaires en `Private` ; relecture
(tests, diagnostic) par un tampon `Shared`. `KARBON_METAL_STORAGE=managed` force
`Managed` partout, relecture comprise (synchronisée par un blit `synchronizeResource`) :
les tests y passent le chemin d’un GPU dédié.

Perte ou changement de GPU : un tampon de commandes impossible à créer, ou l’image
précédente terminée en erreur (`MTLCommandBufferStatus::Error`, GPU retiré ou
réinitialisé), rend une erreur ; la session retire le calque Metal et la fenêtre reste au
chemin Core Graphics. Une fenêtre déplacée vers un écran d’un autre GPU continue d’être
rendue par le GPU retenu (le système recopie l’image) ; elle ne change pas de
périphérique.

Bornes des ressources (`resources.rs`) : une texture d’image ou une rastérisation
inutilisée depuis 60 images est libérée, et toutes celles que l’image courante n’emploie
pas au-delà de 64 Mo (images) ou 32 Mo (rastérisations) ; une cible en réserve
inutilisée depuis 30 images est libérée, la réserve gardant au plus huit textures et
32 Mo ; un groupe filtré absent de l’image est oublié ; l’atlas est vidé au-delà de huit
pages. Une seconde après la dernière image (réveil `trim_at` de la session), `Trim::Idle`
vide la réserve, les tampons de l’anneau et les images et rastérisations que la dernière
image n’a pas employées. Fenêtre réduite dans le Dock ou entièrement cachée
(`windowDidMiniaturize:`, `windowDidChangeOcclusionState:`) : `Trim::Hidden` libère toutes
les ressources GPU (atlas compris) et les surfaces Core Graphics ; aucune image n’est
dessinée dans le Dock, la suivante est refaite à la sortie (`windowDidDeminiaturize:`).
Les vecteurs d’instances d’une image sont réutilisés par la suivante (`Built::recycle`).
Les trois drawables du `CAMetalLayer` (~17 Mo chacun en Retina 1440 × 791) restent
alloués : avec deux, l’attente du drawable double le temps d’image d’un
redimensionnement.

Au repos, aucune image n’est rendue ni présentée : la boucle attend le prochain
événement ou le prochain réveil du runtime (pas de `CADisplayLink`) ; seul le réveil
d’allègement suit, une fois, la dernière image.

`--profile` écrit pour chaque image Metal la traduction de la scène en instances
(`traduction`), l’encodage des passages, l’attente du drawable et de la planification, le
temps GPU de l’image précédente et, quand elle change, la mémoire GPU tenue par le
renderer (atlas, images, rastérisations, cibles, groupes filtrés, tampons).

## Présentation à l’écran (Windows)

Par défaut (`PresentMode::Auto`), une fenêtre est présentée par
DirectComposition, sans relecture GPU → CPU :

1. fenêtre créée avec `WS_EX_NOREDIRECTIONBITMAP` (aucune surface GDI) ;
2. périphérique D3D11 du renderer (matériel ou WARP, avec `BGRA_SUPPORT`) →
   `IDXGIFactory2::CreateSwapChainForComposition` : modèle flip, 2 tampons,
   B8G8R8A8, alpha `PREMULTIPLIED` ou `IGNORE`, latence d’une image (objet
   attendable, `FrameSlot`) ;
3. Direct2D dessine dans un bitmap cible (`TARGET | CANNOT_DRAW`, DPI de la
   fenêtre) créé sur le back buffer ;
4. `DCompositionCreateDevice` → `CreateTargetForHwnd` → visuel racine dont la
   chaîne d’échange est le contenu → `Commit` ;
5. chaque WM_PAINT attend sa place dans la file de présentation (au plus 50 ms),
   dessine puis appelle `Present(1, 0)` (vsync), sans rectangles sales. Une image
   d’animation (`FRAME_WAKE`) prend cette place **avant** de lire l’horloge : elle
   est échantillonnée juste après le balayage qui a pris la précédente et
   s’affiche au suivant, à intervalle constant (le `BeginFrame` de Chromium).

Fichiers : `platform/windows/present/composition.rs` (choix du chemin, visuel
DComp), `graphics/swapchain.rs` et `graphics/target.rs` (chaîne d’échange),
`present/paint.rs` (peinture), `present/runtime.rs` et
`ui/rendering/scene/coverage.rs` (test de contenu pour WM_NCHITTEST).

L’alpha est `PREMULTIPLIED` dès que des pixels peuvent être transparents :
fenêtre transparente, ou fenêtre opaque non agrandie, car son fond a des coins
arrondis de 12 px dessinés par le moteur (avec `IGNORE`, ces coins seraient
noirs). Une fenêtre opaque agrandie, entièrement couverte, utilise `IGNORE`.
Le mode alpha d’une chaîne étant fixé à sa création, agrandir/restaurer ou F3
recrée la chaîne et la rattache au visuel. Un simple redimensionnement libère
la cible Direct2D, appelle `ResizeBuffers`, puis recrée la cible.

La chaîne est créée dès l’ouverture, avant de confier l’application à la
fenêtre : toute la chaîne de composition est ainsi validée. Une erreur
`DXGI_ERROR_DEVICE_REMOVED/RESET/HUNG` ou `D2DERR_RECREATE_TARGET` (au rendu,
au redimensionnement ou à `Present`) devient `Error::DeviceLost` : le renderer est
recréé dans le même mode (`Auto` retente le GPU), puis sa nouvelle chaîne est
rattachée au visuel, qui ne dépend d’aucun périphérique DXGI. Les autres échecs
de rendu conservent le repli WARP historique.

### Chemin layered (repli)

L’ancien chemin reste disponible : rendu dans une texture, copie vers une
texture de relecture, `Map`, pixels écrits dans un DIB persistant, puis
`UpdateLayeredWindow` sur une fenêtre `WS_EX_LAYERED`. Il est utilisé :

| Cas | Chemin |
| --- | --- |
| `PresentMode::Auto` (défaut, `--present=auto`) | composition |
| `Auto` et échec de la composition à l’ouverture | layered, message sur stderr |
| `click_through` (quel que soit le mode `Auto`/`Layered`) | layered : `WS_EX_TRANSPARENT` n’est traversant qu’avec `WS_EX_LAYERED` |
| `PresentMode::Layered` (`--present=layered`) | layered, pour comparaison |
| `PresentMode::Composition` (`--present=composition`) | composition ; erreur si indisponible ; refusé avec `click_through` |

L’ombre (`window/shadow.rs`) reste une fenêtre layered séparée. Le rendu hors écran
(`render_benchmark`, `render_preview`, export PNG) garde la relecture CPU et
n’est pas concerné par `PresentMode`.

### Différences restantes

- **Test de contenu.** Windows ignore les clics sur les pixels d’alpha nul d’une
  fenêtre layered. Une fenêtre composée reçoit tous les clics : WM_NCHITTEST
  renvoie donc `HTTRANSPARENT` hors du contenu peint (coins arrondis, fond d’une
  fenêtre transparente). La couverture est estimée sur la scène (rectangles
  arrondis, traits, boîtes du texte et des images, transformations, clips,
  opacité nulle) : elle est plus généreuse que l’alpha réel entre les glyphes ou
  dans une image partiellement transparente. Surtout, `HTTRANSPARENT` ne transmet
  le clic qu’aux fenêtres **du même thread** : sur une zone vide d’une fenêtre
  transparente, le clic n’atteint plus les autres applications. Utiliser
  `click_through` ou `--present=layered` si ce passage est nécessaire.
- **Fenêtre agrandie.** DWM arrondissait aussi les coins d’une fenêtre layered
  agrandie (8 px) ; la fenêtre composée agrandie a des coins droits, comme une
  fenêtre Windows standard. Fenêtres opaques et transparentes non agrandies :
  captures identiques au pixel près sur la machine de développement.
- **Redimensionnement interactif.** La chaîne est redimensionnée au WM_PAINT
  suivant ; pendant un agrandissement à la souris, une bande transparente peut
  apparaître brièvement au bord, alors qu’UpdateLayeredWindow changeait taille
  et contenu en une fois (comportement attendu, pas encore vérifié en recette
  visuelle).
- **Mesure.** `Present` rend la main dès que l’image est mise en file : le temps
  GPU n’est plus inclus dans la peinture, alors que la relecture l’attendait.

Le [guide de syntaxe](syntax.md) présente l’API. Les anciennes démonstrations
sont documentées uniquement dans les [archives](archive/README.md).
