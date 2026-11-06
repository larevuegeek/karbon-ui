# Point de reprise

Le projet s’appelle `karbon-ui` (crate Cargo du même nom) ; sa publication en dépôt
public séparé est décidée (voir « Publication » ci-dessous). Dépendances : le
compilateur proc-macro local `crates/ui-macros` ; sous Windows, les liaisons
officielles Microsoft `windows`, `windows-sys`, `windows-core`, `windows-numerics` ;
sous macOS, la famille `objc2` et `libc` ; sous Linux, `libc` seul (bibliothèques du
bureau par liaisons écrites à la main). Backends : Windows complet, macOS (boucle AppKit,
Metal), Linux (hors écran et boucle Wayland, voir plus bas). Sous Windows, la présentation passe par une chaîne
d’échange DirectComposition (`WS_EX_NOREDIRECTIONBITMAP`, Present vsync, sans
relecture) ; le chemin layered historique reste le repli, le chemin du clic
traversant et l’option `--present=layered` (voir
[architecture](architecture.md#présentation-à-lécran-windows)). Le rendu hors
écran et l’export PNG gardent la relecture. Aucun gain face à Tauri n’est établi.

Points ouverts de la présentation : rectangles sales (`Present1`), latence
maximale d’images (`SetMaximumFrameLatency`/objet attendable), recette visuelle
du redimensionnement interactif, partage d’un périphérique entre fenêtres, et
clic traversant sur les zones vides d’une fenêtre transparente composée
(`HTTRANSPARENT` ne sert que les fenêtres du même thread).

## Publication (décidée le 2026-11-06)

karbon-ui devient un **dépôt public séparé**, `github.com/larevuegeek/karbon-ui`,
frère de `karbon` (framework web, AGPL, CI Linux, releases sur `main`) et de
`karbonjs` — ni branche ni membre du workspace de Karbon : politique de dépendances
opposée (zéro crate tiers contre 506 paquets), CI à trois OS contre un, rythmes et
maturités différents, licence différente. Le lien futur avec Karbon, s’il doit exister,
passera par une dépendance crates.io versionnée. Préparé dans le dépôt :
`LICENSE-MIT` et `LICENSE-APACHE` (double licence, standard des bibliothèques Rust),
métadonnées des deux `Cargo.toml` (publication crates.io prévue : `karbon-ui-macros`
d’abord, puis `karbon-ui`, même version ; `exclude` écarte docs, outils et CI du paquet,
5,6 Mo), `.github/workflows/ci.yml` (lint, tests sur Windows, macOS et Linux,
exports de référence Windows à télécharger), README en anglais comme les dépôts
frères, `artifacts/` et `accueil.png` sortis de git (captures de RustMusic avec de
vraies pochettes ; le dossier reste utile localement), et les 58 pochettes de
`examples/rustmusic_html/assets/covers/` remplacées par des images synthétiques de
mêmes noms et dimensions (exports de référence rebasés à cette occasion : mise en page
inchangée, pochettes différentes).

Dépôt public créé le 2026-11-06 avec **un historique neuf** : un commit initial depuis
l’arbre courant (l’ancien historique pèse 78 Mo de captures, porte les vraies pochettes
et les adresses des commits privés ; il reste dans le dépôt privé, à part). Avant ce
commit, balayage des contenus privés : `.vscode/` retiré et ignoré, chemins personnels
remplacés par des chemins neutres (dossier affiché dans la barre d’état de la démo
`D:\Music\…`, fixtures des tests de texte), nom de bibliothèque de test anonymisé.
Exports de référence rebasés une dernière fois à cette occasion (seule la barre d’état
change ; SHA-256 sur le poste de développement : 1280 × 900 `A69EA80E3D31DB2D…`,
960 × 700 `952A635750B67377…`, inchangé).
`.github/workflows/ci.yml` est conservé : c’est la CI elle-même, sans rien de privé.

Reste à faire à la main :

1. pousser `main` (`git push -u origin main`), puis publier sur crates.io
   `karbon-ui-macros` d’abord, `karbon-ui` ensuite, et poser l’étiquette `v0.1.0` ;
2. première exécution de la CI : les tests Linux de la boucle de fenêtres se sautent
   sans compositeur, ceux de Vulkan sans GPU, mais aucun n’a encore tourné sur un
   exécuteur GitHub — ajuster au besoin.

## Backend Linux (en cours)

Développement passé sous Linux le 2026-11-05 (Zorin OS 18.1, GNOME 46 sous Wayland,
XWayland, Mesa 25.2, Radeon 760M). Pile choisie par le propriétaire : Wayland natif puis
X11, Pango + HarfBuzz + fontconfig pour le texte, Cairo puis Vulkan pour le rendu
(détail et bibliothèques liées : [architecture](architecture.md#backend-linux)).

Revue complète du 2026-11-06, menée depuis Windows : [linux-review.md](linux-review.md)
suit chaque constat (bugs, sécurité, lisibilité) avec son état. Les corrections
compilent pour la cible Linux (`cargo clippy --target x86_64-unknown-linux-gnu`) mais
n'ont pas tourné sous Linux : **commencer par la liste « À valider sous Linux avant
tout »** de ce fichier. Le socle `shell/` a gagné `scroll.rs` (cran de molette, Maj) et
`composition.rs` (séquence de composition des méthodes d'entrée), communs aux deux
boucles ; `ffi/xcb.rs` et `ffi/pango.rs` sont devenus des dossiers.

Étape 1 faite : services hors écran (`src/platform/linux/`, `linux::Linux` est
`platform::Native` sous Linux). Texte Pango, polices embarquées (fontconfig privé, `memfd`,
WOFF2 décodé par FreeType, famille privée prioritaire sur une police installée du même
nom), rendu Cairo (formes, dégradés, images, SVG, clips, effets, groupes filtrés,
`backdrop-filter`, masques, texte rempli), images gdk-pixbuf (choix du backend, faute
de décodeur dans Cairo : PNG, JPEG, GIF, WebP selon les modules), PNG par Cairo,
compteurs `/proc/self/status` et `getrusage`. `run_windows` valide puis renvoie
`Error::Unsupported`.

Remaniement commun (rendus macOS inchangés : code déplacé tel quel) : filtres CPU,
portées de la scène, géométrie, empreintes et cache des mises en page de texte de macOS
sont dans `src/platform/shared/`, partagés avec Linux ; le cache de texte est générique
sur la pile du backend (`Typesetter`).

Exports sous Linux (Cairo, `--renderer=software`) : RustMusic 1280×900 et 960×700 et la
démo Bootstrap ont la mise en page des images de référence, polices et anticrénelage
différents ; ils ne sont pas identiques octet pour octet aux images Windows ou macOS.
Export RustMusic 1280×900 à 1,5 : 0,5 s par le binaire de l’exemple (construction et
chargement compris). Les exports headless de Linux d’avant cette étape ne sont plus
produits (le rastériseur headless reste celui de `HeadlessApp`).

Étape 2 faite : boucle Wayland (`src/platform/linux/wayland/`, détail :
[architecture](architecture.md#boucle-wayland-linuxwayland)). Fenêtres `xdg_toplevel`
peintes par Cairo dans des tampons `wl_shm` au rythme des rappels de trame, barre de
titre et bords du moteur sous GNOME (pas de décorations serveur), échelle fractionnaire,
pointeur, molette et pavé tactile, clavier xkbcommon avec touches mortes et répétition,
raccourcis d’édition, curseurs du thème, presse-papiers, minuteries, tâches de fond,
commandes de fenêtre. Ce compositeur (GNOME 46) annonce `wp_fractional_scale` (1,5 ici),
`zwp_text_input_v3`, ni `xdg-decoration` ni `wp_cursor_shape`. Sonde de la boucle sur le
vrai compositeur ; entrées vérifiées à la main.

Mesures (`--profile`, Radeon 760M, échelle 1,5, image entière à chaque fois) : RustMusic
2160×1560, dessin Cairo ~19,5 ms, copie et envoi ~1 ms (première image 271 ms, chargement
et première mise en page compris) ; démo Bootstrap 1920×1350, dessin 2,3 à 3,1 ms.

Reste, dans l’ordre :

1. Recette à la main des entrées sur RustMusic et les autres exemples.
2. Fait : redessin partiel sous Cairo (`platform/shared/damage.rs`, partagé avec macOS ;
   zones recopiées par tampon et déclarées par `wl_surface.damage_buffer`) et méthode
   d’entrée (`zwp_text_input_v3`). À vérifier à la main : composition IBus (touches
   mortes, méthode japonaise ou chinoise si installée).
3. Fait : accessibilité AT-SPI (`linux/accessibility/`, libdbus-1 ; voir
   [architecture](architecture.md#accessibilité-at-spi-linuxaccessibility)), vérifiée par
   le client Python d’AT-SPI. À faire : recette avec Orca (voix), événements
   `ChildrenChanged` et changements de nom, cache AT-SPI rempli (moins d’appels pour les
   grands arbres), clignotement du caret (absent aussi sous macOS).
4. Renderer Vulkan des fenêtres (Cairo en repli), comme Metal sous macOS (choix du
   2026-11-05 : shaders GLSL compilés par glslang-tools, `.spv` versionnés). Étape 1
   faite (infrastructure, voir [architecture](architecture.md#renderer-vulkan-linuxvulkan)).
   Étape 2 : traduction des scènes, copie propre à Linux de `macos/metal/encode/`
   (choix du propriétaire, code macOS intact) : faite. Paliers a (formes, dégradés,
   découpes, transformations, groupes), b (texte : atlas, glyphes placés comme Cairo au
   quart de pixel), c (images reprises des caches Cairo, mipmaps ; icônes, SVG et
   polygones rastérisés par Cairo) et d (filtres en trois boîtes comme le CPU, groupes
   filtrés gardés, `backdrop-filter`, masques composés en groupe, texte rempli). Toute
   scène est traduite : `--renderer=gpu --export` passe par Vulkan (RustMusic : écart
   moyen 0,121 avec Cairo) ; voir [architecture](architecture.md#renderer-vulkan-linuxvulkan).
   Images comparées : `KARBON_VULKAN_DUMP=<dossier>` avec les tests Vulkan.
   Étape 3 faite : les fenêtres Wayland présentent par Vulkan par défaut (chaîne
   d’échange « mailbox », Cairo en repli et en mode `--renderer=software`, `--profile`).
   À vérifier à la main : redimensionnement, échelles 1x/1,25x/1,5x/2x, fenêtre réduite
   puis rétablie, plusieurs fenêtres, animations (pochettes, survols) : vérifiés par le
   propriétaire le 2026-11-06 (l’interface fonctionne). Faits ensuite : contexte Vulkan
   partagé par les fenêtres, ressources GPU allégées au repos et fenêtre suspendue.
   Puis une image en vol (le processeur n’attend plus le GPU après chaque image).
5. X11 (libxcb), en cours : socle commun des boucles (`linux/shell/`), XCB chargé au
   lancement, fenêtres, image Vulkan ou Cairo, choix du backend (`KARBON_BACKEND=x11`
   pour l’essayer sous XWayland), pointeur, clavier, curseurs, déplacement et
   redimensionnement par le gestionnaire, presse-papiers, défilement fin (XInput 2.1).
   Méthode d’entrée par IBus (D-Bus) faite. Restent : Fcitx (D-Bus), recette à la main
   sur une vraie session X11 (pointeur XI2 vérifié par le propriétaire sous XWayland).
6. Fait : police d’interface et à chasse fixe du bureau dans les fenêtres (GSettings,
   `linux/desktop.rs` ; les exports restent sur fontconfig) et animations réduites.
   Ordre convenu ensuite (2026-11-06) : défilement fin X11 (XInput 2), une image d’avance
   pour le GPU, puis méthode d’entrée X11 par IBus (D-Bus) et Fcitx, pas XIM.
   Observation ouverte : sous très forte charge (plusieurs processus de tests Vulkan à la
   fois, à froid), quelques comparaisons Vulkan/Cairo ont échoué (image absente côté
   GPU ?) ; non reproduit en une trentaine de passages normaux.

Vérifications croisées depuis Linux : `rustup target add aarch64-apple-darwin
x86_64-pc-windows-msvc` puis `cargo clippy --workspace --all-targets --target …` vérifie
les backends macOS et Windows sans éditeur de liens (ni les tests ni les exports de ces
cibles ne tournent ici).

## Backend macOS (en cours)

`src/platform/macos/` (liaisons `objc2-*`, sous `cfg(target_os = "macos")`) est
`platform::Native` sur macOS. Faits : les services hors écran et une première boucle
AppKit (`window/`).

- Texte Core Text (`graphics/text/`) : une mise en page `CTTypesetter` sert la mesure,
  le dessin, les carets et les rectangles des liens ; hauteur naturelle et interlignage
  uniforme calqués sur DirectWrite, ellipses de fin et de début, `line-clamp`,
  justification, décorations dessinées par le moteur, couleurs par portion.
- Polices (`fonts.rs`) : polices embarquées privées (WOFF2 compris), axe `wght` des
  polices variables, listes CSS ; `Segoe UI`, `system-ui`, `sans-serif` → police
  d’interface macOS ; `Consolas`, `monospace` → police à chasse fixe du système.
- Rendu (`graphics/`) : Core Graphics sur bitmap BGRA prémultiplié sRGB (CPU, les modes
  GPU et logiciel sont identiques), libellé `CPU · Core Graphics`. Filtres, ombres
  portées et `backdrop-filter` calculés sur CPU (`filters.rs`), masques et texte rempli
  par masque en niveaux de gris (`layers.rs`).
- Images et PNG par ImageIO (`imaging.rs`), compteurs mach/`proc_pid_rusage`.

Les exports de référence tournent sous macOS (`target/release/examples/...` sans
`.exe`) : même mise en page que Windows, polices et anticrénelage différents. Ils
ne sont pas identiques octet pour octet aux images Windows.

Boucle AppKit (`window/`) : boucle écrite à la main (`nextEventMatchingMask`
jusqu’au prochain réveil du runtime, qui sert de minuterie ; événement « application »
posté par les tâches de fond pour réveiller le thread UI). Une `NSWindow` par
descripteur ; sans `title_bar`, contenu sous une barre de titre transparente et boutons
standard masqués (l’application dessine les siens, `data-drag-region` déplace la
fenêtre, double clic = zoom). La vue (`view.rs`) affiche l’image du renderer comme
contenu de son calque (CPU, image entière à chaque peinture). Souris, molette (lignes ×
16 px sans défilement précis), clavier (codes `kVK_*`, ⌘A/C/X/V/Z, ⌥←/→, ⌘←/→, ⌘Q/W/M),
curseurs, `NSPasteboard`, commandes `Open`/`Close`/`Action`/`Exit` et politique de
sortie.

Performance de la peinture CPU (fenêtre Retina 2560×1580, accueil de RustMusic) :
250 ms par image au départ, ~36 ms pour une image entière, ~5 ms pour un survol.
- `graphics/damage.rs` : seule la zone modifiée est redessinée (empreinte de chaque
  élément et de son état, bornes en pixels ; un groupe filtré compte pour un élément ;
  un `backdrop-filter` touché agrandit la zone à ce qu’il relit). Test de référence :
  redessin partiel identique au pixel près à un dessin complet.
- `graphics/cache.rs` : groupes filtrés en cache par signature du contenu (la signature
  de Windows est désormais commune : `ui/rendering/scene/signature.rs`),
  `backdrop-filter` par empreinte des pixels relus, images redimensionnées une fois à
  leur taille d’affichage et posées sur les pixels.
- Flou gaussien calculé sur une image réduite au-delà de quelques pixels d’écart type.
- Opacité de groupe : contenu invisible (opacité nulle) sauté, pas de calque pour un
  seul élément, calques bornés au contenu (`contentBounds` de Direct2D).
- Fenêtre en sRGB : Core Animation ne convertit plus l’image vers l’espace de l’écran
  sur le CPU (−15 ms par image).
- Zones modifiées en liste de rectangles (pas un rectangle englobant) ; une découpe
  rectangulaire n’entre pas dans l’empreinte (l’ouverture d’un menu ajoute autour de la
  page une découpe de la taille de la fenêtre, sans effet visible).
- Mouvements qui changent toute la fenêtre (repli de la barre latérale, défilement) :
  images en demi-résolution agrandies par Core Animation (~21 ms au lieu de ~70 ms),
  image nette dès l’arrêt (`refine_at` dans `window/session.rs`). Les petites
  animations (survol, menus) restent nettes en redessin partiel.
- Groupes à grand flou dessinés réduits (facteur ≤ 8 selon le flou), pochettes animées
  étirées puis redimensionnées exactement à l’arrêt.
- Mesures (Retina, accueil) : repli de la barre ~21 ms par image de mouvement,
  défilement ~15 ms, survols 7–11 ms, ouverture d’un menu 27–53 ms (flou d’arrière-plan
  du menu), curseur de volume ~8 ms (dont ~6,5 ms de reconstruction de la vue par le
  moteur, coût commun à Windows).
- `--profile` écrit le temps de préparation et de dessin de chaque image et les zones
  redessinées.

Comportement des couches (moteur commun, donc aussi Windows) : une couche ouverte ou
fermée après une interaction à la souris déplace le focus sans le montrer ni afficher
l’aide du contrôle (avant : l’aide « Changer de source » restait sur le bouton voisin
après un choix, l’aide du premier élément d’un menu s’affichait à son ouverture). Au
clavier (Entrée, Échap…), le focus et l’aide sont montrés comme avant.
Test : `ui/tests/overlays/tooltip.rs`
(`closing_a_popover_with_the_pointer_restores_focus_without_showing_its_help`).

Clic hors d’un popover (menu), décidé le 2026-11-05 : comme une page web, l’appui
ferme le popover et atteint l’élément dessous (rejoué par `prepare_at` une fois la
vue reconstruite, capture rendue au relâchement). Sur l’ancre du popover, l’appui le
referme seulement ; une modale garde son fond bloquant. Avant, ce clic était consommé
(menus natifs) : choisir une bibliothèque pendant que le menu Source était ouvert
demandait deux clics. Test : `ui/tests/overlays/popover/panel.rs`
(`outside_click_closes_the_popup_and_reaches_the_control_underneath`).

Saisie de texte (`window/text.rs`) : `keyDown:` garde les touches et raccourcis du
moteur, le reste passe par `interpretKeyEvents:` (`NSTextInputClient`) : texte, touches
mortes, IME, accents par appui long ; un clic valide la composition. Options de fenêtre :
position `x`/`y` depuis le coin haut-gauche de l’écran principal, `topmost`,
`transparent`, `click_through`. Barre de menus (`window/menu.rs`) : application, Édition,
Fenêtre, libellés français ou anglais selon la langue du système.

Moteur commun : la cascade des styles n’est plus relancée à chaque image pour les seules
keyframes (opacité, transformations), uniquement pour les transitions de couleurs et de
longueurs (`Timeline::animating_styles`) : `bootstrap_demo` au repos, spinners animés
hors de la vue, passe de 30 % à 11 % de CPU.

Accessibilité (`accessibility/`, `window/accessible.rs`) : un `NSAccessibilityElement`
par élément de l’arbre du moteur, créé à la première interrogation d’un lecteur d’écran ;
rôles (dont les repères `AXLandmark…`), libellés, aides, valeurs (texte, case, curseur,
option choisie), cadre à l’écran, actions (appui, incrément, valeur, focus), test de
position et élément focalisé ; notifications de focus, de valeur et de mise en page
seulement quand un lecteur d’écran est actif.

Mémoire (RustMusic, fenêtre Retina, chemin Core Graphics avant l’étape 4 du renderer
Metal ; mesures à jour plus bas) : empreinte 124 Mo, dont ~18 Mo de pochettes
décodées par l’application, ~22 Mo de surfaces de rendu, ~31 Mo d’images Core Graphics
(copie de l’image présentée, pochettes redimensionnées, caches de filtres) et ~40 Mo
d’allocations du moteur (arbre, styles, caches de texte), communes à Windows. Les images
du moteur sont données à Core Graphics sans copie (`bitmap::image`, l’`Arc` est retenu
par le fournisseur de données) ; la surface de mouvement est libérée après l’image nette.

Renderer GPU Metal des fenêtres (`metal/`, décision du propriétaire), étapes 1 à 4
faites ; c’est le chemin par défaut des fenêtres (détail :
[architecture](architecture.md#présentation-à-lécran-macos)) :

- Infrastructure : GPU économe préféré sur les Mac à deux GPU, shaders MSL compilés au
  lancement, un pipeline de quads instanciés, `CAMetalLayer` sous-calque de la vue,
  présentation dans la transaction Core Animation de la session, tampons `Shared` en
  mémoire unifiée, textures `Shared` sur GPU Apple seulement, `Managed`/`Private` sur GPU
  dédié. Le rendu hors écran reste Core Graphics ; `metal::Renderer::render` rend vers
  une texture relue pour les tests.
- Chemin par défaut (règle) : Metal en `RenderMode::Auto` (défaut) et `Gpu` dès qu’un
  périphérique Metal existe ; Core Graphics en `RenderMode::Software`
  (`--renderer=software`), sans périphérique, si les shaders ne compilent pas, après une
  erreur du GPU (toute la vie de la fenêtre), pour une image sans drawable, et sous
  `KARBON_METAL_FALLBACK` (diagnostic). Core Graphics garde le redessin partiel et la
  demi-résolution pendant les mouvements. Passage propre dans les deux sens (une
  transaction), vérifié par la sonde native. `--profile` écrit le chemin de chaque
  image, son temps de préparation et de dessin, la traduction, l’encodage, l’attente, le
  temps GPU de l’image précédente, la mémoire GPU et la raison d’un repli.
- Toutes les commandes de la scène passent par Metal : formes, traits, dégradés,
  segments, polygones, images, icônes, SVG, textes, découpes, effets, et (étape 3)
  groupes filtrés (`PushFilter` : flou, ombre portée, luminosité, contraste, saturation,
  niveaux de gris, sépia, inversion, rotation de teinte, opacité, enchaînés),
  `backdrop-filter`, `mask-image` et `background-clip: text`, émojis compris (masque de
  luminance, étape 4) : plus aucune scène ne passe par Core Graphics.
- Écarts Metal/Core Graphics mesurés par les tests (écart moyen par canal sur 255, à 1x ;
  2x plus faible) : texte droit 0,05 (au plus 1 : glyphes identiques), segments,
  polygones, icônes et SVG 0,02, découpes 0,04, opacité de groupe 0,17 (arrondis de 1),
  images 0,17, transformations 0,13, formes 0,21 (coins des traits de 1 px : Core Graphics
  couvre entièrement le pixel de l’onglet), dégradés 0,44 ; texte tourné 2,0 (glyphes
  rastérisés droits puis tournés, quand Core Graphics rastérise les contours tournés).
- Temps d’image Metal (`--profile`, M1, Retina) : `bootstrap_demo` 2560×1582 0,8 à
  2,7 ms (préparation 0,2 à 0,4 ms), première image 24 à 49 ms (atlas et textures) ;
  `simple_ui` 1440×1160 0,9 à 1,1 ms ; `multi_window` 800×600 1,5 ms ; `background_task`
  1360×840 5,7 ms (première image).
- Étape 3 (effets) : groupes filtrés rendus dans une couche bornée (réduite pour un grand
  flou, comme Core Graphics), passages de filtre en RGBA 16 bits flottants (flou gaussien
  séparable sur image réduite, ombre, matrices enchaînées), résultat gardé par élément
  tant que la signature du contenu ne change pas ; `backdrop-filter` : la zone relue est
  redessinée dans une couche à sa taille (la cible reste dessinée en un passage),
  filtrée, recomposée sous la forme arrondie ; masques en maillons de la chaîne des
  découpes ; texte rempli par masque de glyphes. Écarts Metal/Core Graphics : voir
  [tests](tests.md#suite-minimale-du-backend-macos) ; scène RustMusic complète
  (`KARBON_METAL_COMPARE`) : écart moyen 0,22 à 0,30 sur 255, 0,2 % des pixels au-delà de
  16 (pochettes redimensionnées autrement).
- RustMusic par Metal, mesuré le 2026-11-05 (M1, fenêtre Retina 2880×1582, `--profile`,
  pilote jetable qui envoie à la vue des événements AppKit synthétiques : 120 survols à
  16 ms d’intervalle, 120 crans de défilement de 40 px, quatre menus ouverts puis
  survolés, deux replis et deux déploiements de la barre latérale, 90 redimensionnements
  de ±200 pt) ; temps total moyen / p95 (dont dessin Metal moyen), même pilote en
  `--renderer=software` pour Core Graphics :

  | Scénario | Metal | Core Graphics |
  | --- | --- | --- |
  | survol | 3,9 / 4,7 ms (dessin 1,9) | 7,8 / 39,3 ms |
  | défilement | 3,9 / 8,9 ms (dessin 2,5) | 16,0 / 36,2 ms |
  | menus | 4,9 / 13,3 ms (dessin 1,9) | 29,9 / 63,9 ms |
  | barre latérale | 9,9 / 13,7 ms (dessin 2,1) | 33,3 / 81,0 ms |
  | redimensionnement | 16,6 / 23,2 ms (dessin 3,5) | 68,9 / 89,1 ms |

  Le dessin Metal reste sous 4 ms (p95 ≤ 5,1 ms) ; au-delà, c’est la préparation du
  moteur (reconstruction de la vue et mise en page, commune à Windows) : ~15 ms à
  l’ouverture d’un menu, ~8 ms par image de la barre latérale, ~13 ms par image de
  redimensionnement. Temps GPU rapporté par Metal : 6 à 9 ms par image quelle que soit la
  scène (`bootstrap_demo` 3 à 5 ms) ; une sortie anticipée du shader ne le change pas,
  il suit la fréquence choisie par le GPU plutôt que le travail.

Étape 4 (2026-11-05) : chemin par défaut, mémoire, charge, Mac Intel, encodage, points
ouverts. Mesures : M1, RustMusic, fenêtre Retina 2880×1582, pilote jetable d’événements
AppKit (repos 5 s, 120 survols, 120 crans de défilement, quatre menus, barre latérale,
90 redimensionnements, réduction dans le Dock), même pilote en `--renderer=software`.

- Mémoire (`phys_footprint`, `footprint`) après chargement et 5 s de repos : Metal
  170 → 139-148 Mo, Core Graphics 124 → 114-117 Mo. Détail Metal : drawables du
  `CAMetalLayer` 53 Mo (trois de ~17 Mo ; deux font passer un redimensionnement de 17 à
  30 ms par image), tas 48 Mo (moteur, pochettes décodées), mémoire GPU 33 Mo (textures des
  pochettes 15 Mo, cibles de l’image 13-14 Mo, atlas 1-2 Mo, rastérisations 1 Mo, groupes
  filtrés 0,6 Mo). Core Graphics : tas 57 Mo, surface de la fenêtre 26 Mo, images Core
  Graphics 25 Mo. Gains : la première image n’est plus dessinée à une taille plus haute
  que l’écran (fenêtre ramenée dans l’écran avant elle : −24 Mo de drawable gardé en
  Metal, −10 Mo de surface en Core Graphics, une image entière de moins au lancement) ;
  caches Core Graphics jamais alloués en Metal (libérés au passage) ; caches Metal bornés
  et allégés une seconde après la dernière image (mémoire GPU 67 → 29 Mo après les menus) ;
  tout le GPU libéré dans le Dock (216 → 196 Mo). Après tout le scénario : Metal ~220 Mo,
  Core Graphics ~198 Mo ; la croissance commune vient du tas (fragmentation après menus et
  redimensionnement : tas vivant 43-49 Mo pour 80-110 Mo de pages sales, sans fuite ;
  `malloc_zone_pressure_relief` n’en rend rien).
- Charge : au repos, aucune image (pas de boucle d’affichage) ; CPU du processus 1 ms en
  5 s dans les deux chemins. Temps CPU du processus par scénario, Metal / Core Graphics :
  survol 344 / 899 ms, défilement 491 / 1713 ms, menus 866 / 1515 ms, barre latérale
  595 / 1438 ms, redimensionnement 2192 / 7393 ms. Temps GPU rapporté : 6 à 10 ms par
  image quelle que soit la scène (fréquence du GPU), zéro au repos. Énergie : non mesurée
  (`powermetrics` demande les droits administrateur).
- Temps d’image Metal (moyen / p95) : survol 2,2 / 4,2 ms, défilement 3,9 / 7,6 ms,
  menus 4,3 / 11,7 ms, barre latérale 11,2 / 15,1 ms, redimensionnement 16,6 / 26,2 ms
  (Core Graphics : 7,7 / 40,6, 17,8 / 47,7, 31,3 / 73,3, 37,7 / 96,1, 71,7 / 98,7 ms).
- Coût d’encodage CPU par image : traduction de la scène 0,3-0,4 ms au survol, 0,6-1,2 ms
  au défilement, 1,3-1,7 ms pendant la barre latérale et le redimensionnement (dont
  ~0,4 ms de mises en page Core Text aux nouvelles largeurs et jusqu’à 0,8 ms de
  rastérisations d’icônes aux nouvelles positions sous-pixel) ; encodage Metal
  0,2-0,5 ms. Décision : pas de cache des instances par sous-arbre (`Signature`) : sous
  1 ms dans les scènes stables, et au-delà le coût vient de contenus qui changent
  (largeurs, positions) qu’un tel cache ne garderait pas.
- Mac Intel / GPU dédié : textures `Shared` réservées aux GPU Apple (les GPU Intel et
  AMD les refusent, même intégrés : la règle précédente, « mémoire unifiée », les aurait
  données à un GPU Intel intégré) ; `KARBON_METAL_STORAGE=managed` force `Managed`
  partout (tampons signalés par `didModifyRange`, relecture synchronisée par blit) : toutes
  les comparaisons Metal/Core Graphics passent, et le rendu `Managed` est identique au
  pixel près au rendu par défaut. Binaire `x86_64-apple-darwin` sous Rosetta : tests Metal
  verts (stockage par défaut et `Managed`), RustMusic par le pilote (images et mémoire du
  même ordre). Ne se vérifie que sur un vrai Mac Intel : les copies réelles du pilote
  `Managed`, un GPU Intel/AMD (shaders, fréquences, temps), la commutation de GPU. Perte de
  GPU (lecture de code et chemin d’erreur) : tampon de commandes impossible à créer ou
  image précédente en erreur → la fenêtre passe à Core Graphics.
- Points ouverts de l’étape 3 : (a) émoji sous `background-clip: text` dessiné par Metal
  en masque de luminance, dernier repli supprimé ; (b) décalage d’un pixel d’une ligne de
  texte : reproduit sur RustMusic pendant le défilement (`KARBON_METAL_COMPARE`, titre
  « Smells Like Teen Spirit » : première ligne d’ordonnée 1098,1667 px dessinée en 1098
  par Core Graphics, en 1099 par Metal, seconde ligne identique ; repère sans
  composition, échelle 2 seule). Non reproduit isolément : balayages au vingtième de
  pixel, polices système, Inter et Figtree, tailles 11 à 60, deux lignes serrées, dans un
  groupe à opacité, un masque, une découpe arrondie, un groupe filtré, sous translation :
  Core Graphics y prend toujours l’entier supérieur, comme Metal. Ni les arrondis `f32`
  du repère (calcul en `f64` essayé, sans effet) ni le moteur ne sont en cause ; la règle
  d’arrondi de Core Graphics change dans ce contexte, cause non trouvée. Écart connu :
  l’image d’un émoji (Core Graphics la pose à sa position verticale fractionnaire).

Reste :

- Mesurer sur un vrai Mac Intel (GPU intégré Intel et dédié AMD, commutation, temps).
- Libérer les drawables (53 Mo) dans le Dock : retirer le `CAMetalLayer` et le recréer à
  la sortie demande de vérifier l’animation de retour du Dock.
- Pression mémoire système (source `DISPATCH_SOURCE_TYPE_MEMORYPRESSURE`) : non branchée.
- Barre latérale et redimensionnement : voir « Préparation du moteur » ci-dessous.
- Décalage d’un pixel du titre de RustMusic pendant le défilement (cause dans Core
  Graphics non trouvée) et position verticale des émojis.

## Préparation du moteur (2026-11-05)

Moteur commun (macOS et Windows) : `UiRuntime::prepare_at` et ce qu’il appelle. Mesuré sur
RustMusic avec le pilote jetable de la section Metal (fenêtre Retina 2880×2080, renderer
Metal, `--profile`) et un banc headless jetable qui rejoue les mêmes scénarios avec le
mesureur Core Text et un temps par phase (instrumentation temporaire, non commitée).

Répartition avant (fenêtre, ms de préparation par image en moyenne) :

| Scénario | Préparation | Postes principaux |
| --- | --- | --- |
| survol | 2,8 | cascade 2,0 (relancée à chaque déplacement), animations 0,3, peinture 0,3 |
| défilement | 1,9 | animations 0,7 (2,4 préparations par image), ombres 0,6, peinture 0,3 |
| menus | 12,0 | cascade 9,1 (3,4 par image : transitions en cours), animations 1,3 ; ouverture ≈ 17 : vue 5,1 + seconde vue 4,0, layout 4,6, ombres 1,9 |
| barre latérale | 8,0 | layout 6,0 dont mesures de texte 2,2 (1 900 par image), cascade 0,7, ombres 0,3 |
| redimensionnement | 13,7 | layout 6,3 dont texte 2,5, ombres 4,1, vue 0,9 + index 0,5, cascade 0,6 + 0,4 (requêtes de conteneur) |

Faits (un commit chacun ; détail dans [l’architecture](architecture.md#mémoires-de-la-préparation)) :

1. **Mesures d’une passe de layout** (`layout/memo.rs`) : 46 000 appels de `measure` par
   image de redimensionnement pour 6 000 contraintes différentes ; un appel déjà fait rend
   sa taille. Layout −40 % (banc : barre latérale 4,3 → 2,6 ms, redimensionnement
   5,9 → 3,7 ms).
2. **Mesures de texte d’une image à l’autre** (`runtime/text_cache/`) : une mesure à la
   largeur `W` sert entre la ligne la plus large + 0,5 et `W`. Mesures de texte du layout
   −65 à −75 % pendant la barre latérale et le redimensionnement.
3. **Faits du pointeur** : la cascade n’est plus relancée quand le pointeur reste sur les
   mêmes éléments (survol : 0,4 cascade par image au lieu de 1,1).
4. **Une seule construction de la vue** à l’ouverture ou la fermeture d’une couche quand
   l’annulation n’a livré aucun message.
5. Tables internes hachées par `core/hash.rs` ; poids bilinéaires des ombres floutées par
   colonne et par ligne (−15 % par ombre, images identiques).

`UiRuntime::set_incremental(false)` désactive ces mémoires : les tests d’équivalence
(voir [tests](tests.md)) jouent les mêmes interactions avec et sans elles. Exports de
référence identiques octet pour octet.

Après (fenêtre : trois passages alternés avant/après du même pilote, toutes les images
réunies, total moyen / p95 et préparation moyenne) :

| Scénario | Avant | Après |
| --- | --- | --- |
| survol | 4,0 / 5,4 ms (prép. 2,2) | 3,6 / 5,3 ms (prép. 1,5) |
| défilement | 4,0 / 7,9 ms (prép. 1,5) | 4,0 / 7,6 ms (prép. 1,4) |
| menus | 5,1 / 13,8 ms (prép. 3,4, p95 12,6) | 4,7 / 9,5 ms (prép. 2,8, p95 8,2) |
| barre latérale | 10,2 / 15,0 ms (prép. 8,0) | 6,3 / 11,5 ms (prép. 4,4) |
| redimensionnement | 15,8 / 23,7 ms (prép. 12,4) | 11,5 / 17,2 ms (prép. 8,2) |

Banc headless (Core Text, ms par pas, moyenne / p95) : survol 1,33 / 6,1 → 0,81 / 3,6 ;
ouverture ou fermeture d’un menu 9,9 / 12,7 → 5,0 / 6,2 ; barre latérale 6,3 / 11,1 →
3,1 / 6,3 ; redimensionnement 11,6 / 19,4 → 5,6 / 11,7. Les temps de la fenêtre suivent
aussi la fréquence du processeur (images courtes et espacées) : comparer des passages
alternés du même pilote, pas des mesures prises à des moments différents.

Reste :

- **macOS, redimensionnement à la souris** : fait. `windowWillStartLiveResize` /
  `windowDidEndLiveResize` appellent `UiRuntime::set_live_resize` comme sous Windows :
  pendant le geste, les ombres floutées des pochettes (250 px, flou de 32 px) sont étirées
  au lieu d'être recalculées à chaque pas. Mesuré avec le pilote qui envoie les deux
  notifications autour de ses `setFrame:display:` (deux passages alternés) : 11,3 / 18,1 →
  7,3 / 10,0 ms (total moyen / p95), traduction Metal 1,0 → 0,6 ms. La fin du geste
  repeint les ombres exactement.
- Redimensionnement : la vue est reconstruite à chaque taille (≈ 0,8 ms, contrat de
  `view(&ViewContext)`) ; la seconde cascade des requêtes de conteneur (≈ 0,4 ms) pourrait
  être sautée quand aucune requête ne change d’état (même méthode que les faits du
  pointeur).
- Transition de longueur (barre latérale) : cascade complète à chaque image (≈ 0,7 ms)
  pour une seule largeur animée ; layout encore ≈ 2 ms.
- `Timeline::sync` à chaque préparation dès qu’une piste existe (0,3 à 0,9 ms en
  défilement et dans les menus) ; plusieurs préparations par image dans la boucle macOS
  avec le pilote (3,9 dans les menus), chacune relance la cascade pendant une transition.

## État actuel des styles

Les cinq familles d’extension demandées sont raccordées au moteur :

1. Propriétés communes des contrôles, héritage typographique, états, métriques
   DirectWrite partagées par le texte, l’édition et l’accessibilité.
2. Border/content-box, marges auto/négatives, flex-wrap/basis/grow/shrink,
   pistes de grille variables, relative/absolute/fixed, z-index et overflow.
3. Coins indépendants et masques arrondis, ombres de boîte en cache,
   object-fit/position, fonds multiples, radiaux et filtres Direct2D 1.1.
4. Expressions compilées em/rem/vw/vh/calc/min/max/clamp, media queries imbriquées
   et variables typées de l’application avec partage Arc/copie à l’écriture.
5. Sélecteurs composés/pseudo-classes, héritage et keyframes : pistes indépendantes
   par propriété (étapes partielles, courbe par étape), couleurs, listes
   d’animations, `reverse`/`alternate-reverse`, délai négatif et `animation-play-state`.

La [couverture exacte](css.md) est le contrat de référence : il ne s’agit pas
d’un navigateur CSS complet. Les min-max/rayons restent en px,
Scroll est un composant explicite, certaines propriétés ne sont pas conditionnelles,
les sélecteurs ne parcourent pas encore les relations ancêtre/frère, et les
keyframes animent opacité, transformations et couleurs (pas les tailles ni les
ombres ; pas d’itérations fractionnaires ni de `steps()`).
Ne pas annoncer la prise en charge de toute propriété CSS.

## Exemples

Cinq exemples restent : `rustmusic_html` (accueil de RustMusic en templates HTML et
CSS, voir [rustmusic-html.md](rustmusic-html.md)), `bootstrap_demo` (Bootstrap 5.3 en
mode framework), `simple_ui`, `multi_window` et `background_task`. Les anciens
`rustmusic` (natif), `css_features`, `template_ui`, `animation_playground` et
`profile_session` ont été retirés ; leurs scénarios sont couverts par `tests/` (voir
[l’organisation des tests](tests.md)) et le profilage passe par `--profile`
([profilage](presentation-profiling.md)). Les exemples restent indépendants de la
bibliothèque.

Exports de référence, à garder identiques octet pour octet quand le rendu ne doit
pas changer :

```powershell
cargo build --release --example rustmusic_html --example bootstrap_demo
./target/release/examples/rustmusic_html.exe --export x.png --size 1280x900 --scale 1.5 --renderer=software
./target/release/examples/rustmusic_html.exe --export x.png --size 960x700 --scale 1.5 --renderer=software
./target/release/examples/bootstrap_demo.exe --export x.png --size 1280x1720 --scale 1.5   # deux fois : la première image GPU après une construction peut différer
```

## Organisation et points sensibles

- `src/ui/styling` : héritage, états, propriétés, variables et texte formaté.
- `src/ui/layout` : boîtes, flex, grilles, positions et expressions statiques.
- `src/ui/rendering` : scènes indépendantes de l’OS, ombres et filtres.
- `src/platform/windows/graphics` : D2D Factory1/device/context commun GPU/WARP,
  caches de fontes/chemins/dégradés, listes de commandes et graphes de filtres.
- `crates/ui-macros/src/css` : parsing et diagnostics pendant la compilation.

Les nouvelles constantes de méthodes COM conservent le suffixe numérique demandé
par l’utilisateur. Ne pas transformer les index ABI COM en HashMap.
Les contrôles éditables, sliders, selects, couches, scrolls et drag/drop conservent
leurs restrictions de transformations spatiales. Les fondus fonctionnent.
Les fenêtres fixed ne peuvent pas être sous un ancêtre animé/transformé/filtré.
Les ombres ont un budget par composant ; les filtres ont un budget par groupe,
pas un plafond RAM global de l’application.

Une régression flex a été trouvée par export : basis:0 mesurait le texte à
largeur zéro et pouvait pousser le contenu hors écran. La mesure de la taille
transversale se fait désormais après distribution de la largeur. Le test dédié
est dans `tests/css/layout/flex.rs`. Les tests couvrent aussi les raccourcis
conditionnels qui doivent remettre à zéro les côtés/gaps précédents
(`tests/css/selectors.rs`).

## Vérifications

```powershell
cargo fmt --all --check
cargo clippy --offline --workspace --all-targets -- -D warnings
cargo clippy --offline --workspace --all-targets --target x86_64-unknown-linux-gnu -- -D warnings
cargo clippy --workspace --all-targets --target aarch64-apple-darwin -- -D warnings    # depuis Linux
cargo clippy --workspace --all-targets --target x86_64-pc-windows-msvc -- -D warnings  # depuis Linux
cargo test --offline --workspace --all-targets
cargo test --offline --lib -- --include-ignored
cargo test --offline --doc --workspace
cargo doc --offline --no-deps --workspace
cargo build --offline --release --workspace --all-targets
```

292 tests unitaires du socle sous Windows (279 sous macOS, dont les 26 du renderer
Metal ; 252 sous Linux, dont les 19 du backend), 53 du compilateur (`karbon-ui-macros`), 146
d’intégration (`tests/css`, `tests/templates`, `tests/frameworks`,
`tests/interaction`) et 11 de l’exemple `rustmusic_html`. Sont ignorés par défaut
les deux tests GPU du socle (rendu, présentation par composition), qui demandent un
adaptateur matériel, et les quatre outils de mesure du compilateur. WARP couvre le
logiciel. Les tests natifs de présentation créent des fenêtres cachées :
composition, ResizeBuffers, perte simulée du périphérique, repli layered et
WM_NCHITTEST. Pixels vérifiés à 1x/1,5x/2x pour le texte, les coins, clips,
gradients, alpha et filtres. Les différences d’arrondi et d’anticrénelage GPU/WARP
sont possibles. Le détail, et le contrat qu’un nouveau backend doit passer, sont
dans [l’organisation des tests](tests.md).

## Suites possibles

Le menu contextuel intégré des champs de saisie (`ui/components/text_input/menu.rs`,
ajouté le 2026-11-06 : Couper, Copier, Coller, Tout sélectionner, dessiné par le moteur
comme la liste du `Select`) n’est pas encore exposé aux lecteurs d’écran : ses entrées
n’apparaissent pas dans l’instantané accessible, seuls les raccourcis clavier le
remplacent (Ctrl+C/V/X/A). L’exposer demande un rôle de menu dans `AccessibleElement`,
porté par les trois ponts.
Le scroll CSS, les sélecteurs relationnels,
les keyframes de taille et d’ombre et les itérations fractionnaires ; ensuite
mesurer la présentation sur une UI réelle avant de promettre des performances.
La validation sous Linux de la [revue](linux-review.md) et les recettes avancées de
lecteurs d’écran restent à traiter. La
préférence système de mouvement réduit est suivie (macOS : « Réduire les animations »,
Windows : « Afficher les animations dans Windows »), en plus de
`WindowOptions::reduced_motion`.
