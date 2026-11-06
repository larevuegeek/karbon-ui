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

