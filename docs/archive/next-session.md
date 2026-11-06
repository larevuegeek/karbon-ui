> Archive : cette page décrit les anciennes démonstrations retirées du projet. Les commandes et chemins de leur code sont historiques ; la bibliothèque actuelle conserve ses API et ses tests.

# Point de reprise — 2 novembre 2026

Dernier travail : réorganisation complète de `src/ui/` par responsabilité.
Lire le [plan du moteur](../../src/ui/README.md). Dossiers `core`, `components`,
`overlays`, `layout`, `rendering`, `runtime`, `view` et `tests` ; les seuls
fichiers à la racine sont `mod.rs`, `prelude.rs` et `README.md`.
Les anciens chemins `ui::element`, `ui::widgets`, `ui::text_input`, etc. sont
réexportés, chaque type et fonction ayant une seule implémentation.
TextInput, Select, Slider, choix, styles, scène et runtime sont découpés par
responsabilité. Checkbox, Switch et RadioButton ont chacun leur fichier.
Le placement a des fichiers dédiés aux lignes/colonnes, à la grille, au split,
aux couches et au scroll ; le runtime sépare clavier, souris, focus,
accessibilité, glisser-déposer et préparation de scène.
215 tests passent, dont les tests GPU explicites ; formatage et Clippy passent
sans avertissement. Toutes les cibles compilent en release dans
`target/ui-organization`. Les PNG GPU/WARP après réorganisation sont identiques
aux références existantes, SHA-256 compris. Les copies de contrôle sont dans
`target/ui-organization/sidebar-after*.png`. La sauvegarde de l'ancien `ui/`
est dans `target/ui-organization/source-before/ui/` ; les scripts temporaires
de découpage sont dans le même répertoire de build.

Étape précédente : simplification de la syntaxe et organisation. Point d’entrée
API : `ui::prelude::*`, fonctions `row/column/grid/...`, unités numériques,
`ViewStyle` partiels composables, clés nommées et portées `ElementId`, macro
`actions!`, et `WidgetExt::element`. Les contrôles courants acceptent les actions
typées. `Text` accepte famille/graisse et conserve le retour à la ligne natif.
Lire [la syntaxe](../syntax.md) et [l’organisation](../architecture.md).
Exemple : `cargo run --offline --example simple_ui --target-dir target/syntax-check`.
`sidebar/` sépare maintenant vue, état, styles, clés, ressources et composants ;
`demo/` regroupe la galerie, et `ui/widgets/` sépare Text et Button. Les chemins
publics de la bibliothèque restent compatibles avec les méthodes existantes.
215 tests passent, tests explicites compris ; Clippy sans avertissement.
Toutes les cibles compilent en release dans `target/syntax-check`.
Le PNG logiciel du panneau est identique avant/après refonte (SHA-256
`b342330b6d40a302a5da8711208d22585e36c264a4a15720a0dd9289f27754f0`).

Étape précédente : reproduction du panneau gauche RustMusic dans une démo séparée,
`src/bin/sidebar-demo.rs` et `src/sidebar/`. Voir le
[guide et les aperçus](sidebar-reference.md). Polices Figtree/Inter privées,
icônes vectorielles à plusieurs contours et option `WindowOptions::title_bar`.
Lancer `cargo run --offline --release --bin sidebar-demo --target-dir target/sidebar-check`.
209 tests passent, dont les tests GPU explicites ; Clippy et formatage passent.
Release du panneau compilée. Les PNG GPU/WARP sont dans `artifacts/sidebar-preview*.png`.
La fenêtre visible n’a pas été vérifiée : l’ouverture Computer Use a expiré ;
les contrôles, le layout, le focus et une fenêtre native cachée sont testés.
Les comptes et recettes des étapes précédentes ci-dessous restent historiques.

Le socle reste une bibliothèque Rust sans crate externe. Windows est le seul
backend implémenté. La démonstration et ses données sont dans `src/demo/mod.rs` ;
le backend ne contient pas de code propre à RustMusic.

Direction validée : futur nom `karbon-ui`, bibliothèque autonome et intégration
facultative à Karbon. Continuer ici ; ni migration de dépôt ni renommage Cargo
pour l’instant. Une même API et une identité visuelle partagée sont la cible
Windows/macOS/Linux. Viser des gains importants face à Tauri, sans annoncer de
résultats avant une comparaison complète. Voir les sections 1.1 et 15 du
[cahier des charges](../ui-specification.md).

## Disponible

- Fenêtre sans bordure, déplacements, resize, boutons, coins et ombre.
- Rendu GPU Direct2D et logiciel WARP, texte DirectWrite, images et transparence.
- Lignes, colonnes, grille de 12 colonnes, spans et positions explicites.
- Tailles Auto / Px / % / Fill, contraintes, alignements, espacements.
- Panneaux redimensionnables en option, verrouillage et inversion dans la démo.
- Boutons avec souris, capture, focus, Entrée et Espace.
- Checkbox, Switch et groupes radio contrôlés par l’application, états mixtes,
  styles de couleur, Tab/flèches et patterns accessibles Toggle/Selection.
- Slider horizontal/vertical : bornes, pas, capture, clavier, annulation,
  aperçu/validation séparés et RangeValuePattern. Voir le [guide](../slider.md).
- Select non éditable : options par clé, popup défilante, souris, clavier,
  recherche par préfixe et ExpandCollapse/Selection. Voir le [guide](../select.md).
- Modales composables, pile jusqu’à huit dialogues, fond bloquant, focus initial,
  boucle Tab, retour du focus et contenu accessible limité à la couche active.
  Voir le [guide](../modal.md).
- Popovers ancrés, menus au clic droit et Shift+F10, actions, séparateurs,
  désactivation, navigation par flèches et fermeture temporaire avec retour
  du focus. Voir le [guide](../popover.md).
- Info-bulles au survol et au focus clavier, styles, délai, fermeture par Échap,
  réveil ponctuel, HelpText et nœud ToolTip. Voir le [guide](../tooltip.md).
- Recettes partagées SurfaceStyle, ButtonStyle et TextStyle, couleurs de thème,
  surcharges locales, variantes Ghost/Outline et `Theme::on_accent`.
  Voir le [guide](../styles.md) et `src/demo/styles.rs`.
- Plusieurs fenêtres Windows sur une boucle UI, runtimes et ressources distincts,
  commandes différées Open/Close/Exit et politiques LastWindowClosed/MainWindowClosed.
  Voir le [guide](../windows.md), `src/demo/windows.rs` et `examples/multi_window.rs`.
- Tâches de fond bornées, résultats typés sur le thread UI, annulation, relance
  et rejet des anciennes réponses, y compris après fermeture/réouverture.
  Voir le [guide](../tasks.md), `src/demo/tasks.rs` et `examples/background_task.rs`.
- Premier écran RustMusic fictif : navigation, recherche, favoris, lecteur
  simulé, 10 000 titres virtualisés, illustrations de fond et cache CPU LRU.
  Voir le [guide](rustmusic-demo.md). Le budget du cache exclut scènes et GPU.
- Premier fournisseur Windows UI Automation : arbre, noms, états, focus,
  ValuePattern des champs, TextPattern monoligne et InvokePattern des boutons,
  avec notifications de texte et de sélection.
- Champ TextInput monoligne, styles, sélection, historique, presse-papiers et
  raccordement IMM. Valeur conservée par `Application::text_changed`.
- Scroll vertical et horizontal, barres déplaçables, molette, clavier,
  viewports imbriqués et révélation des contrôles focalisés.
- État conservé par identifiant : focus, proportions et offsets de scroll.
- Liste virtualisée verticale à hauteur fixe : réutilisation et libération des
  lignes, marge configurable, navigation logique et activation d’une ligne.

## Reprendre la démo

```sh
cargo run --offline
```

Le formulaire en haut accepte un nom de playlist. Essayer accents/emoji,
Ctrl+A/C/X/V/Z/Y, la sélection souris, puis Entrée ou Valider. Le bouton Effacer
illustre une remise à zéro depuis l’application. Voir le [guide TextInput](../text-input.md).

Défiler dans la liste de 10 000 titres, glisser sa petite barre jusqu’au bout,
puis défiler hors de la liste pour déplacer son panneau parent. Cliquer la
barre donne aussi accès à Home, End et Page Up/Down. F2 change GPU/CPU ; F3
change la transparence ; F5 change le thème ; F8 affiche les compteurs, dont
`virtual_rows`. Cliquer une ligne puis End atteint directement la dernière ;
Entrée l’active. Haut/Bas parcourent les lignes et Tab quitte la liste.

Lire [le guide du scroll](../scroll.md) et le [guide des listes virtualisées](../virtual-list.md),
puis suivre ces fichiers :

| Fichier | Rôle |
| --- | --- |
| `src/demo/mod.rs` | Construire la vue avec des éléments et des styles. |
| `src/ui/core/element/mod.rs` | Contrat public des éléments et des widgets. |
| `src/ui/layout/mod.rs` | Mesurer et positionner. |
| `src/ui/layout/scroll.rs` | Calculer les barres, décaler le contenu et recalculer les clips. |
| `src/ui/layout/virtual_list.rs` | Construire et réutiliser seulement les lignes utiles. |
| `src/ui/runtime/mod.rs` | État, focus, capture, événements, puis production de la scène. |
| `src/ui/components/text_input/mod.rs` | Style, édition, historique, sélection et composition du champ. |
| `src/platform/windows/text_input.rs` | Presse-papiers, UTF-16, caret et IMM. |
| `src/platform/windows/accessibility/` | Fournisseurs COM, snapshots natifs, notifications et retour des commandes sur le thread UI. |
| `src/platform/windows/events.rs` | Convertir les messages Windows en événements UI. |
| `src/platform/windows/drawing.rs` | Coordonner préparation de scène et rendu natif. |

## Vérifications de cette étape

Formatage et Clippy sans avertissement. La suite compte 204 tests, avec les tests
explicites GPU et logiciel ; le test de longue liste contrôle le nombre de widgets peints,
l’absence de remesure pendant le scroll et la libération des widgets.

```sh
cargo fmt --check
cargo clippy --offline --all-targets -- -D warnings
cargo test --offline -- --include-ignored
cargo build --offline
```

`--include-ignored` demande un adaptateur matériel pour le test GPU ; les tests
ordinaires peuvent être lancés avec `cargo test --offline`.

Vérifié à l’écran : molette interne puis externe, glissement jusqu’à la ligne
1 000 lors de l’étape scroll, puis 10 000 à l’étape virtualisation, Home et Page Down, maintien des offsets après passage GPU vers CPU,
fond transparent, puis resize d’une fenêtre opaque de 1 040 à 800 unités de
large avec retour de la grille à deux cartes. La démo a été fermée après les essais.

La nouvelle liste a aussi été vérifiée sur GPU, sur WARP et en transparence :
clic d’une ligne, End, Entrée sur la ligne 10 000, conservation après mise à
jour et changement de backend, puis retour à Home. Les tests contrôlent le
nombre d’objets détenus et leur libération ; la RAM globale et le temps d’image
complet restent à mesurer. Les chronométrages du cœur seul sont dans le guide.

TextInput a été essayé à l’écran avec accents et emoji, copie/collage, annulation,
validation par Entrée et insertion au début après passage au rendu logiciel.
Les compositions IME sont testées par événements simulés ; l’essai avec un IME
japonais/chinois reste à faire. Le [premier pont UI Automation](../accessibility.md)
a été vérifié depuis un client externe : découverte du champ, lecture de valeur,
SetValue, activation de Valider et relecture du résultat. TextPattern a ensuite
été ajouté et testé dans les interfaces COM et le runtime : plages, recherche,
géométrie, sélection, révélation sans vol de focus et composition simulée.
La recette externe de TextPattern et les essais lecteur d’écran restent à faire.

L’instance utilisée pour le test UI Automation a été laissée ouverte, car elle
était en cours d’utilisation. La version finale a été compilée séparément avec
`cargo build --offline --target-dir target/accessibility-check` ; l’instance déjà
ouverte doit être relancée pour charger les dernières modifications.

Les [composants de choix](../choices.md) sont dans `src/ui/components/choices/mod.rs` et présentés
dans le panneau « Choisir et activer » de la démo. Version vérifiée :
`cargo run --offline --target-dir target/choices-check`. L’instance de recette
a été fermée. Vérifiés à l’écran : checkbox, switch, radio exclusif, Tab puis
flèche gauche, thèmes clair/sombre et passage GPU vers WARP avec valeurs conservées.
Les rôles et actions sont découverts par le client UI Automation. Son indication
de focus est restée sur le premier champ alors que les anneaux et le clavier
suivaient les contrôles ; faire la recette d’annonces avec un lecteur d’écran.

La dernière démo ajoute « Régler une valeur » avant les choix. Volume immédiat
et position validée au relâchement sont dans `src/demo/mod.rs`, via `value_changed`
et `value_committed`. Commande de reprise :
`cargo run --offline --target-dir target/slider-check`. L’instance de recette a
été fermée. Vérifiés : UI Automation SetValue à 80, volume glissé à 40, position
à 165 puis flèche gauche à 164, passage WARP et thème clair avec état conservé.

La dernière version ajoute « Bibliothèque active », avant les sliders. La valeur
est conservée par `selection_changed` ; les clés et les options sont dans
`src/demo/mod.rs`. Reprise : `cargo run --offline --target-dir target/select-check`.
Vérifiés à l’écran : ouverture par clic et par clavier, menu devant les sliders,
choix de Serveur NAS depuis le client accessible, puis End et Entrée sur la
dernière option. Le choix est conservé après passage WARP et thème clair.
La recette a aussi révélé puis corrigé un mouvement souris sans déplacement
qui reprenait le choix actif après navigation au clavier ; un test le couvre.
L’instance de recette a été fermée. Les annonces lecteur d’écran restent à tester.

La dernière démo ajoute « Afficher une modale » dans l’en-tête. Elle contient un
formulaire avec TextInput et Select, puis une confirmation au-dessus. Les états
applicatifs sont `modal_open` et `confirmation_open` dans `src/demo/mod.rs`.
Reprise : `cargo run --offline --target-dir target/modal-check`.
Vérifiés à l’écran : ouverture, saisie, Tab, Select superposé, Échap, confirmation,
retour avec texte conservé, passage WARP, thème clair et fermeture par le fond
avec restauration du focus au déclencheur. Le client accessible ne découvre
que le dialogue actif ; IsDialog est testé dans le fournisseur COM. Une frappe
simple fonctionne après le focus automatique. L’injection de texte complète
du client a parfois nécessité de recliquer le champ ; sa recette et les annonces
Narrateur/NVDA restent à approfondir. L’instance de recette a été fermée.

Dernière version compilée : `cargo run --offline --target-dir target/popover-final`.
« Réglages » et « Actions » se trouvent dans le panneau du compteur ; le clic
droit dans ce panneau et Shift+F10 sur ses contrôles ouvrent aussi le menu.
La modale a son bouton Réglages. `src/ui/overlays/popover/mod.rs` porte l’API et les widgets
de menu ; `ui/overlays/layers.rs` porte maintenant la validation et la pile de focus communes.
Les tests couvrent aussi les ancres retirées, les menus longs et la ligne active
d’une liste virtualisée. À l’écran, le menu est visible et le client accessible
découvre ses rôles, actions, séparateur et option désactivée.
La démo `target/popover-check` est restée ouverte car elle était en cours
d’utilisation : ne pas écraser son exécutable pendant qu’elle tourne. La dernière
correction du signal clavier WM_CONTEXTMENU en 32/64 bits est compilée dans
`target/popover-final` et prendra effet après relance de cette version.
La recette visuelle complète du
popover, du clic droit, de Shift+F10 et des annonces lecteur d’écran reste à faire.

Les pixels d’alpha nul d’une fenêtre layered laissent passer les interactions
Windows : attraper un coin entièrement transparent pour redimensionner ne
fonctionne pas comme sur un fond opaque. La politique de clics par zones et
les poignées utilisables sur un overlay restent à approfondir.

Dernière étape : `cargo run --offline --target-dir target/tooltip-check`.
`.tooltip("…")` et `.tooltip_with(Tooltip::new(...))` ajoutent une aide sans état
applicatif. L’état et le dessin sont dans `src/ui/overlays/tooltip/mod.rs`, et le réveil
ponctuel dans `src/platform/windows/timer.rs`. Les délais par défaut sont de
500 ms ; aucun timer d’aide ne tourne au repos ou après son affichage.
Vérifié à l’écran : premier Tab vers le bouton de modale, délai Windows,
présence du texte et du nœud UIA, Échap sans fermer l’application, Tab/Shift+Tab
pour réarmer, passage WARP et thème clair. Cette instance de recette a été
fermée. Les tests couvrent aussi le pointeur stationnaire pendant la navigation
clavier, les contrôles désactivés, le survol de la bulle, les clics traversants,
les scopes et la libération du texte. Compléter la recette manuelle du survol
et les annonces Narrateur/NVDA. Le client peut encore conserver une indication
de focus sur le premier champ malgré le focus réel et le tooltip correspondants.

## Prochaines étapes proposées

Dernière étape : file de lecture et drag/drop interne réutilisable, avec
commandes accessibles hors de la liste pour le clavier. Ordre par clés stables,
doublons, ajout/retrait, choix du successeur et borne à 10 000 entrées virtualisées.
Voir [la file](playback-queue.md), [le drag/drop](../drag-drop.md) et [la recette
avec ses mesures](queue-validation.md). 204 tests passent, Clippy sans avertissement,
formatage et compilation release de toutes les cibles validés.
Reprise : `cargo run --offline --release --bin rustmusic-demo --target-dir target/queue-verified -- --queue --profile --artwork-dir artifacts/artwork-fixtures-v1`.
Les sessions visibles GPU/WARP ont confirmé images locales, drag, boutons,
sélection/lecture, titre 10 000 et scroll indépendant. GPU a aussi été essayé
en compact et transparent. Les sessions visibles précèdent la dernière barre
de commandes clavier ; celle-ci est testée par focus/Espace et dans le rendu
hors écran final. Le client accessible peut encore rapporter un focus ancien.
La fenêtre WARP de `target/queue-final` a reçu des interventions utilisateur et
est laissée ouverte ; elle ne contient pas la dernière barre de commandes.
Ne pas écraser cet exécutable en cours d’utilisation. Les logs visibles sont
figés dans `artifacts/queue-visible-*.log`, avec contexte et empreintes JSON.
Le relevé final hors écran avec file ouverte donne un p95 de scroll de 3,275 ms
GPU / 3,638 ms WARP à 1 360 × 920, sans reconstruction ni layout. Les mesures
visibles à 150 % de DPI, le repos et la RAM sont détaillés séparément ; elles
ne sont pas une comparaison Tauri ni une preuve d’absence de fuite.
Prochaine priorité : menus et formulaires métier représentatifs, puis recette
lecteur d’écran/IME, mesure de composition et présentation sans relecture CPU
selon les résultats. Répéter les séries longues avec une charge contrôlée
avant d’annoncer un gain global.

Étape précédente : relecture directe dans le DIB persistant, supprimant le Vec
de pixels et sa copie supplémentaire dans le chemin normal de peinture.
Le profilage devient `window-submit-direct-dib-v2`, surface créée avant rendu,
soumission sans copie. Le benchmark hors écran conserve son Vec et son protocole.
195 tests passent : pixels comparés GPU/WARP avec alpha, clipping et DPI,
destination invalide refusée, 96 changements d’image avec resize et libération
des anciennes références. Le contrôle natif caché en release est dans
`artifacts/presentation-direct-smoke.json` ; aucun gain chiffré n’en est déduit.
La recette visible Computer Use a expiré à l’approbation avant lancement ;
aucune nouvelle fenêtre visible de test n’a été ouverte. Le lanceur
`examples/profile_session.rs` ouvre deux sessions identiques GPU puis WARP
et conserve les logs. Voir les commandes dans le guide du profilage.
Faire cette recette et les mesures visibles de repos/navigation avant de
conclure sur le gain global. La relecture CPU reste présente.

Étape précédente : [profilage de fenêtre](presentation-profiling.md), option
`--profile` désactivée par défaut, 1 024 échantillons maximum par fenêtre.
Préparation, rendu/relecture CPU, création de surface et soumission Windows sont
mesurés séparément. F8 et fermeture produisent le JSON ; F2 sépare les backends.
Les instantanés CPU/RAM concernent tout le processus, repos compris.
La suite compte 194 tests. Contrôle release isolé, fenêtre cachée WARP avec
resize : rapport dans `artifacts/presentation-smoke.json`, aucun échec et
toutes les fenêtres détruites. Aucun nouveau relevé représentatif visible.
Reprise : `cargo run --offline --release --bin rustmusic-demo --target-dir target/presentation-check -- --profile --artwork-dir artifacts/artwork-fixtures-v1`.
Les sessions visibles GPU/WARP, repos et navigation ont depuis été consignées
avec la file ; la composition DWM et la latence jusqu’au moniteur restent hors mesure.

Étape précédente : [chargement PNG/JPEG](../image-loading.md) sur worker, miniature
192 pixels, limites de fichiers/dimensions, erreurs partielles et partage du
cache par fichier. Reprise :
`cargo run --offline --bin rustmusic-demo --target-dir target/artwork-check -- --artwork-dir src/assets`.
La suite compte 190 tests, dont onze musicaux. Les tests natifs couvrent PNG,
JPEG et le confinement du service de décodage au thread appelant.
Le [benchmark hors écran](performance.md) mesure 1 024 rendus par phase,
RAM et temps CPU en release ; rapports et contexte dans `artifacts/`.
Scroll : zéro reconstruction et zéro layout ; p95 local 2,505 ms GPU et
2,509 ms WARP, avec relecture CPU mais sans présentation Windows.
Le lot mêlant fichier invalide et PNG valide conserve bien l’image utilisable.
La nouvelle instance lancée par le terminal n’était pas visible au contrôleur
visuel et a été arrêtée ; cette recette a depuis été réalisée avec la file.

Étape précédente : exécutable `rustmusic-demo`, données fictives et illustrations
procédurales sans réseau. Commande :
`cargo run --offline --bin rustmusic-demo --target-dir target/music-check`.
La galerie reste le default-run. Le cache générique est dans `src/resources.rs` ;
le modèle, la vue, les recettes et les widgets sont dans `src/music/`.
Six tests musicaux et quatre du cache avaient été ajoutés. Recette visuelle réalisée :
chargement, recherche Luma, activation, favoris, diagnostics, GPU vers WARP,
thème clair et redimensionnement 900 × 640 avec scroll et lecteur fixe.
Les deux instances de recette ont été fermées ; les anciennes galeries peuvent
toujours tourner dans leurs autres target-dir. Aucun audio réel ni intégration
à l’application RustMusic existante. Aucun gain global face à Tauri annoncé.

Étape précédente : tâches de fond et retour au thread UI, avec pool créé à la
première demande, un à quatre workers, file de 64 et limite de 64 tâches par
fenêtre. Reprise : `cargo run --offline --target-dir target/tasks-check`, panneau
« Travail en arrière-plan ». Génération factice de 20 000 titres, relance et
annulation ; le délai de simulation n’est pas un benchmark. L’exemple
`background_task` fait une vraie lecture du dossier courant. Neuf tests communs,
trois tests natifs, un test du runtime et un de la démo ont été ajoutés. La
recette visuelle et le lecteur d’écran restent à essayer ; les tests natifs
créent des fenêtres cachées. Aucun gain de performance global n’est annoncé.

Étape précédente : première API multi-fenêtre implémentée sous Windows.
Reprise : `cargo run --offline --target-dir target/windows-check`, puis bouton
« Ouvrir une fenêtre » dans le panneau latéral. `examples/multi_window.rs` montre
deux fenêtres initiales. Neuf tests natifs sur fenêtres cachées couvrent état,
réentrance, fermeture, erreurs, libération et réveil pendant GetMessage ; un test
supplémentaire vérifie les arbres et IDs de la démo. La recette visuelle et les
annonces lecteur d’écran simultanées restent à faire ; aucune fenêtre visible
de recette n’a été ouverte à cette étape. Les anciennes instances de démo
peuvent toujours être ouvertes ; ne pas écraser leur exécutable.

Priorités restantes : menus, formulaires et états métier représentatifs pour J5,
recette lecteur d’écran/IME et séries prolongées avec charge contrôlée.
Étudier la composition et la présentation sans relecture CPU à partir des mesures.
Conserver les essais d’accessibilité ci-dessous dans la recette.
Les styles seront approfondis plus tard. La progression des tâches, leurs
priorités et un runtime async restent hors de cette première étape.

Étape précédente des styles : `cargo run --offline --target-dir target/styles-final`.
Les recettes sont créées une fois dans `src/demo/styles.rs`. Les panneaux
partagent SurfaceStyle ; Ajouter et Confirmer partagent un bouton compact ;
les titres et surtitres partagent TextStyle. Réglages utilise Outline et Actions
utilise Ghost. Les dix nouveaux tests communs couvrent états, couleurs,
surcharges, focus, partage/libération et validation ; un test logiciel relit
les pixels d’un contour arrondi à plusieurs DPI. Vérifié à l’écran : panneaux,
boutons, compteur, taille de fenêtre, WARP et thème clair avec état conservé.
L’utilisateur a arrêté Computer Use avec Échap avant la fermeture de l’instance.
Aucune autre action dans cette fenêtre n’a été effectuée ; elle peut encore être ouverte.

1. Faire la recette externe de TextPattern avec un lecteur d’écran, puis les
   essais réels IMM/IME et bidi ; compléter les attributs utiles et TextPattern2
   selon les besoins constatés.
2. Compléter menus, formulaires et états de l’écran RustMusic et terminer
   la recette des popovers, menus et info-bulles. La généralisation des couleurs
   sémantiques aux autres widgets, les menus en cascade et les animations restent
   à ajouter selon les besoins de cet écran.
3. Étendre la virtualisation aux hauteurs variables, aux grilles et au suivi
   d’une sélection par clé après tri ; mesurer RAM et temps d’image complets.
4. Étudier la présentation GPU sans relecture CPU, en conservant le rendu
   logiciel et les overlays transparents.

Le cahier des charges reste la cible : ni J2 ni J4 ne sont entièrement terminés.
La saisie multiligne, la recette IME réelle, TextPattern2, les patterns avancés, les hauteurs variables, le docking libre et
les backends macOS/Linux restent à implémenter. Aucun gain chiffré face à Tauri
n’a encore été établi.



