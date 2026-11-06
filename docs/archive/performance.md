> Archive : cette page décrit les anciennes démonstrations retirées du projet. Les commandes et chemins de leur code sont historiques ; la bibliothèque actuelle conserve ses API et ses tests.

# Premier relevé de rendu et de mémoire

Le [relevé avec file de lecture](queue-validation.md) complète cette première
mesure : version finale hors écran, sessions visibles, CPU au repos et RAM
après navigation, avec leurs limites de comparaison.

Le protocole `offscreen-prepare-render-readback-v1` mesure la préparation UI,
le rendu Direct2D et la copie de la texture vers des pixels CPU. Il ne crée pas
de fenêtre visible et exclut `UpdateLayeredWindow`, le compositeur du bureau,
la latence d’entrée et la VRAM. Il ne constitue pas une comparaison avec Tauri.

## Reproduire

```powershell
cargo build --offline --release --bin rustmusic-demo --target-dir target/artwork-check
& './target/artwork-check/release/rustmusic-demo.exe' --benchmark --renderer=gpu --artwork-dir artifacts/artwork-fixtures-v1
& './target/artwork-check/release/rustmusic-demo.exe' --benchmark --renderer=software --artwork-dir artifacts/artwork-fixtures-v1
```

Exécuter les deux commandes successivement. Chaque processus écrit son rapport
JSON sur la sortie standard. `--renderer=gpu` impose le matériel sans repli ;
`software` utilise WARP. Le jeu local contient six copies distinctes du PNG
fourni dans `src/assets`, de 256 × 256 pixels et 16 972 octets chacune.
Ce jeu simple ne représente pas la diversité d’une bibliothèque réelle.

Paramètres par défaut : 1 360 × 920 unités, échelle 1, thème sombre, fond opaque,
attente des tâches initiales puis trois rendus de chauffe. Chaque phase force
1 024 rendus sans cadence : une scène immobile, puis un scroll de ±24 unités
dont le sens change toutes les huit images. Les ressources restent chargées
entre les deux phases. L’API `BenchmarkOptions` permet d’autres dimensions,
échelles et nombres d’images, dans les limites validées par le moteur.

Le chronométrage d’une image comprend l’événement de scroll éventuel, `prepare`,
le rendu, la relecture CPU et la libération du buffer retourné. Les percentiles
utilisent le rang supérieur dans les échantillons triés. Les compteurs UI
distinguent reconstruction de vue, layout et peinture de scène.

## Résultat local du 2 novembre 2026

Build release, Windows 10.0.26300.0, Rust 1.99.1, 16 processeurs logiques.
L’inventaire WMI était inaccessible : modèle CPU, modèle GPU et pilote ne sont
pas établis. Il s’agit d’une exécution par backend, sans série statistique.

| Backend / scénario | Médiane | p95 | Maximum | CPU, équivalent d’un cœur |
| --- | ---: | ---: | ---: | ---: |
| GPU / immobile repeint | 1,827 ms | 2,191 ms | 3,178 ms | 112,28 % |
| GPU / scroll | 1,776 ms | 2,505 ms | 3,206 ms | 115,71 % |
| WARP / immobile repeint | 2,203 ms | 2,514 ms | 3,196 ms | 222,88 % |
| WARP / scroll | 2,122 ms | 2,509 ms | 3,922 ms | 194,85 % |

Le pourcentage CPU est le temps CPU utilisateur + noyau de tous les threads,
divisé par le temps écoulé de la phase. 200 % signifie environ deux cœurs
occupés pendant cette boucle. Ce n’est pas la valeur normalisée sur les
16 processeurs du Gestionnaire des tâches, ni la consommation au repos.
Le moteur normal attend les événements ; ce benchmark impose des rendus continus.

| Après le scroll | Working set | Octets privés | Pic du working set du processus |
| --- | ---: | ---: | ---: |
| GPU | 63,75 Mio | 77,41 Mio | 68,61 Mio |
| WARP | 44,25 Mio | 36,36 Mio | 49,53 Mio |

Ces instantanés viennent de `GetProcessMemoryInfo`. Le working set décrit les
pages résidentes ; les octets privés décrivent la mémoire privée engagée.
Ils ne mesurent pas le seul cache d’images et n’incluent pas toute la VRAM.
Le pic est celui du processus depuis son lancement, pas celui de la seule phase.

Les deux scènes contiennent six commandes d’image et quatre images distinctes,
soit 589 824 octets de buffers CPU référencés par la scène. Le scroll produit
1 024 peintures, **zéro reconstruction de vue et zéro layout** dans ce scénario.
La phase immobile réutilise la scène sans peinture UI, tout en forçant le rendu.

Les 246,396 ms GPU et 154,342 ms WARP d’initialisation comprennent création des
services, chargement des pochettes et chauffe. Ce ne sont pas des mesures de
démarrage d’une fenêtre interactive. Les bibliothèques et fichiers peuvent déjà
être dans les caches système.

Rapports bruts : [GPU](../../artifacts/render-gpu.json),
[WARP](../../artifacts/render-cpu.json), [contexte et empreintes](../../artifacts/benchmark-context.json).
Le [rapport mixte](../../artifacts/render-mixed-input.json) valide seulement la
coexistence d’un fichier invalide et d’un PNG valide ; il utilise un ancien
échantillon de 48 images et ne fait pas partie de la comparaison ci-dessus.

## Suite de la mesure

Le [profilage de fenêtre](presentation-profiling.md) est maintenant disponible
avec `--profile`, jusqu’au retour d’UpdateLayeredWindow. Les chronométrages DWM
et la latence jusqu’au moniteur restent hors de ce protocole.
Le chemin de fenêtre écrit maintenant directement dans le DIB persistant
(`window-submit-direct-dib-v2`), sans buffer Vec intermédiaire. Les relevés hors
écran ci-dessus conservent leur protocole et ne mesurent pas ce changement.

Recetter les fichiers réels dans une fenêtre observable, puis mesurer la
présentation Windows complète, le repos, le resize et une session prolongée de
navigation. Varier dimensions et formats des images, DPI et taille de bibliothèque.
Répéter les séries avec matériel et pilote identifiés avant toute généralisation.
Une comparaison Tauri demandera le même écran, les mêmes ressources, la même
qualité visuelle et des scénarios identiques. La relecture CPU de la texture
reste une piste d’optimisation à étudier après ces mesures.
