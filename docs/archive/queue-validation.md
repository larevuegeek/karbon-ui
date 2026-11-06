> Archive : cette page décrit les anciennes démonstrations retirées du projet. Les commandes et chemins de leur code sont historiques ; la bibliothèque actuelle conserve ses API et ses tests.

# File, drag/drop et mesures locales

Relevés du 2 novembre 2026, sans crate externe. Les quatre étapes comprennent
la recette de l’écran natif, la file, le glisser-déposer réutilisable et des
mesures de rendu, CPU et mémoire. La suite finale compte **204 tests** :
186 dans la bibliothèque, seize musicaux et deux dans la galerie.

## Recette visible

Deux processus release ont été lancés successivement, GPU puis WARP, avec les
six PNG locaux de `artifacts/artwork-fixtures-v1` et une file de douze titres.
La fenêtre initiale faisait 1 360 × 920 unités, soit 2 040 × 1 380 pixels avec
le DPI de 150 % de cette session.

| Scénario | Résultat observé |
| --- | --- |
| Images locales et composition | Affichées sous GPU et WARP, sans navigateur. |
| Déplacer le premier titre après le troisième | Ordre 2, 3, 1 confirmé dans les deux backends ; le morceau courant reste le même. |
| Bouton ↑ puis sélection et Écouter | Ordre et titre du lecteur mis à jour dans les deux backends. |
| Bibliothèque, End puis Entrée | Signal 10000 atteint, activé et ajouté à la file dans les deux backends. |
| Scroll central et scroll de file | Viewports indépendants et lecteur fixe, GPU et WARP. |
| Fenêtre compacte avec file ouverte | Vérifiée sur GPU à environ 1 003 × 700 unités : passage du panneau droit à une modale, puis fermeture par Échap. |
| Transparence puis retour opaque | Vérifiés visuellement sur GPU avec images et lecteur conservés. Les tests natifs GPU/WARP couvrent aussi alpha et DPI. |

La recette a révélé des symboles invisibles dans les boutons de 30 unités :
leur padding occupait toute la largeur. Une recette compacte partagée et un
test de largeur utile corrigent ce défaut. Le contrôle clavier a ensuite
révélé que Tab quitte la liste virtualisée sans entrer dans chaque ligne.
Les commandes « Monter », « Descendre », « Retirer » sont donc aussi placées
hors de la liste et testées par focus puis Espace, avec lecture conservée.

Les grandes sessions visibles précèdent cette dernière barre de commandes.
Les tests et mesures hors écran ci-dessous utilisent sa version finale.
Les annonces avec un lecteur d’écran restent à recetter ; le client UI Automation
peut encore annoncer un focus ancien, limite déjà suivie dans le socle.

Les essais automatiques couvrent en plus suppression du titre courant,
successeur/prédécesseur, doublons, file vide, borne de 10 000 entrées, formats
compacts/larges et sélection de la dernière entrée avec moins de 24 lignes
matérialisées pour le viewport testé. Les tests du drag couvrent seuil du clic,
dépôt, groupes incompatibles, cible désactivée, annulation, perte de focus,
reconstruction et auto-scroll sans layout complet.

## Mesure répétable sur la version finale

```powershell
cargo build --offline --release --bin rustmusic-demo --target-dir target/queue-verified
& './target/queue-verified/release/rustmusic-demo.exe' --benchmark --queue --renderer=gpu --artwork-dir artifacts/artwork-fixtures-v1
& './target/queue-verified/release/rustmusic-demo.exe' --benchmark --queue --renderer=software --artwork-dir artifacts/artwork-fixtures-v1
```

Commandes exécutées successivement. Protocole
`offscreen-prepare-render-readback-v1`, 1 360 × 920 pixels, échelle 1,
thème sombre, opaque, trois images de chauffe puis 1 024 rendus par phase.
Les tâches initiales sont terminées avant le chronométrage. La file est
ouverte ; la phase scroll déplace la page centrale. La scène initiale comporte
quatre commandes d’image, deux sources distinctes et 294 912 octets de pixels.

| Backend / phase | Médiane | p95 | Maximum | CPU, équivalent d’un cœur |
| --- | ---: | ---: | ---: | ---: |
| GPU / immobile repeint | 2,612 ms | 3,011 ms | 4,733 ms | 108,75 % |
| GPU / scroll | 2,616 ms | 3,275 ms | 3,666 ms | 115,06 % |
| WARP / immobile repeint | 3,006 ms | 3,700 ms | 11,135 ms | 239,73 % |
| WARP / scroll | 3,214 ms | 3,638 ms | 4,124 ms | 247,89 % |

Le CPU est la somme des temps de tous les threads divisée par le temps écoulé,
relative à un cœur. Ces boucles imposent des rendus continus ; elles ne
décrivent pas le CPU au repos. Le scroll donne 1 024 peintures, zéro vue
reconstruite et zéro layout complet, dans les deux backends.

| Après le scroll hors écran | Working set | Octets privés |
| --- | ---: | ---: |
| GPU | 66,26 Mio | 78,63 Mio |
| WARP | 45,41 Mio | 38,58 Mio |

La mesure comprend préparation, rendu et relecture CPU. Elle exclut surface
DIB, UpdateLayeredWindow, compositeur, latence d’entrée et VRAM. La sortie Vec
est conservée par ce protocole ; une fenêtre utilise le DIB direct.
Un seul passage par backend a été effectué, sans modèle CPU/GPU ni pilote
établi. La machine expose seize processeurs logiques. Ce relevé n’est ni une
garantie de temps d’image ni une comparaison Tauri.

Rapports : [GPU](../../artifacts/queue-render-gpu.json),
[WARP](../../artifacts/queue-render-warp.json),
[contexte, empreintes et relevés de repos](../../artifacts/queue-measurement-context.json).

## Fenêtre visible : relevés exploratoires

Le protocole `window-submit-direct-dib-v2` mesure jusqu’au retour de la
soumission Windows ; il ne mesure pas l’arrivée des pixels au moniteur.

| Relevé | Échantillons conservés | Médiane totale | p95 total | Échecs / récupérations |
| --- | ---: | ---: | ---: | ---: |
| GPU, fermeture après recette | 42 | 5,825 ms | 9,517 ms | 0 / 0 |
| WARP, dernier snapshot conservé | 157 | 5,974 ms | 7,732 ms | 0 / 0 |

**Ces deux lignes ne classent pas les backends.** La session GPU mélange deux
tailles, des interactions, un resize et un changement de transparence ; la
session WARP inclut davantage de peintures sans navigation. Elle a été
interrompue puis reprise et a reçu des interventions utilisateur. Son dernier
snapshot F8 est antérieur à la fin de la recette : les nouvelles demandes F8
ont été bloquées par une saisie concurrente. Sa fenêtre a été conservée ouverte.

Au repos observé entre deux snapshots :

| Backend | Intervalle | Temps CPU du processus | Peintures ajoutées |
| --- | ---: | ---: | ---: |
| GPU | 23,890 s | 515,625 ms | 0 |
| WARP, intervalle court retenu | 27,119 s | 406,250 ms | 2 |

Cela représente respectivement 2,16 % et 1,50 % d’un cœur sur ces intervalles.
Les snapshots et l’automatisation contribuent au coût. Un intervalle WARP plus
long a aussi produit 70 peintures : la session n’était pas un repos strict,
et le focus, le caret, les changements d’activation ou les interventions
utilisateur peuvent provoquer des mises à jour. On ne conclut pas à un repos
sans réveils à partir de ce relevé.

Le working set GPU passe de 92,79 à 93,73 Mio après environ deux minutes de
navigation et retour à l’accueil, soit +0,94 Mio. La file comporte alors une
occurrence supplémentaire et le titre courant a changé. Un relevé de processus
WARP après navigation indique environ 87,68 Mio résidents ; son état a aussi
été modifié par l’utilisateur. Ces observations ne démontrent ni fuite ni
absence de fuite. Les tests de 96 changements d’image avec resize vérifient
séparément la libération des anciennes références et du cache natif.

Les copies figées des journaux sont [GPU](../../artifacts/queue-visible-gpu.log)
et [WARP](../../artifacts/queue-visible-warp.log). Les journaux originaux peuvent
continuer d’évoluer si la fenêtre WARP ouverte reçoit F8. Les tailles/DPI et
les versions différentes interdisent une comparaison directe avec le tableau
hors écran. Aucune VRAM, période longue de plusieurs heures, composition DWM
ou interface Tauri équivalente n’a été mesurée.
