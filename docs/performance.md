# Mesurer le rendu du socle

L’API `karbon_ui::diagnostics` accepte une application fournie par l’appelant.
Elle ne dépend d’aucune démonstration ni d’un écran particulier.

```rust
use karbon_ui::{diagnostics::{self, BenchmarkOptions}, window::RenderMode};

let report = diagnostics::render_benchmark(
    BenchmarkOptions { renderer: RenderMode::Software, ..Default::default() },
    MyApplication::default(),
)?;
println!("{}", report.to_json());
```

Remplacer `MyApplication` par une implémentation d’`Application`.
`RenderMode::Gpu` impose le GPU ; `Software` utilise WARP. Hors Windows, le
même protocole mesure le rastériseur logiciel [headless](headless.md), sans
compteurs mémoire ni CPU (zéros) : ces chiffres ne se comparent pas à Direct2D.

## Périmètre

Le protocole `offscreen-prepare-render-readback-v1` mesure la préparation UI,
le rendu Direct2D et la relecture des pixels CPU. Il attend les tâches initiales
avec une limite, effectue une chauffe puis mesure la scène et le défilement.
Il exclut la soumission à une fenêtre visible, le compositeur du bureau,
la latence d’entrée et la VRAM.

Cinq phases : repeinture statique, défilement à la molette (crans de 24 unités au
40 % de la hauteur), reconstruction (la vue redemandée à chaque image, comme sous
une horloge qui change l'état toutes les 250 ms), survol (le pointeur passe d'une
ligne à l'autre, 60 unités plus bas, à chaque image) et animation. Chaque phase donne
aussi les percentiles du seul runtime (`prepare_*_ms` : événement, vue, styles, layout,
scène), hors rendu et relecture.

La phase `animation` avance une horloge simulée de 16,67 ms par image (réveil
`UiEvent::Timer`, puis image préparée à cet instant), comme une fenêtre cadencée par un
écran à 60 Hz : le temps de calcul ne décale pas l'échantillonnage. Avec
`BenchmarkOptions::click`, un appui et un relâchement à ce point ouvrent la phase et la
transition qu'ils lancent est mesurée (« Suivant » d'un carrousel…) ; sans lui, les
animations déjà en cours. `frame_requests` compte les images après lesquelles le runtime
a demandé la suivante : une animation qui tourne le garde égal au nombre d'images, un
écart montre des images sautées ou une animation figée. `view_rebuilds` et `layouts`
doivent rester à 1 (le clic) pour une transition de transformation ou d'opacité.

Le rapport contient les percentiles, le temps CPU du processus, la mémoire
résidente/privée et les compteurs de reconstruction, layout et peinture.
`BenchmarkOptions` règle dimensions, échelle, thème, transparence, nombre
d’images et présence du chrome. Ses limites sont validées avant le rendu.

Une boucle de benchmark force les images en continu ; elle ne mesure pas la
consommation au repos d’une application qui attend les événements.

## Exporter un aperçu

```rust
let image = diagnostics::render_preview(options, MyApplication::default())?;
diagnostics::write_png(&image, std::path::Path::new("preview.png"))?;
```

L’export utilise le runtime et le renderer natifs, sans ouvrir de fenêtre.

Pour une vraie fenêtre, utiliser le [profilage de présentation](presentation-profiling.md).

## Mesurer le démarrage (Windows)

La variable d’environnement `KARBON_STARTUP_TRACE` trace chaque étape de
l’ouverture sur la sortie d’erreur : temps écoulé depuis la **création du
processus** et mémoire privée du processus. Avec la valeur `exit`, la boucle de
messages s’arrête après le premier `Present`, pour enchaîner les lancements.

```powershell
$env:KARBON_STARTUP_TRACE = "exit"
.\target\release\examples\rustmusic_html.exe --size 1280x900
```

```text
karbon-startup:     13.3 ms     1.7 MiB  run
karbon-startup:     15.4 ms     1.8 MiB  GDI+ initialized
karbon-startup:     22.6 ms     3.0 MiB  fonts registered
karbon-startup:     54.9 ms    20.7 MiB  application loaded
karbon-startup:     95.4 ms    63.4 MiB  first view built, styled and laid out (predicted size)
karbon-startup:    166.9 ms    83.1 MiB  renderer ready (D3D11/D2D)
karbon-startup:    171.0 ms    83.2 MiB  window and presentation created
karbon-startup:    204.9 ms   142.3 MiB  first frame rendered
karbon-startup:    205.7 ms   142.5 MiB  first present
karbon-startup:    239.7 ms   143.4 MiB  window shown
```

Lancer l’exécutable directement : `Start-Process` retarde le démarrage effectif
du processus d’environ 300 ms après sa création, ce qui fausse la mesure.

Ordre d’ouverture d’une fenêtre : le périphérique D3D11 (`D3D11CreateDevice`,
~140 ms, le poste dominant) se crée sur un thread de travail pendant que le
thread UI enregistre les polices, charge l’application (décodage des images) et
prépare la première image à la taille prévue de la fenêtre (DirectWrite
suffit). La fenêtre et la composition sont créées ensuite ; la première
peinture ne refait pas la mise en page si la taille est celle prévue, et les
animations de cette image démarrent à sa présentation.

Relevé du 2026-11-03 (release, `--size 1280x900`, GPU, présentation
composition, médiane de 7 lancements alternés, du lancement au premier
`Present`) :

| Exemple | Avant | Après | Fenêtre montrée (avant → après) |
| --- | --- | --- | --- |
| `rustmusic_html` | 277 ms | 199 ms | 312 → 234 ms |
| `rustmusic` (natif, retiré depuis) | 277 ms | 185 ms | 310 → 218 ms |

Mémoire privée après 6 s (médiane de 3) : `rustmusic_html` 167 → 144 Mo,
`rustmusic` 95 → 96 Mo. L’écart restant vient surtout de la mémoire native
de la première image : envoi au GPU de 47 bitmaps (couvertures 420 × 420 et
ombres floutées rastérisées, ~19 Mo de pixels contre ~5 Mo), arrière-plan flouté
du hero et calques des clips arrondis ; le tas Rust ne diffère que de ~3 Mo.

Les [anciens relevés RustMusic](archive/performance.md) et la
[recette avec file de lecture](archive/queue-validation.md) sont conservés
avec leur contexte historique. Ils ne constituent pas une comparaison Tauri.

## Mise à jour et redimensionnement

Une mise à jour de l'état reconstruit la vue (`view()`), puis restyle et remet en
page tout l'arbre. Sur l'accueil de `rustmusic_html` (1280 × 900, release, harnais
headless, sans le rendu GPU) :

| Étape | Avant | Après |
| --- | --- | --- |
| Mise à jour (position du lecteur, curseur) | 38 ms | 7,4 ms |
| Redimensionnement d'un pas | 35 ms | 14,7 ms |

- `view()` ne coûte que ~1,4 ms ; le reste était surtout la rastérisation des ombres
  floutées (18 ms), refaite à chaque reconstruction : le cache d'ombre et la taille vue
  des conteneurs `@container` passent maintenant de l'ancienne vue à la nouvelle pour un
  même `ElementId` (plus de seconde passe styles + layout).
- Une ombre dont le flou dépasse 4 px se calcule sur une grille réduite (jusqu'à 1/8)
  puis s'interpole ; seule la découpe nette de l'élément reste en pleine résolution
  (écart ≤ 6/255 avec le calcul complet, testé).
- Reste : la remise en page complète (~6 ms) et, au redimensionnement, les ombres des
  éléments qui changent de taille. Une mise à jour ciblée (seuls les nœuds qui dépendent
  du champ modifié) est la suite logique.
- Depuis (2026-11-05) : mesures mémoïsées pendant une passe, mesures de texte gardées
  d'une image à l'autre et cascade sautée quand le pointeur garde les mêmes faits
  (mesures et répartition dans [le point de reprise](next-session.md#préparation-du-moteur-2026-11-05)).

### Glisser un curseur dans la fenêtre (Windows, GPU)

Mesuré avec `--profile` en envoyant à la fenêtre 150 déplacements de souris
(glisser du volume, 8 ms d'écart) :

| | Avant | Après |
| --- | --- | --- |
| Images présentées pendant le glisser | 15 | 239 |
| Layout d'une image | ~18 ms | 3,6 ms |
| Mesures de texte par image | 31 414 (19 ms) | 1 771 (0,4 ms) |

- Un déplacement du pointeur ne reconstruit plus la vue tout de suite : l'événement
  s'applique sur l'arbre de l'image précédente, puis reconstruction, styles et layout se
  font une fois par image. Windows ne produit `WM_PAINT` que lorsque la file est vide ;
  pendant un mouvement continu, la fenêtre peint dès que la dernière image date de plus
  de 8 ms.
- Pendant une passe de layout du runtime, la mesure d'une feuille pour une même place et
  la largeur min-content d'un élément sont calculées une seule fois (un layout flex
  imbriqué les redemandait à chaque niveau) ; la largeur des mots isolés (min-content des
  grilles) a son propre cache.
- Le moteur de texte DirectWrite garde les tailles mesurées à part des mises en page
  dessinées : le dessin purge les secondes à chaque image, ce qui faisait recréer toutes
  les mises en page de mesure.

### Redimensionner la fenêtre (Windows, GPU)

Mesuré avec `--profile` : 120 pas de `SetWindowPos` encadrés par
`WM_ENTERSIZEMOVE` / `WM_EXITSIZEMOVE`, comme un redimensionnement à la souris.

| Par pas | Avant | Après |
| --- | --- | --- |
| Traitement du changement de taille (`SetWindowPos`) | 58 ms | 15,7 ms |
| Préparation (vue, styles, layout, ombres) | 18,5 ms | 9 ms |
| Rendu GPU | 7 ms | 4,6 ms |

- Ombre de la fenêtre (fenêtre layered derrière la principale) : calculée sur les seules
  bandes du bord, en miroir (résultat identique, testé). Pendant un redimensionnement à
  la souris, elle est masquée quand la taille change (l'ombre native de DWM reste) et
  recalculée une fois à `WM_EXITSIZEMOVE` ; un simple déplacement la fait suivre.
- Ombres floutées des éléments ([`UiRuntime::set_live_resize`]) : pendant le geste, une
  couche floutée dont seule la taille change est étirée ; anneaux et bandes sont refaits.
  L'image exacte revient à la fin du geste.
- UI Automation : l'arbre accessible est publié au plus dix fois par seconde (et pas
  pendant le geste) ; un changement de focus part tout de suite. Le reconstruire à chaque
  image coûtait ~10 ms dès qu'un client UIA écoute.

### Défilement à la molette dans la fenêtre (Windows, GPU)

RustMusic, fenêtre 1920 × 1350 à 150 %, 30 crans vers le bas puis 30 vers le haut
(60 ms d'écart, postés à la fenêtre), `--profile` :

| | Avant | Après |
| --- | --- | --- |
| Accueil (cartes à pochettes) : reconstructions de la vue pendant le défilement | 55 (une par pochette arrivée) | 0 |
| Accueil : écart entre images (`intervals`, p50 / p95) | — | 16,7 / 24 ms |
| Morceaux (liste de 200 lignes, bibliothèque de 9 440 titres) : écart entre images (p50 / p95) | — | 14 / 36 ms |
| Morceaux : images en retard (`late_frames`) | — | 27 |

Attendre la composition (`DwmFlush`) avant chaque image a été essayé et rejeté : avec
12 ms de rendu, une composition sur deux était manquée (42 ms entre images).

#### Liste des morceaux (2026-11-08, suite)

Retour : sur Morceaux, bibliothèque chargée, défilement « le jour et la nuit » face à
Tauri. Banc hors écran du clone (`--bench`, GPU, 1920 × 1350 à 150 %, copie de la
bibliothèque) et fenêtre réelle (`--profile`, 150 crans postés) :

| | Avant | Après |
| --- | --- | --- |
| Banc, repeinture statique (p50, rendu et relecture compris) | 203 ms | 8,6 ms |
| Banc, molette (p50) | 76 ms | 10 ms |
| Commandes de scène par image | 12 336 (5 067 rognages, 704 calques d'opacité) | lignes visibles seulement |
| Fenêtre, rendu Direct2D d'une image | ~10 ms | ~3,5 ms |
| Mesures (`measure`) par mise en page au chargement d'une page de la liste | 188 000 | ~65 000 |
| Mise en page au chargement d'une page de la liste | 40 à 57 ms | 13 à 30 ms |

- **Sous-arbres hors de leur découpe** (`runtime/painting/cull.rs`) : un élément en
  `overflow: visible` hérite la découpe de son parent, si bien qu'une ligne sortie de
  la vue gardait celle de toute la liste et que le test « découpe vide » ne l'écartait
  pas : les 200 lignes étaient peintes à chaque image, un rognage par élément. Un
  enfant n'est peint que si l'encre de son sous-arbre (boîte, ombres, contour, taille
  naturelle d'un widget, plus une marge de 4 px) coupe sa découpe ; transformation ou
  contrôle natif : peint dans le doute. Le parcours s'arrête au premier élément qui
  touche la découpe.
- **Groupes d'opacité nulle** (`graphics/canvas.rs`) : les boutons révélés au survol
  (`opacity: 0`) ouvraient chacun un calque Direct2D. Le backend saute le groupe ; la
  scène le garde (opacité lisible, texte trouvé par `contains_text`).
- **Mémoire des mesures** (`layout/memo.rs`) : un widget conteneur (bouton, lien,
  élément cliquable, ~17 par ligne) empêchait de reprendre les mesures de tous ses
  ancêtres, d'où ~40 mesures par élément et par passe. Seuls les widgets qui relisent
  leur taille naturelle à la peinture (`Widget::paints_natural_size` : cases à cocher,
  interrupteurs, boutons radio) gardent cette précaution.
- **Mémoire des styles d'un sous-arbre mémorisé** (`styling/resolve/cascade.rs`) : son
  rang et celui de ses ancêtres n'entrent dans l'empreinte que si une règle du
  sous-arbre (ou une règle du parent pour ses enfants) lit une position. Une liste qui
  perd ses premières lignes, ou gagne un indicateur de chargement à côté, ne recalcule
  plus les styles des lignes restantes ni n'oublie leurs mesures.
- **Images en pourcentage d'une case fixée** (`runtime/images.rs`) : la pochette
  `w-full h-full` d'une case `w-10 h-10` ne dépend pas de l'image ; son arrivée ne
  relance plus la mise en page de la ligne, de la liste et de la page.
- Côté clone : les lignes de la table sont numérotées dans la liste entière (`Row::n`)
  et non dans la fenêtre chargée ; elles gardent leur clé de mémoire quand la fenêtre
  perd ses premières lignes.

#### Liste des morceaux (2026-11-08, fin)

Second retour : « tu ne vides pas la mémoire ou les dessins quand je scroll et que je
remonte ». Mesures en fenêtre réelle (600 crans postés à 40 ms, `--profile`, copie de la
bibliothèque) et échantillonneur de pile maison (thread de l'interface suspendu toutes
les 2 ms, piles résolues par dbghelp) :

| | Avant | Après |
| --- | --- | --- |
| Préparation d'une image (p50 / p95) | 1,9 / 4,2 ms | 0,5 / 0,8 ms |
| Image complète (p50 / p95) | 5,9 / 9,7 ms | 3,8 / 4,5 ms |
| Écart entre images (p50 / p95) | 15,2 / 22,2 ms | 16,7 / 22,0 ms (cadence de l'écran) |
| Processeur pendant le défilement (un cœur) | 58 % | 44 % |
| Reconstruction au chargement d'une page (chargeur, puis lignes) | ~50 + ~50 ms | ~15 + ~35 ms |

- **Accessibilité** (`accessibility/notify.rs`) : dès qu'un client UI Automation
  écoutait, chaque publication signalait le déplacement de chaque élément défilé
  (`BoundingRectangle`, des milliers d'événements inter-processus), cherchait l'ancien
  nœud par un parcours linéaire (n²) et créait un fournisseur pour chaque nœud : 38 %
  du thread de l'interface. Position signalée pour le seul élément focalisé (comme
  Chromium), anciens nœuds indexés, fournisseur demandé seulement pour un nœud qui
  change, snapshot indexé pour les requêtes des clients.
- **`content-visibility: auto`** (`layout/containment.rs`, voir
  [scroll.md](scroll.md#contenu-sauté-content-visibility)) : posé par l'original sur
  chaque ligne et vignette (`row-track`, `card-album`…), il manquait au moteur. Une
  ligne qui s'approche est disposée sur place (sa seule boîte), sans layout de l'arbre.
- **Pistes d'animation** (`animation/timeline`) : `Timeline::sync` parcourait à chaque
  image les pistes des 200 lignes (43 % de la préparation) ; il ignore le contenu sauté.
- **Ombres floutées partagées** (`component_shadow`) : la même ombre de pochette était
  rastérisée et envoyée au GPU pour chaque ligne, et refaite à son retour dans la vue ;
  une ombre de même géométrie est calculée une fois (64 entrées au plus).
- **Vérification de l'arbre** (`layout::validate`, ~3 ms sur 10 000 éléments) : une
  fois par reconstruction (sous-arbres repris compris) au lieu de trois, et après le
  layout seulement s'il y a une liste virtuelle.
- **Images de la scène Direct2D** : leur tri à chaque image parcourait toutes les
  commandes pour chaque image (n × m) ; un ensemble des images utilisées le remplace.
- Côté clone : la clé `memo` des lignes ne contient plus l'identité de la pochette en
  cache. Les pochettes arrivées pendant un défilement refaisaient près de 100 lignes au
  chargement suivant (vue, styles et layout), alors qu'une ligne reprise garde sa source
  d'image, déjà remplie.

Mémoire du clone (copie de la bibliothèque, `--page tracks`) : ~536 Mo privés au
démarrage, dont seulement ~92 Mo de tas Rust (compteur d'allocations temporaire). Le
reste vient surtout du cache de SQLite réglé par le backend, commun avec Tauri
(`cache_size = -64000` : 64 Mo par connexion du pool) : avec 2 Mo, le clone démarre à
265 Mo. Pendant un long défilement, le tas Rust monte de ~80 Mo : le cache des
pochettes (borné à 128 Mo, vignettes de 160 px) et les styles des nouveaux éléments.

Ensuite, comme Chromium, les styles du contenu sauté ne sont résolus qu'à son approche
(résolution partielle limitée à l'élément rendu, `UiRuntime::resolve_revealed`) : styles
de la reconstruction qui apporte les lignes ~7 → ~2 ms. L'ancien arbre d'une
reconstruction (à partir de 4 096 éléments) est libéré par tranches de 2 ms hors des
images qui reconstruisent (`UiRuntime::bury`). Défilement long de 30 s face à Tauri
(copie de la bibliothèque, même fenêtre) : pire écart 133 → 67 ms (Tauri 35 ms),
12 à-coups (Tauri 2), processeur 43 % (Tauri 64 %), RAM privée 661 Mo (Tauri 949 Mo,
sept processus).

Puis, échantillonnage du thread de l'interface hors préparation (trous de 40 à 88 ms
entre deux images bon marché) : l'accessibilité republiait l'arbre (~6 ms) dix fois par
seconde pendant le défilement et signalait `IsOffscreen` de chaque élément entré ou
sorti de la vue ; elle attend désormais l'arrêt du défilement. Côté clone,
`take_task_commands` (appelé après chaque message Windows) reconstruisait tout le
`ViewModel` pour redemander les pochettes ; il ne le fait plus qu'après une vue.
Défilement long de 30 s : pire écart 67 → 50 ms, 11 à 12 à-coups, processeur 35 %,
écart p95 entre images 21,6 → 17,8 ms. Construire la vue à une image et la brancher à la
suivante pendant un défilement a été essayé et retiré : à l'écran, l'image qui reporte
ne montre rien de neuf (21 à 24 à-coups au lieu de 12).

Reste au chargement d'une page de la liste : ~25 ms pour la reconstruction qui apporte
les 100 nouvelles lignes (création ~13-18 ms, essentiellement des allocations ; layout
~8 ms) et ~15 ms pour celle du chargeur. Piste suivante : `Styles` pèse 3 824 octets
par élément stylé (`ComputedStyle` 1 480, `Properties` 1 384), alloués et mis à zéro
pour chaque élément de chaque nouvelle ligne.

Construction de la vue (2026-11-09). Banc hors écran du clone, page Morceaux (200
lignes) reconstruite en entier à chaque image, et compteur d'allocations par taille :

| | Construction de la vue |
| --- | --- |
| Départ (tas de Windows) | 36,5 ms |
| `mimalloc` dans l'exécutable du clone | 32,0 ms |
| Listes d'enfants réservées d'avance (`Generator::capacity`) | 29,3 ms |
| Un seul ancien arbre en attente (`rebuild_view`) | 24,4 ms |

- **Allocateur** : `mimalloc` (v2 ou v3, même résultat) gagne ~12 % ; le coût venait
  moins de l'allocateur que du volume de mémoire neuve touchée.
- **Listes d'enfants** : elles partaient de `Vec::new()` et doublaient (4, 8, 16
  éléments de 504 octets recopiés) : 14 Go alloués sur le banc, le premier poste. Le
  compilateur de gabarits réserve la borne haute du nombre d'enfants, et chaque
  `{#each}` la taille de son itérable.
- **Cimetière borné** : quand chaque image reconstruit (saisie, banc), aucune image ne
  libérait les anciens arbres ; la mémoire grossissait sans fin et l'allocateur
  demandait sans cesse des pages neuves au système (42 % de la construction dans sa
  recherche de pages). Un arbre en attente au plus : le précédent est libéré quand un
  autre arrive.
- Essayé et retiré : fonctions de classes `#[inline(always)]` (−4 %, mais exécutable de
  81 à 123 Mo et LTO de 11 à 17 min) ; **contenu différé** des éléments
  `content-visibility: auto` éloignés (la vue ne construit que les lignes proches et les
  autres à leur approche, sans jamais montrer de ligne vide) : le grand pic disparaît
  (pire préparation 23 → 14 ms), mais chaque construction à l'approche coûte une
  reconstruction complète (~8 ms de frais fixes, surtout les parties de la vue non
  mémorisées et les parcours de tout l'arbre : `validate`, `collect_ghosts`) ; défilement
  long de 30 s : 27 à 55 à-coups selon la largeur des zones, contre 10-11 sans.

En fenêtre (défilement long de 30 s, copie de la bibliothèque, build LTO), cette première
série ne se voyait pas encore : 12 à 17 à-coups, pire écart ~50 ms. Une image de
chargement de page restait au-dessus du budget de 16,7 ms.

Coût fixe d'une reconstruction (2026-11-09, suite). Phase `rebuild` du banc, où rien ne
change d'une image à l'autre, et journal des images lentes (`--profile`) en fenêtre :

| | Préparation de la phase `rebuild` (vue + styles + layout) |
| --- | --- |
| Avant | 10,3 ms (5,7 + 1,8 + 2,8) |
| Après | 3,9 ms (1,2 + 1,2 + 1,5) |

- **Sous-arbres repris non reparcourus.** La vérification de l'arbre (`layout::validate`)
  était le premier poste (28 % de la préparation). Un sous-arbre mémorisé retient le
  résumé de sa dernière vérification (`CheckedSubtree` : nombre d'éléments, hauteur,
  présence d'un `fixed`, d'un élément refusé sous une transformation) : seul ce qui
  dépend de ses nouveaux ancêtres est encore regardé. Les identifiants en double sont
  vérifiés par `Interaction::index_tree`, qui voit de toute façon tout l'arbre. Le même
  résumé dispense de descendre les parcours qui cherchent une seule sorte d'élément
  (champs de saisie, radios, couches). `previous_keys` s'arrête aux sous-arbres sans
  mémoire imbriquée, `memo::flags` à ceux dont le layout est repris.
- **Zones stables du clone mémorisées** (`region_keys.rs` du clone : barre latérale,
  lecteur, recherche, actions de la barre de titre) : −1,1 ms.
- **Le travail lourd avant l'attente de l'écran.** Une image d'animation attend sa place
  dans la file de présentation avant de lire l'horloge (`FrameSlot`). Quand un lot de
  données arrivait pendant un défilement, la reconstruction venait après cette attente :
  12 ms inactives, puis 11 ms de travail et 5 ms de dessin, affichage manqué.
  `runtime::prepare_pending_rebuild` la fait avant, à la place de l'attente.

Fenêtre, build LTO, 5 essais de 30 s : 5 à 8 à-coups, pire écart 34 ms (avant : 12 à 17
et ~50 ms ; Tauri : 2 et 35 ms). Restait l'image qui reçoit les 100 lignes d'un lot
(~25 ms, ~130 µs par ligne), au-delà du budget d'une image.

Étalement (2026-11-09, fin). `Application::has_frame_work` et `frame_work` : l'application
annonce un travail découpé, le runtime lui donne une tranche par image (réveil par
`next_wake`, donc sur tous les systèmes). Le clone fait entrer un lot de 100 lignes par
tranches de 10 (`TRANCHE`, `pages/tracks/mod.rs`).

- **Étaler ne suffit pas.** Matrice en alternance sur le même binaire (4 essais de 30 s
  chacun, médiane des à-coups) : lot entier 5,5, tranches de 10 : 3,5, de 20 : 5,5, de
  40 : 5. Le gain de la seule découpe est dans le bruit de la mesure.
- **Une tranche par image.** Le premier essai en faisait deux : une avant l'attente de
  l'écran, une après (l'attente dépassait l'écart minimal entre deux tranches). Sous
  Windows, `FRAME_WAKE` fait la tranche (`begin_frame_work`) et le réveil qui suit ne
  compte pas.
- **Ce qui a débloqué la mesure : les reconstructions demandées pendant l'attente de
  l'écran** (message « charge la suite », arrivée du lot). Elles se faisaient après
  l'attente, dans le reste de l'image, et lui faisaient manquer son affichage.
  `hold_rebuild` les reporte à l'image suivante pendant un défilement (20 ms au plus) ;
  elles passent alors avant l'attente, et l'image courante montre la nouvelle position.

Fenêtre, build LTO, 8 essais au calme : **0 à 4 à-coups (4, 0, 2, 0, 0, 2, 1, 1), pire
écart 20 à 33 ms** (Tauri : 2 et 35 ms), 60 images par seconde. Un essai lancé juste après
une compilation (processeur déjà occupé) donne 12 à-coups et 50 ms : la marge reste
faible sur une machine chargée. Le clone étale aussi les grilles Albums et Artistes (12
cartes par image). Le coût par élément (`Styles` de 3,8 Ko, `Node` de 536 octets passé
par valeur) reste l'amélioration de fond.

- **Images qui arrivent plus tard** (`ImageSlot`, `runtime/images.rs`) : une pochette
  décodée en tâche de fond remplit sa source ; le runtime la repeint sans reconstruire la
  vue, et ne refait la mise en page que si l'élément n'a pas sa largeur et sa hauteur
  fixées et a mesuré une autre taille (ses conteneurs oublient alors leur placement
  retenu). Avant, chaque pochette qui arrivait pendant le défilement reconstruisait toute
  la vue (vue, styles et mise en page : 6 à 13 ms), presque à chaque image.
- **Cadence des images** (`graphics/swapchain.rs`, `FrameSlot`) : la chaîne d'échange a
  une latence d'une image (objet attendable de DXGI) et une image d'animation attend sa
  place dans la file **avant** de lire l'horloge, comme le `BeginFrame` de Chromium.
  Avant, `Present` rendait la main tout de suite au départ d'un glissement : deux images
  lues à quelques millisecondes d'écart s'affichaient à une image d'écart (à-coup à
  chaque cran), et la file ajoutait jusqu'à trois images de latence.
- Phase de banc `wheel` (crans de défilement doux sous un pointeur posé, à l'heure
  réelle) et lignes `Slow frame` de `--profile`, voir
  [le profilage d'une fenêtre](presentation-profiling.md).

### Animations de taille (barre latérale repliable)

Repli/dépli de la barre (`transition: width 200ms`), Windows, GPU :

- Cadence : un réveil d'animation proche (≤ 17 ms) n'arme plus `SetTimer` (granularité
  ~15,6 ms, minimum 10 ms), mais poste `FRAME_WAKE` juste après chaque présentation ;
  `Present(1)` cale alors les images sur l'écran, comme `requestAnimationFrame`. Le message
  n'est posté qu'après une présentation : posté plus tôt, il passerait avant `WM_PAINT`
  à chaque fois et affamerait l'affichage.
- Une préparation par image : le réveil fait seulement avancer l'horloge, la vue est
  préparée au dessin (comme pour les déplacements du pointeur).
- Pendant une transition de taille ou un redimensionnement : ombres floutées étirées et
  requêtes `@container` appliquées avec une image de retard (une passe de layout au lieu
  de deux) ; tout redevient exact à la première image stable.

Avant : ~4 images par repli de 200 ms. Après : 8 à 15 selon la charge de la machine
(mesures faites pendant une compilation en parallèle), ~13 ms par image.


## Banc « éditeur » : l'interface autour d'une vue 3D vivante (2026-11-21)

Exemple `examples/editor_bench/` (exigence KUI-001 du cahier des charges de l'éditeur de
karbon-engine) : une interface d'éditeur chargée, faite des composants existants (barre
de 12 outils, arbre de 100 000 objets en liste virtualisée, frise de 120 clés, 200
propriétés — 140 champs de texte, 20 cases, 40 curseurs — dans un panneau qui défile,
100 vignettes de 96 × 64 pixels, ligne d'état), autour d'une vue 3D factice : un fil
livre à un `ImageSlot` 60 images par seconde de 1740 × 1176 pixels (la taille de la vue
de l'éditeur), différentes à chaque fois. La ligne d'état change toutes les 500 ms (une
reconstruction de la vue), comme dans l'éditeur. C'est le **mode copie** : chaque image
passe par la mémoire de l'ordinateur et Direct2D en fait un bitmap neuf.

```powershell
cargo build --release --example editor_bench
./target/release/examples/editor_bench.exe --seconds 20 --feed live    # image neuve à chaque fois
./target/release/examples/editor_bench.exe --seconds 20 --feed still   # même image relivrée
```

Le banc active `--profile`, se ferme seul au bout de la durée et écrit trois lignes : le
rapport `Presentation : {…}` habituel, une ligne `UI thread : {…}` et son résumé.

**Compteur ajouté à `--profile`** (ligne `UI thread`, protocole `window-ui-thread-v1`) :
`work` est le travail du fil de l'interface par image, `total` moins l'attente de la
place dans la file de présentation (`queue_wait`, la cadence de l'écran) ; mêmes images
que les étapes du rapport (les 1 024 dernières). Le rapport `window-present-v4` ne change
pas.

`--feed still` relivre la même image : la fenêtre repeint autant de fois, mais Direct2D
garde son bitmap (sa clé est l'adresse de l'image). L'écart avec `live` est donc le prix
du bitmap neuf (création et envoi de 8 Mo) sur le fil de l'interface.

Relevés : Windows 11, fenêtre 1600 × 1000 unités à 150 % (2400 × 1500 pixels), écran
3840 × 2160 à 60 Hz branché sur la RTX 4080 (pilote 32.0.16.2114), composition, build
release, 20 s par tour, 1 190 peintures, statistiques sur les 1 024 dernières. Trois tours
de chaque, alternés. **karbon-ui a dessiné sur la RTX 4080** : il crée son périphérique
Direct3D sans désigner de carte, donc sur la première que DXGI énumère, que le banc nomme
(« NVIDIA GeForce RTX 4080 ») ; aucune préférence de carte n'est réglée pour l'exécutable.

| Tour | `work` p50 / p95 / max | `prepare` p50 / p95 | `render` p50 / p95 | `total` p50 | `queue_wait` p50 | CPU (un cœur) |
| --- | --- | --- | --- | --- | --- | --- |
| live 1 | 2,99 / 4,33 / 6,8 ms | 0,61 / 1,11 | 2,29 / 2,97 | 12,74 | 9,73 | 36,0 % |
| live 2 | 2,95 / 3,64 / 6,2 ms | 0,60 / 0,96 | 2,28 / 2,70 | 16,55 | 13,63 | 32,7 % |
| live 3 | 2,92 / 3,53 / 5,3 ms | 0,59 / 0,95 | 2,26 / 2,56 | 15,24 | 12,31 | 33,4 % |
| still 1 | 2,07 / 2,77 / 5,0 ms | 0,60 / 1,08 | 1,34 / 1,80 | 8,67 | 6,52 | 16,8 % |
| still 2 | 2,10 / 2,88 / 4,6 ms | 0,61 / 1,10 | 1,34 / 1,81 | 7,28 | 5,13 | 18,7 % |
| still 3 | 2,11 / 3,06 / 5,1 ms | 0,61 / 1,12 | 1,34 / 1,89 | 9,33 | 7,16 | 18,6 % |
| live, WARP | 4,29 / 6,08 / 10,3 ms | 0,71 / 1,24 | 3,45 / 4,72 | 7,48 | 3,14 | 130,8 % |
| still, WARP | 3,28 / 4,52 / 6,3 ms | 0,66 / 1,11 | 2,52 / 3,30 | 13,58 | 10,28 | 117,0 % |

Lecture :

- **Mode copie sur la RTX : environ 3 ms de travail du fil de l'interface par image 3D au
  médian, 3,5 à 4,3 ms au 95ᵉ centile**, pour une image livrée à 60 par seconde (59,5 à
  60 images reçues par seconde, intervalles entre présentations de 16,7 ms au médian).
  C'est 18 % d'une image à 60 Hz.
- Le bitmap neuf coûte **0,9 ms par image** au médian (2,9 contre 2,1 ms), tout entier dans
  `render` (2,27 contre 1,34 ms), et double le CPU du processus (33 à 36 % d'un cœur contre
  17 à 19 %) ; 20 Mo de mémoire en plus. Le reste, environ 2,1 ms, est le redessin de toute
  la fenêtre : la préparation (0,6 ms, vue non reconstruite) et le rendu Direct2D de toute
  la scène.
- **Le fil de l'interface reste bloqué dans la peinture** jusqu'à la place suivante dans la
  file de présentation : `total` vaut 7 à 17 ms selon le tour, dont 5 à 14 ms d'attente
  (`queue_wait`), parce que le réveil par la vue 3D tombe à une phase quelconque de la
  cadence de l'écran. Pendant ce temps, l'interface ne traite pas les entrées. Ce n'est pas
  du travail, mais c'est du temps de réponse ; à creuser avec KUI-003 et KUI-005.
- Le fil de la vue passe 1,8 ms par image à fabriquer l'image et à la faire vérifier par
  `Image::new` (hors du fil de l'interface).
- En rendu logiciel (WARP), 4,3 ms au médian, 6,1 ms au 95ᵉ centile, 1,3 cœur occupé.

**Non mesuré : le Radeon intégré.** karbon-ui ne sait pas encore choisir sa carte (c'est
KUI-004) et l'écran est branché sur la RTX ; le faire dessiner sur le Radeon demanderait de
changer la préférence de carte de Windows pour l'exécutable, un réglage de la machine qui
n'a pas été touché. Ni la mise en page ni le défilement des panneaux ne sont mesurés : rien
n'est cliqué ni tapé pendant le banc.

### Mode copie allégé (KUI-002, 0.3.23)

`Image::opaque_bgra` (alpha forcé à 255, sans vérifier chaque pixel) et `LiveImage` (le
bitmap Direct2D de l'image précédente d'une source est repris et mis à jour par
`CopyFromMemory` quand la taille ne change pas). Le banc s'en sert avec `--image live` ;
`--image checked` garde l'ancien chemin (`Image::new` dans un `ImageSlot`, un bitmap neuf
par image).

```powershell
./target/release/examples/editor_bench.exe --seconds 15 --feed live --image checked
./target/release/examples/editor_bench.exe --seconds 15 --feed live --image live
```

Mêmes conditions que plus haut (RTX 4080, 1600 × 1000 unités à 150 %, composition,
release), 15 s par tour, trois tours de chaque, alternés, 887 à 891 images par tour :

| Tour | `work` p50 / p95 | `render` p50 / p95 | CPU (un cœur) | fil de la vue |
| --- | --- | --- | --- | --- |
| checked 1 | 3,29 / 4,74 ms | 2,47 / 3,34 | 40,0 % | 2,35 ms |
| live 1 | 2,83 / 4,39 ms | 2,03 / 2,92 | 34,4 % | 2,01 ms |
| checked 2 | 2,77 / 3,77 ms | 2,11 / 2,69 | 35,7 % | 2,10 ms |
| live 2 | 2,46 / 3,61 ms | 1,79 / 2,36 | 32,4 % | 1,84 ms |
| checked 3 | 2,66 / 3,46 ms | 2,00 / 2,42 | 34,5 % | 2,12 ms |
| live 3 | 2,54 / 3,92 ms | 1,83 / 2,73 | 30,4 % | 1,83 ms |

Lecture : **−0,3 ms de travail du fil de l'interface par image 3D au médian** (2,61 contre
2,91 ms en moyenne des tours, −10 %), tout entier dans `render` (1,88 contre 2,19 ms) ;
−0,3 ms sur le fil de la vue (`opaque_bgra` au lieu de `Image::new`) ; 4 points de CPU en
moins. **Le gain est le tiers de ce que coûtait le bitmap neuf** (0,9 ms, écart entre
`--feed live` et `--feed still` plus haut) : la création du bitmap disparaît, l'envoi des
8 Mo à la carte reste (`CopyFromMemory`). L'exigence (« baisse d'au moins le coût mesuré
de `CreateBitmap` ») est tenue si l'on compte la seule création ; pour aller plus loin,
il faut le partage sur la carte graphique (KUI-003). Le Radeon n'est toujours pas mesuré
(KUI-004).

### Arbre de 100 000 lignes qui défile (KUI-029, 0.3.23)

```powershell
./target/release/examples/editor_components.exe --bench 15
```

L'exemple `editor_components` déplie tout son arbre (100 000 objets sur 8 niveaux) et,
60 fois par seconde, descend le curseur de trois lignes : la vue est reconstruite et la
liste défile jusqu'au curseur à chaque image (plus dur qu'un défilement à la molette, qui
ne reconstruit rien). RTX 4080, fenêtre 1280 × 800 unités à 150 %, 15 s : **890 images,
aucune en retard**, intervalles de 16,9 ms au médian et 17,6 ms au 95ᵉ centile ; travail
du fil de l'interface 1,44 ms au médian, 1,96 ms au 95ᵉ centile ; 11 % d'un cœur.

Déplier un nœud de 10 000 enfants dans un arbre de 87 000 lignes visibles :
**1,7 ms** au médian (`TreeState::set_expanded`, release ;
`cargo test --release --lib tree_timings -- --ignored --nocapture`). Au plus 60 lignes
construites en défilant (test `a_hundred_thousand_nodes_build_only_the_visible_rows`).
