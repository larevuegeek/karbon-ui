# Organisation des tests

Les tests sont rangés en deux familles : le **contrat**, indépendant de la
plateforme, que tout backend (Windows, headless, futurs macOS et Linux) doit
respecter, et la **suite minimale d’un backend**, qui sert aussi de modèle pour
écrire les suivants. Le contrat a deux étages : les tests unitaires du moteur
(`src/ui/tests/`, `src/platform/headless/tests/`) et les tests d’intégration de
`tests/`, qui passent par les macros (`ui!`, `ui_file!`, `css_file!`) comme une
application.

## Contrat du moteur : `src/ui/tests/`

Ces tests ne passent que par `UiRuntime`, un mesureur de texte déterministe et, au
besoin, le harnais headless.

- `support.rs` : outils partagés (`Msg`, `view_runtime`, `Measure`, `NoResources`,
  `Counter`, `prepare`, `center`, `key`, `wheel`, `near`…).
- `runtime/` : pointeur, clavier, séparateurs et zones de déplacement (`basics`),
  tabulation et révélation du focus (`focus`), messages typés (`messages`), tâches (`tasks`).
- `layout/` : lignes, grilles, contraintes (`basics`), texte ajusté (`text_fit`),
  défilement (`scroll`), listes virtualisées (`virtual_list`), mémoire des mesures d’une
  passe (`incremental` : 400 arbres tirés au hasard, mis en page à quatre tailles de
  suite avec et sans elle, doivent laisser les mêmes rectangles, découpes, tailles
  naturelles et limites résolues).
- `components/` : cases et radios (`choices`), listes de choix (`select/`), curseurs
  (`slider`), champs de saisie (`text_input/` : édition, accessibilité, multiligne).
- `overlays/` : dialogues modaux (`modal`), popovers et menus (`popover/`), infobulles (`tooltip`).
- `styling.rs` : styles de boutons, surfaces et textes partagés selon le thème.
- `animation/` : animations, transitions, révélation en hauteur, mouvements réduits
  (`mod`) ; étapes par propriété, courbes d’étape, directions, délai négatif, pause et
  listes d’animations (`keyframes`).
- `drag_drop.rs` : glisser-déposer et défilement en bord de zone.

Les autres modules du moteur gardent leurs tests à côté du code (`foo/tests.rs` ou
`mod tests`) : `ui/view`, `ui/styling/{tokens,color}`, `ui/rendering/{scene,svg,shadow,
component_shadow}`, `ui/animation/timeline`, `task/executor`, `resources`, `window`,
`error`, `application`… Parmi eux, `ui/runtime/text_cache/tests.rs` compare la mémoire
des mesures de texte au mesureur du backend natif (Core Text, DirectWrite ou headless)
à mille largeurs : coupures, ellipses de fin et de début, `line-clamp`, centrage,
espacement des lettres, justification, paragraphes enrichis.

Tests d’équivalence des mémoires de la préparation (mesures d’une passe, mesures de
texte, faits du pointeur, construction unique de la vue à l’ouverture d’une couche) :
la même suite d’interactions est jouée avec et sans elles
(`UiRuntime::set_incremental`), et chaque étape doit donner le même arbre placé (bornes
au bit près), les mêmes textes, le même arbre accessible et les mêmes pixels du
rastériseur headless. Pages jouées : `tests/css/layout/incremental` (flex imbriqué,
grille automatique, limites relatives, ligne de base, requêtes média et de conteneur,
textes coupés, transition de largeur, survol par petits pas), la démo Bootstrap
(`tests/frameworks/bootstrap/demo/incremental` : accordéon animé, défilement, points de
rupture) et RustMusic (`examples/rustmusic_html/tests.rs` : menus, barre latérale,
défilement, redimensionnement, recherche).

## Contrat « application headless » : `src/platform/headless/tests/`

Harnais `HeadlessApp` : entrées (`input`), fenêtre et commandes (`window`), temps
virtuel et tâches (`time`), rendu logiciel et protocole hors écran (`render`),
texte riche (`rich_text`). Le rastériseur et les métriques ont leurs tests dans
`raster/tests.rs` et `text/tests.rs`.

## Suite minimale du backend Windows

Modèle à reproduire pour chaque nouveau backend :

- `src/platform/windows/tests/` (fenêtres cachées) : cycle de vie et identités des
  fenêtres (`window`), chemin de présentation et de peinture (`presentation`),
  entrées natives vers `UiEvent` (`input`), minuteries et cadence d’images
  (`timers`), réveil et livraison des tâches (`tasks`), rendu hors écran
  (`offscreen`), décodage d’images (`images`) ;
- `src/platform/windows/graphics/tests/` : scène → pixels sur WARP à plusieurs
  échelles (`paint`, `effects`, `gradients`, `shapes`, `svg`), chaîne d’échange
  (`swapchain`) et mesure du texte DirectWrite (`text`) ;
- à côté de leur module, parce qu’ils lisent des types privés : pont UI Automation
  (`accessibility/`), composition, enregistreur de présentation, UTF-16 et menus
  contextuels (`events/`), encodeur JPEG (`imaging.rs`).

Les tests marqués `#[ignore]` demandent un adaptateur GPU matériel : ce sont les
seuls qui couvrent le chemin GPU.

## Suite minimale du backend macOS

- `tests/macos/` (crate sans harnais : AppKit exige le thread principal) : vraie boucle
  AppKit, fenêtres invisibles. Une application de sonde enchaîne ses étapes par des tâches
  de fond et envoie de vrais événements AppKit à sa fenêtre : erreurs de validation,
  chargement et peinture, clic, frappes, composition IME (`NSTextInputClient`), arbre
  accessible interrogé et appui accessible (NSAccessibility), seconde fenêtre ouverte
  puis fermée, boucle déjà en cours, sortie. La fenêtre est en `RenderMode::Auto` (chemin
  par défaut) et sa vue porte un filtre CSS : la sonde vérifie que le calque Metal
  présente l’image (visible, drawable à la taille de la vue, plus d’image Core Graphics
  sur la vue), que le repli forcé (`KARBON_METAL_FALLBACK`) fait passer l’image à Core
  Graphics (calque Metal masqué) puis que son retrait la rend à Metal, un émoji sous
  `background-clip: text` ajouté entre-temps restant au chemin Metal, et que
  `ViewContext::backend` suit (`GPU · Metal`, `CPU · Core Graphics`, `GPU · Metal`) ;
  sans périphérique Metal, cette partie est sautée ;

- `src/platform/macos/tests/` : protocole hors écran et compteurs du processus
  (`offscreen`), décodage ImageIO, miniature de fond et aller-retour PNG (`images`) ;
- `src/platform/macos/graphics/tests/` : scène → pixels à 1x/1,5x/2x (`paint`), opacité,
  transformations, filtres, `backdrop-filter`, masques et `background-clip: text`
  (`effects`), texte Core Text : polices WOFF2 par graisse, retour à la ligne,
  interlignage, `line-clamp`, carets et clusters, lignes coupées d’un champ multiligne, ellipses,
  paragraphes enrichis (`text`) ;
- `src/platform/macos/metal/tests/` : renderer Metal. Chaque scène est rendue par Metal
  (texture relue) et par Core Graphics, puis comparée à 1x et 2x avec une tolérance
  d’anticrénelage (écart moyen par canal, part des pixels écartés de plus de 16) :
  formes, coins indépendants, traits, dégradés linéaires et radiaux, segments,
  polygones, icônes et SVG (`shapes`), images, texte des polices système et embarquées,
  couleurs par portion, décorations, coupe, texte tourné (`text`), découpes
  rectangulaires, arrondies et imbriquées, transformations, opacité de groupe (`effects`),
  groupes filtrés : flou petit, réduit et très grand, ombre portée, chaque matrice de
  couleur, enchaînements (dont grand flou + saturation des pochettes), groupe filtré dans
  un groupe à opacité transformé, résultat gardé tant que le contenu ne change pas
  (`filters`), `backdrop-filter` (cartes translucides arrondies et superposées sur un fond
  varié, dans un groupe à opacité, ignoré dans un groupe filtré), masques en dégradé
  imbriqués et tournés, texte rempli par un dégradé linéaire, radial ou une couleur
  (`layers`), émoji sous `background-clip: text` en masque de luminance (dégradé, couleur,
  et sans remplissage : écart ≤ 0,25 à la hauteur où la ligne de base tombe sur un pixel,
  voir l’écart connu des émojis dans l’[architecture](architecture.md#présentation-à-lécran-macos))
  et zone relue par un `backdrop-filter` limitée à ce qui a été peint avant (`support`),
  stockage `Managed` (renderer imposé en `Managed` partout : textures gérées, rendu
  identique au pixel près au stockage par défaut sur une scène qui emploie chaque
  ressource, deux images de suite, à 1x et 2x) et bornes des caches (images et
  rastérisations gardées 59 images sans usage puis libérées, cibles en réserve libérées
  après 30 images, `Trim::Idle` ne garde que les images de la dernière image et vide
  réserve et tampons, `Trim::Hidden` libère tout et l’image suivante est identique)
  (`storage`). Sans périphérique Metal (machine virtuelle, intégration
  continue sans GPU), les tests se sautent en l’écrivant sur la sortie d’erreur ;
  `KARBON_REQUIRE_METAL=1` en fait des échecs (machine d’intégration avec GPU), et
  `KARBON_METAL_DUMP=<dossier>` y écrit les deux images et leur écart amplifié. Un
  périphérique présent dont les shaders ne compilent pas est toujours un échec.
  `KARBON_METAL_STORAGE=managed cargo test --lib platform::macos::metal` repasse toutes
  les comparaisons avec le stockage `Managed` partout (`didModifyRange`, textures gérées,
  relecture synchronisée par blit) ; sur Apple Silicon, `Managed` se comporte comme
  `Shared` : les appels sont exercés, pas les copies réelles d’un GPU dédié. Les mêmes
  tests passent sous Rosetta (`--target x86_64-apple-darwin`), en stockage par défaut et
  `Managed` ; le GPU y reste celui d’Apple Silicon.
  Écarts moyens par canal (sur 255, 1x / 2x) : flou petit 0,12 / 0,12, réduit
  0,69 / 0,67, très grand 1,40 / 1,35 (gaussienne exacte contre trois flous en boîte, et
  agrandissement final de Core Graphics en blocs visibles), grand flou + saturation
  2,05 / 1,15, ombre 0,09 / 0,24, matrices de couleur ≤ 0,05, enchaînement 0,25 / 0,38,
  `backdrop-filter` 0,28 / 0,24, masques 0,22 / 0,17, texte rempli 0,04 / 0,03 (au plus 2).
- Scène complète d’une vraie fenêtre : `KARBON_METAL_COMPARE=<dossier>` rend, une seconde
  après la première image Metal puis au plus une fois par seconde, l’image présentée
  aussi par Metal relu et par Core Graphics, écrit les deux et leur écart amplifié, et
  l’écart sur la sortie d’erreur (`metal/compare.rs`). RustMusic (2880×1582, accueil,
  menus ouverts, défilement, barre repliée, redimensionnement) : écart moyen 0,22 à 0,30,
  maximal 110 à 114 (pochettes : redimensionnement Core Graphics de haute qualité contre
  mipmaps), 0,2 % des pixels au-delà de 16.
  Choix retenu plutôt que `#[ignore]` (Windows) : les Mac de développement ont tous un
  GPU Metal, les tests tournent donc par défaut.

## Suite minimale du backend Linux

Services hors écran (Cairo, Pango), boucle Wayland et renderer Vulkan.

- `src/platform/linux/x11/tests.rs` : même sonde (`shell/probe.rs`) sur une vraie
  connexion au serveur X (XWayland compris), échelle tirée de `Xft.dpi`, décodage des
  événements XInput 2 et des axes de défilement (octets construits d’après le protocole) ;
- `src/platform/linux/ime/mod.rs` : nom du fichier d’adresse d’IBus et lecture de
  l’adresse. Le dialogue avec le démon se vérifie sous XWayland par des touches
  synthétiques (saisie Unicode Ctrl+Maj+U) ; Sans serveur
  (`DISPLAY` absent), sautée ; `KARBON_REQUIRE_X11=1` en fait un échec ;
- `src/platform/linux/wayland/tests.rs` : sonde (`shell/probe.rs`, commune avec X11) sur une vraie connexion au compositeur.
  Erreurs de validation, fenêtre configurée et peinte par le chemin par défaut (Vulkan
  s’il y a un GPU, sinon Cairo) (vue construite, échelle reçue),
  tâches de fond qui réveillent la boucle, seconde fenêtre ouverte puis fermée par
  commande, boucle imbriquée refusée (`AlreadyRunning`), sortie. Sans compositeur
  (`WAYLAND_DISPLAY` absent), sautée en l’écrivant sur la sortie d’erreur ;
  `KARBON_REQUIRE_WAYLAND=1` en fait un échec. Les entrées ne peuvent pas être simulées
  par un client Wayland ordinaire : pointeur, clavier, barre de titre et presse-papiers
  se vérifient à la main (`cargo run --release --example rustmusic_html`) ;
- `src/platform/linux/vulkan/tests/` : renderer Vulkan hors écran (GPU s’il y en a un,
  sinon llvmpipe) : formes et peintures du shader au pixel (boîte, coins arrondis, anneau,
  segment, dégradé, découpe, opacité) et boîtes arrondies comparées au rendu Cairo à 1x,
  1,5x et 2x (écart moyen < 0,5 par canal, `instances`) ; scènes traduites (formes,
  traits, segments, dégradés, découpes, transformations, groupes à opacité) comparées au
  rendu Cairo des mêmes scènes, commande non traduite refusée (`scenes`) ; texte
  (tailles, graisses, couleurs par portion, décorations, retours à la ligne, découpe,
  positions fractionnaires, groupe translucide, émojis, rotation) comparé au rendu Cairo
  (`text`) ; images (à leur taille, réduites, agrandies, translucides, tournées, à taille
  animée), polygones, polyligne, icône et SVG droits puis tournés, textures gardées puis
  libérées (`images`) ; groupes filtrés (flous petit, réduit et grand, ombre, chaque
  matrice de couleur, enchaînements, groupe filtré dans un groupe à opacité, résultat
  gardé puis recalculé : `filters`) ; `backdrop-filter` (cartes superposées, dans un
  groupe à opacité, ignoré dans un groupe filtré), masques en dégradé, texte rempli et
  émojis remplis (`layers`). Chaque échelle est rendue deux fois et la seconde image comparée
  (caches du renderer gardés d’une échelle à l’autre). Sans Vulkan, sautés en l’écrivant sur la
  sortie d’erreur ; `KARBON_REQUIRE_VULKAN=1` en fait des échecs.
  `KARBON_VULKAN_DUMP=<dossier>` y enregistre, pour chaque scène comparée et chaque
  échelle, les PNG de Cairo, de Vulkan et de leur écart (amplifié 4 fois). Passent sous la couche
  de validation (`KARBON_VULKAN_VALIDATION=1`, synchronisation comprise), sur RADV et
  llvmpipe (`KARBON_VULKAN_DEVICE=llvmpipe`) ;
- `src/platform/linux/accessibility/tests.rs` : pont AT-SPI sans bus (chemins des
  objets, états, rôles, actions, offsets en caractères, frontières de mot et de ligne).
  Le dialogue avec un vrai client se vérifie avec le client Python d’AT-SPI
  (`gi.repository.Atspi`, celui d’Orca) sur la démo Bootstrap : arbre parcouru, focus
  donné à un champ, texte remplacé puis relu, case cochée par son action, valeur d’un
  curseur, modale ouverte par un bouton ; événements de focus, de texte et de caret
  reçus ;
- `src/platform/linux/tests/` : protocole hors écran et compteurs du processus
  (`offscreen` : `/proc/self/status`, `getrusage`), décodage gdk-pixbuf, miniature de fond
  sur un autre thread et aller-retour PNG par Cairo (`images`) ;
- `src/platform/linux/graphics/tests/` : scène → pixels à 1x/1,5x/2x (`paint`), opacité,
  transformations, filtres CPU, `backdrop-filter`, masques et `background-clip: text`
  (`effects`), texte Pango : polices WOFF2 par graisse (axe `wght`), liste CSS de
  familles, retour à la ligne, interlignage, `line-clamp`, carets et clusters graphèmes,
  lignes coupées d’un champ multiligne, ellipses de fin et de début, paragraphes enrichis,
  police embarquée prioritaire sur une police installée du même nom et `Segoe UI` traduit
  en `system-ui`, polices du bureau suivies par les fenêtres et pas par les exports
  (`text`) ; glyphe rastérisé seul pour l’atlas Vulkan identique au pixel
  près au dessin Cairo, à 16 positions fractionnaires par axe et à 1x/1,5x/2x (masques au
  quart de pixel, émojis au pixel le plus proche : `glyphs`). Mêmes scénarios que la
  suite macOS, qui sert de modèle.

Les tests de contrat (`ui::tests`, `platform::headless`, crates de `tests/`, exemple
`rustmusic_html`) passent sous Linux avec Pango comme mesureur natif, dont
`ui/runtime/text_cache/tests.rs` (mémoire des mesures comparée à Pango à mille largeurs).

## Contrat des templates et des styles : `tests/`

Quatre crates de test, une par thème (`tests/<thème>/main.rs` + modules). Chaque
module compile sa propre feuille, rangée à côté de lui (`selectors.rs` +
`selectors.css`) ; les templates inclus sont dans `tests/templates/ui/`.
`tests/fixtures/` ne garde que les ressources partagées ou tierces (Bootstrap et sa
licence, polices, icône, image), aussi lues par les tests unitaires.

Tout passe par `UiRuntime`, `styling::resolve` + `layout::arrange` ou le harnais
headless, avec des mesureurs de texte déterministes : `Measure` (0,5 em par
caractère) ou `HeadlessTextMeasurer`. Seuls `interaction/previews.rs`
(`diagnostics`, rendu hors écran du backend natif) et `templates/compilation.rs`
(qui lance `cargo check`) sortent de ce cadre.

| Crate | Modules | Rôle |
| --- | --- | --- |
| `css/` (65) | `selectors`, `values`, `text`, `responsive`, `paint`, `motion`, `layout/{boxes, flex, grid, scroll, content, incremental}` | Sous-ensemble CSS compilé : sélecteurs et états, couleurs/unités/variables, typographie, requêtes média et de conteneur, peinture (fonds, bordures, ombres, filtres, masques), keyframes et transitions, mise en page |
| `templates/` (25) | `syntax`, `components`, `html`, `runtime`, `svg`, `textarea`, `compilation` | Templates compilés : blocs et boucles à clé, composants et slots, balisage HTML et feuille par défaut, contrôles natifs pilotés par l’accessibilité, SVG en ligne, champ `<textarea>`, diagnostics du compilateur vus par une crate utilisatrice |
| `frameworks/` (49) | `tailwind/{placement, variants, effects, interaction}`, `bootstrap/{components, controls, demo/}`, `framework_mode` | Utilitaires Tailwind, Bootstrap 5.3 en mode framework (dont la page de `examples/bootstrap_demo` pilotée comme un utilisateur : défilement, accordéon, transitions, rendu à 150 %), cascade générique du mode framework |
| `interaction/` (7) | `menus`, `combobox`, `previews` | Menus et listes de choix en template (placement, clavier, focus, accessibilité), champ à suggestions, aperçus hors écran inspectés et survolés |

`templates/compilation.rs` écrit une crate utilisatrice dans
`target/template-fixture-<pid>`, la vérifie avec `cargo check --offline`, puis y
injecte des templates et des feuilles fautifs : les messages attendus sont le
contrat des diagnostics. C’est le test le plus lent : il compile une crate.

## Contrat d’un nouveau backend

Un nouveau backend réutilise tout le moteur ; il doit passer, sur sa cible,
sans modifier les tests :

1. **Le contrat du moteur** : `cargo test --lib ui::tests` et
   `cargo test --lib platform::headless`.
2. **Le contrat des templates et des styles** :
   `cargo test --test css --test templates --test frameworks --test interaction`.
   Ces tests ne touchent pas le backend natif (sauf `interaction/previews`) : un
   échec y signale un défaut de portabilité du moteur ou des macros, pas du backend.
3. **Sa suite minimale**, écrite sur le modèle de `src/platform/windows/tests/` :
   fenêtres, présentation, entrées vers `UiEvent`, minuteries, tâches, rendu hors
   écran, images, puis scène → pixels sur son rasteriseur.
4. **Les exports de référence**, comparés aux images Windows (même mise en page ;
   l’anticrénelage peut différer) :

   ```powershell
   cargo run --release --example rustmusic_html -- --export x.png --size 1280x900 --scale 1.5 --renderer=software
   cargo run --release --example bootstrap_demo -- --export x.png --size 1280x1720 --scale 1.5
   ```

## Lancer

```powershell
cargo test --lib ui::tests                 # contrat du moteur
cargo test --lib ui::tests::overlays       # un thème
cargo test --lib platform::headless        # backend headless
cargo test --lib platform::windows         # suite Windows
cargo test --lib platform::linux           # suite Linux
cargo test --lib -- --include-ignored      # avec les tests GPU
cargo test --test css                      # contrat CSS
cargo test --test templates                # templates et diagnostics
cargo test --test frameworks               # Tailwind, Bootstrap, mode framework
cargo test --test frameworks bootstrap::demo   # un module
cargo test --test interaction              # menus, suggestions, aperçus
cargo test --examples                      # tests des exemples (rustmusic_html)
cargo test --workspace --all-targets       # tout
```

Au total : 292 tests unitaires du socle sous Windows (+ 2 GPU ignorés ; 279 sous
macOS, dont 26 du renderer Metal ; 263 sous Linux, dont 30 du backend), 53 du compilateur (`karbon-ui-macros`, + 4 outils de mesure ignorés), 146
d’intégration et 11 de l’exemple `rustmusic_html`.

## Critères de tri

On garde tout test de comportement indépendant de la plateforme, et une suite
Windows réduite à l’essentiel. On supprime les doublons (même comportement par le
même chemin), les tests figeant un correctif ponctuel déjà couvert ailleurs, les
mesures de temps ou de coût (`#[ignore]`) et les tests de débogage. Supprimer
retire une redondance, jamais une couverture.
