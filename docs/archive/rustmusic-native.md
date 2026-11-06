# Reproduction native de RustMusic (exemple retiré)

> L’exemple `rustmusic` (vues Rust natives) a été retiré : la page RustMusic est
> désormais reproduite en templates HTML par [`rustmusic_html`](../rustmusic-html.md),
> qui a repris ses polices et ses pochettes PNG
> (`examples/rustmusic_html/assets/`). Le code et les aperçus ci-dessous restent
> dans l’historique git (commit `a37ea7e` et antérieurs) ; les commandes ne sont plus
> exécutables.

Exemple autonome demandé à partir de la capture fournie : barre supérieure,
recherche, panneau gauche, reprise de lecture, file de quatre titres, mosaïques,
lecteur fixe et informations techniques. L’application est dans `examples/` ;
elle ne réintroduit aucune dépendance métier RustMusic dans `src/`.

```powershell
cargo run --offline --release --example rustmusic --target-dir target/rustmusic-native
```

La fenêtre initiale mesure 1440 × 1040 unités logiques. `--size 1600x1152`
change ce format. Les marges et tailles suivent la largeur de la fenêtre,
avec défilement indépendant du contenu principal et lecteur fixe.
Le chrome standard est désactivé pour éviter une seconde barre de titre.
Les boutons Réduire, Agrandir et Fermer utilisent les commandes natives.
Le déplacement par glisser de cette barre personnalisée n’est pas encore câblé.

## Aperçus avec le renderer réel

```powershell
cargo run --offline --release --example rustmusic --target-dir target/rustmusic-native -- --export artifacts/rustmusic-native.png --renderer=software
cargo run --offline --release --example rustmusic --target-dir target/rustmusic-native -- --export artifacts/rustmusic-native-gpu.png --renderer=gpu
cargo run --offline --release --example rustmusic --target-dir target/rustmusic-native -- --export artifacts/rustmusic-native-1440.png --size 1440x1040
```

L’export prend par défaut les dimensions de la référence, **2160 × 1556**, et
appelle le même runtime et le même renderer que la fenêtre. Il n’ouvre aucune
fenêtre et ne capture pas un navigateur.

- référence reproduite à 2160 × 1556 (`artifacts/rustmusic-native.png`, retiré) ;
- version à 1440 × 1040 (`artifacts/rustmusic-native-1440.png`, retiré) ;
- export GPU (`artifacts/rustmusic-native-gpu.png`, retiré).

Les fontes sont Figtree et Inter, avec JetBrains Mono pour les durées et données
techniques. Les icônes sont les vrais contours Material Symbols. Les pochettes
principales proviennent du cache local de RustMusic, copié en lecture seule ;
les autres viennent des catalogues publics. Certaines éditions des pochettes
secondaires diffèrent de la capture. Les métriques et le lissage DirectWrite
peuvent également différer du navigateur.

## Interactions

Les deux boutons Lecture partagent leur état, ainsi que les deux progressions.
Volume, sourdine, favoris, aléatoire et répétition conservent leur état.
La recherche est un vrai champ éditable : saisie, sélection et presse-papiers
utilisent le socle existant. Les tuiles latérales changent de sélection.
La file, les sources, les imports et les mix ouvrent des dialogues de maquette.
Tab, Entrée/Espace et Échap suivent le runtime commun.

C’est un test d’interface : il n’y a pas de lecture audio, d’import réel, de
recherche en bibliothèque, de raccourci Ctrl K ni de navigation métier complète.
Les informations FLAC, DSP et sortie audio reprennent la référence.

## Où modifier la page

| Fichier dans `examples/rustmusic/` | Rôle |
| --- | --- |
| `mod.rs` | État partagé et actions applicatives |
| `header.rs` | Logo, recherche, profil, commandes de fenêtre |
| `sidebar.rs` | Sources, navigation, playlists, profil inférieur |
| `content.rs` | Accueil, reprise, file et cartes de mix |
| `player.rs` | Contexte et appel du template du lecteur |
| `player/templates/` | Layout partagé, includes du transport, du volume et de la bande technique |
| `player/styles.rs`, `player/components.rs` | Recettes typées et adaptateurs des widgets du lecteur |
| `controls.rs`, `rows.rs` | Widgets réutilisables et accessibles |
| `components.rs`, `line.rs` | Images, cadres, texte monoligne avec ellipse |
| `style.rs` | Palette et helpers typographiques |
| `assets.rs`, `assets/` | Polices, icônes, pochettes et mosaïques |

Le lecteur utilisait les [templates compilés](../templates.md).
Le reste de la page utilise `row`, `column`, `grid`, `scroll` et leurs styles Rust.
Les widgets ajoutent des commandes à `Scene`, rendues par Direct2D/DirectWrite.
Images et contours sont préparés une fois au chargement puis partagés par `Arc`.
Les dégradés et mosaïques sont calculés au chargement et réutilisés.
Une crate locale de macros compile le balisage ; elle n'a aucune dépendance
tierce et n'ajoute aucun parser au runtime. Aucun navigateur n'est requis.
La comparaison pixel à pixel avant/après migration du lecteur à 2160 × 1556
était dans `artifacts/template-parity.json` (retiré avec l’exemple).

Deux ajouts au socle permettaient cette personnalisation :
`WindowCommand::Action(id, WindowAction)` pour une barre personnalisée (toujours
utilisé par `bootstrap_demo` et `rustmusic_html`), et `SliderStyle::track_thickness`
pour régler l’épaisseur des pistes (sans exemple désormais, couvert par les tests
du Slider dans `src/ui/tests/components/slider.rs`).

Les tests de l’exemple vérifient les positions à deux formats, la réutilisation
des images, les actions du lecteur, les valeurs synchronisées, le champ éditable,
la file et les commandes de fenêtre. Un test natif vérifie le ciblage de la
fermeture ; les épaisseurs de slider sont vérifiées dans les deux orientations.

Les sources des polices et des pochettes reprises sont documentées dans
[`examples/rustmusic_html/assets/SOURCES.md`](../../examples/rustmusic_html/assets/SOURCES.md).
