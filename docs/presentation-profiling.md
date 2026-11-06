# Profiler une fenêtre

Activer `WindowOptions::profile` lors de l’ouverture de sa fenêtre :

```rust
let options = WindowOptions { profile: true, ..WindowOptions::DEFAULT };
platform::run(options, MyApplication::default())?;
```

Une application qui utilise `WindowOptions::from_args` peut aussi accepter
`--profile`, et choisir le chemin de présentation avec
`--present=auto|composition|layered`. Les exemples `simple_ui` et `rustmusic_html`
le font :

```powershell
cargo run --offline --release --example simple_ui -- --profile --renderer=gpu
cargo run --offline --release --example rustmusic_html -- --profile --renderer=gpu --present=layered
```

Les mesures sont bornées et séparées par fenêtre. Un rapport est produit à la
fermeture ; F8 permet de consulter les compteurs pendant la session.
Il détaille préparation, rendu, surface et soumission Windows, les erreurs,
les recréations du renderer et les changements de taille.

## Protocole `window-present-v3`

Le champ `presentation` indique le chemin mesuré (`composition` ou `layered`,
voir [l’architecture](architecture.md#présentation-à-lécran-windows)).

| Étape | Composition | Layered |
| --- | --- | --- |
| `prepare` | Géométrie/DPI et préparation UI faite pendant cette peinture. | idem |
| `render` | Dessin Direct2D dans le back buffer jusqu’à `EndDraw`, sans relecture. | Dessin, copie GPU → texture de relecture, `Map` et copie dans le DIB. |
| `surface` | Création, `ResizeBuffers` ou recréation (mode alpha) de la chaîne d’échange. | Vérification ou création du DIB persistant. |
| `submit` | `Present(1, 0)` et, si la chaîne a changé, `SetContent` + `Commit`. | `UpdateLayeredWindow`. |
| `total` | Début de préparation jusqu’après la soumission. | idem |

`surface_creations` compte aussi, en composition, la chaîne créée à l’ouverture
pour valider DirectComposition et chaque `ResizeBuffers`.

Attention à la comparaison : la relecture layered attend la fin du travail GPU
(`Map`), alors que `Present` rend la main dès que l’image est en file. En
composition, `render` peut contenir l’attente d’un back buffer libre ; le temps
GPU et la composition DWM jusqu’au moniteur ne sont pas mesurés. Le CPU du
processus est donc la mesure la plus comparable entre les deux chemins. Les
compteurs mémoire et CPU concernent le processus, pas exclusivement la fenêtre ;
le travail de DWM n’y figure pas.

`cpu_ms` provient de `GetProcessTimes`, comptabilisé par échantillonnage au tick
d’horloge (~15,6 ms). Des peintures courtes, réveillées juste après un tick,
peuvent ne jamais être comptées : on a relevé 0 ms pour une session qui
consommait en réalité 2,6 milliards de cycles. Pour comparer des chemins rapides,
utiliser `QueryProcessCycleTime` depuis un outil externe (méthode ci-dessous).

v3 remplace `window-submit-direct-dib-v2` : l’étape `render_readback` s’appelle
désormais `render` et le champ `presentation` est ajouté.

## Avant / après la composition (machine de développement)

Recette : `rustmusic` (ancien exemple natif, retiré depuis) en release, `--profile`, fenêtre 1440 × 1040 à 150 %
(2160 × 1560 px physiques), écran 3840 × 2160. Un script externe attend 2 s,
puis invalide la fenêtre toutes les ~16 ms pendant 12 s (environ 33 peintures/s,
limitées par la résolution de `Start-Sleep`), puis envoie `WM_CLOSE`. Le CPU du
processus est relevé sur cette fenêtre de 12 s avec `QueryProcessCycleTime`
(cycles ÷ fréquence nominale de 4 691 MHz, en % d’un cœur) ; les étapes et la
mémoire viennent du rapport de fermeture (environ 483 peintures). Médianes de
trois sessions alternées par configuration. « Avant » = binaire du commit
précédent ; « layered » = nouveau binaire avec `--present=layered`.

| Renderer | Chemin | total p50 / p95 | render p50 | submit p50 | CPU processus | Working set |
| --- | --- | --- | --- | --- | --- | --- |
| GPU | avant (layered) | 7,43 / 13,03 ms | 6,45 ms | 0,88 ms | 33,6 % | 143 Mo |
| GPU | **composition** | **2,40 / 3,70 ms** | 2,32 ms | 0,05 ms | **13,3 %** | **104 Mo** |
| GPU | layered (nouveau binaire) | 8,10 / 13,49 ms | 7,17 ms | 0,82 ms | 37,7 % | 144 Mo |
| WARP | avant (layered) | 7,82 / 13,86 ms | 6,73 ms | 1,04 ms | 90,6 % | 133 Mo |
| WARP | **composition** | **3,28 / 5,10 ms** | 3,15 ms | 0,08 ms | **83,5 %** | **114 Mo** |
| WARP | layered (nouveau binaire) | 7,36 / 9,91 ms | 6,38 ms | 0,91 ms | 85,0 % | 133 Mo |

Lecture : sur GPU, la composition divise par ~3 le temps jusqu’au retour de
`Present` et le CPU du processus, et retire ~40 Mo de working set (texture de
relecture et DIB de 13 Mo chacun, plus leurs copies). Sous WARP, le rendu
reste sur le CPU : le temps de peinture est divisé par ~2,4 mais le CPU baisse
peu (~5 à 8 %). Le CPU système (tous cœurs, DWM compris) a aussi été relevé
mais restait dominé par d’autres processus de la machine (27 à 59 %) : aucun
effet sur DWM n’est établi. Le chemin layered du nouveau binaire reste au niveau
de l’ancien. `Present(0, 0)` sous WARP a été essayé : aucun gain en cycles.

## Deux sessions, GPU puis WARP

L’ancien utilitaire `profile_session` enchaînait une session GPU et une session WARP
d’un exécutable et gardait leurs sorties. Il a été retiré : `--profile` et
`--renderer=` suffisent, puisque le rapport est écrit sur la sortie standard à la
fermeture de la fenêtre. Fermer la première fenêtre ouvre la seconde :

```powershell
cargo build --offline --release --example rustmusic_html
foreach ($renderer in 'gpu', 'software') {
    & ./target/release/examples/rustmusic_html.exe --profile --renderer=$renderer *> "artifacts/profile-session-$renderer.log"
}
```

Ajouter les arguments communs à comparer (par exemple `--present=layered` ou
`--size 1280x900`) après `--profile`. Seule perte : l’arrêt automatique de la
recette quand la première session échoue (vérifier `$LASTEXITCODE`).

Les anciennes séries sont conservées dans les
[archives de profilage](archive/presentation-profiling.md) avec leurs limites.
