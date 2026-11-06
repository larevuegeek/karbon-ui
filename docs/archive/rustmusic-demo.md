> Archive : cette page décrit les anciennes démonstrations retirées du projet. Les commandes et chemins de leur code sont historiques ; la bibliothèque actuelle conserve ses API et ses tests.

# Premier écran RustMusic natif

Cet exécutable valide le socle Karbon UI sur une interface musicale composée.
Il utilise des données fictives et ne lit ni fichiers musicaux, ni configuration
de votre application RustMusic existante. Aucun navigateur ni crate externe.

## Lancer

```powershell
cargo run --offline --bin rustmusic-demo --target-dir target/music-check
```

`cargo run --offline` continue de lancer la galerie de composants actuelle.
`default-run` dans Cargo.toml conserve ce comportement malgré le deuxième binaire.

Les options communes `--renderer=auto|gpu|software`, `--light` et `--transparent`
peuvent être ajoutées après `--`. F2 change le renderer, F5 le thème et F8
affiche les compteurs du runtime dans le terminal. La modale Diagnostics
indique aussi le backend effectif et les compteurs du cache CPU.

`--artwork-dir <dossier>` charge des PNG/JPEG locaux à la place des motifs
procéduraux, sans modifier les métadonnées fictives. Exemple :

```powershell
cargo run --offline --bin rustmusic-demo --target-dir target/artwork-check -- --artwork-dir src/assets
```

Voir le [guide de chargement](../image-loading.md) pour les budgets et les erreurs.
`--benchmark` produit un rapport JSON hors écran ; le protocole et un premier
relevé local sont dans le [guide des mesures](performance.md).
`--profile` enregistre les peintures d’une fenêtre normale jusqu’à la soumission
Windows ; F8 et la fermeture écrivent le [rapport détaillé](presentation-profiling.md).

## Ce qui est fonctionnel

| Partie | Comportement |
| --- | --- |
| Navigation | Accueil, Tous les titres et Albums favoris. |
| Recherche | Filtre les albums et artistes, sans tenir compte de la casse et des espaces extérieurs ; les titres associés sont filtrés aussi. |
| Collection | 48 albums fictifs, six par page, grille de deux à quatre colonnes. |
| Pochette | Son activation sélectionne un titre associé à l’album. |
| Liste | 10 000 titres fictifs à hauteur fixe ; seules les lignes utiles sont matérialisées. |
| Lecteur fixe | Lecture/pause simulée, titre précédent/suivant, position, volume et favoris d’album. |
| File de lecture | Panneau ou modale compacte, sélection, doublons, ajout, retrait, ordre suivant/précédent, drag/drop et commandes clavier ; 10 000 entrées maximum virtualisées. |
| Diagnostics | Modale, backend actif, utilisation du cache et rechargement des pochettes. |

Le lecteur modifie un état applicatif ; il n’émet aucun son et la position
n’avance pas automatiquement. Les favoris et la recherche ne sont pas
persistés à la fermeture. Le nom du titre choisi est visible dans le lecteur.

Le cœur est dessiné par un widget de la démo et expose une commande de bouton
avec un nom accessible qui distingue l’ajout du retrait. Il n’expose pas encore
un pattern Toggle comme la Checkbox commune. Les commandes accessibles des
boutons et sliders restent celles du socle.

L’en-tête et le lecteur encadrent une zone centrale flexible. Le défilement
déplace le contenu de cette zone, pas le lecteur. Les seuils de colonnes sont
pour cette démo : quatre à partir de 1 280 unités, trois à partir de 1 050,
sinon deux. La largeur minimale de fenêtre est de 900 unités logiques.

## Pochettes en arrière-plan

Les illustrations sont générées par du code Rust avec des motifs et palettes
déterministes ; elles ne représentent pas des albums réels et ne sont pas
téléchargées. Le travail crée des pixels BGRA prémultipliés de 192 × 192,
avec vérification du jeton d’annulation à chaque ligne.

La page demande les six pochettes de son lot et celle du lecteur, sans doublon.
Les clés déjà présentes dans le cache ne sont pas regénérées. Une relance
utilise le même TaskId : le [service de tâches](../tasks.md) annule la précédente
et rejette ses résultats obsolètes. Des emplacements de remplacement restent
interactifs pendant le chargement.

Avec `--artwork-dir`, le même service de tâches lit et décode les fichiers
PNG/JPEG en miniatures. Une erreur laisse un remplacement visible ; les autres
fichiers restent utilisables. Le cache utilise une clé par fichier, y compris
si plusieurs albums fictifs le partagent. Diagnostics indique les erreurs et
les durées du dernier lot. L’extraction depuis les fichiers musicaux reste à faire.

## Cache CPU réutilisable

`window_engine::resources::ImageCache<K>` est indépendant de la démo et de l’OS :

```rust,ignore
let mut cache = ImageCache::<u64>::new(1_179_648);
cache.insert(album_id, image)?;
let image = cache.get(&album_id);
let stats = cache.stats();
```

Le budget de la démo est de 1 152 Kio, soit huit pochettes à la taille actuelle.
Le cache compte la capacité allouée du Vec de pixels de chaque entrée, et non
seulement sa longueur. Ses compteurs concernent ses entrées et excluent les
en-têtes, les clés, les métadonnées et les autres allocations de l’application.
Une même image sous plusieurs clés serait comptée pour chaque entrée.

`get` actualise la récence et renvoie un Arc. L’insertion évince les entrées
les moins récemment utilisées jusqu’à respecter le budget. Une image trop
grande est refusée avant toute éviction ; un buffer aux dimensions incohérentes
renvoie une erreur. Le producteur doit fournir une Image BGRA prémultipliée
valide, normalement construite par `Image::new`.

`contains` consulte sans changer les compteurs. `remove`, `clear` et
`set_budget` permettent de gérer le cycle de vie ; les compteurs d’accès et
d’éviction restent cumulatifs après un clear. Le cache ne conserve pas de
second buffer de pixels pour faire ses copies : il partage les Arc.

**Ce budget n’est pas une borne de RAM globale.** Une scène ou un widget qui
utilise encore une image conserve son Arc après l’éviction. Les textures GPU
et les résultats en transit ont leur propre durée de vie. Le renderer Windows
conserve les bitmaps de la scène actuelle et retire les références qui n’y
figurent plus à la présentation suivante. Une scène précédente peut donc
encore retenir ses ressources jusqu’au prochain rendu.

Le bouton « Vider puis recharger » vide les références du cache et redemande
les images de la page actuelle. Il n’efface aucun fichier. Sa logique et les
évictions sont testées automatiquement ; ce bouton n’a pas été activé pendant
la recette visuelle de cette étape.

## Où intervenir

| Fichier | Rôle |
| --- | --- |
| `src/bin/rustmusic-demo.rs` | Entrée et options de l’exécutable séparé. |
| `src/music/mod.rs` | État, commandes, filtrage, demandes de tâches et réception. |
| `src/music/view.rs` | Composition de la page, du lecteur et de la modale. |
| `src/music/styles.rs` | Recettes partagées de panneaux, textes et boutons. |
| `src/music/components.rs` | Pochettes interactives, icônes et lignes spécialisées. |
| `src/music/model.rs` | Données fictives, durées et génération des illustrations. |
| `src/music/artwork.rs` | Catalogue local, chargement borné et erreurs par fichier. |
| `src/resources.rs` | Cache générique, réutilisable dans d’autres applications. |
| `src/resources/image_loader.rs` | Lecture commune et contrat de décodage. |

Ces fichiers applicatifs n’utilisent aucun handle natif. Les éléments sont
construits avec Row, Column, Grid, Scroll, VirtualList et les widgets communs.
Les composants spécialisés implémentent Widget, produisent des commandes de
scène et exposent leurs noms et actions accessibles.

## Validation

La suite compte 204 tests : 186 dans la bibliothèque, seize dans la démo musicale
et deux dans la galerie. Ils couvrent notamment le cache, la génération BGRA,
PNG/JPEG, les limites avant décodage, l’annulation, les erreurs partielles,
le partage par fichier, le recadrage et le benchmark natif. Les commandes,
le filtre, les favoris et le layout aux formats 900, 1 050 et 1 360 sont aussi
testés, dans les deux thèmes.

Un test clavier atteint le titre 10 000 avec End et l’active avec Entrée : le
lecteur affiche Signal 10000 et le nombre de lignes matérialisées reste sous
24 pour ce viewport. Ce seuil décrit ce scénario, pas une limite globale du
moteur. Un test conserve puis libère une référence externe après éviction pour
vérifier la différence entre budget du cache et durée de vie des images.

Recette précédente à l’écran, avec illustrations procédurales : pochettes chargées,
navigation vers les titres, recherche « luma » donnant 1 667 titres, activation
d’un résultat, ajout au favori et affichage de l’album, modale et compteurs,
backend GPU puis CPU WARP confirmé dans Diagnostics, thème clair, réduction
à 900 × 640 et défilement avec deux colonnes et lecteur fixe. Les instances
de recette de `target/music-check` ont été fermées après les essais.

Le chargement de fichiers est vérifié par tests natifs, rendu hors écran et
recette visible GPU/WARP. Le [guide de la file](playback-queue.md) décrit les
commandes ; le [relevé de session](queue-validation.md) distingue les essais
visuels, les limites clavier/accessibilité et les mesures de la version finale.

Le lecteur d’écran, la saisie IME et les overlays restent à recetter sur cet
écran. Le client UI Automation peut encore annoncer un focus différent du
focus visuel dans certains contrôles, limite déjà suivie dans le socle.

Les tests et le relevé hors écran ne constituent pas un benchmark face à Tauri.
Le temps de composition et la latence jusqu’au moniteur restent à mesurer,
puis à comparer sur un
écran et une qualité équivalents. J5 n’est pas entièrement terminé : menu
métier, formulaire et autres états représentatifs restent à
valider. La prochaine mesure devra inclure la composition de bout en bout
avant intégration dans RustMusic.
