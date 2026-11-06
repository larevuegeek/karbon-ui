# Layout et premiers composants J2

Le socle propose une mise en page native inspirée des usages de Flexbox et de
Grid : lignes, colonnes, grille régulière, tailles fixes ou flexibles, spans,
positions explicites et panneaux redimensionnables en option. Il ne s’agit pas
d’une implémentation complète de CSS. Aucune crate n’a été ajoutée.

## Exemple minimal

Le formulaire de `examples/simple_ui.rs` montre la composition publique :

```sh
cargo run --offline --example simple_ui
```

Les tests de layout et de runtime couvrent grille, contraintes, séparateurs,
clavier, capture et annulation. Les anciennes galeries ont été retirées.

## Composer une interface

L’application implémente `Application` et fournit un `Element` racine dans
`view()`. Le moteur mesure et dispose les enfants, puis produit les commandes
graphiques. Le backend Windows reçoit l’application par `platform::run` ;
il ne référence plus `gallery` ni une page de démonstration.

`context.width` et `context.height` décrivent l’espace utile sous la barre de
titre, en unités logiques. Le runtime positionne cet espace dans la fenêtre.

```rust
use karbon_ui::ui::{element::{Element, Length}, widgets};

#[derive(Clone)]
enum Msg { Play, Favorite }

let content = Element::column(1, vec![
    widgets::text(2, "Ma bibliothèque"),
    Element::row(3, vec![
        widgets::button(4, "Lire", Msg::Play),
        widgets::button(5, "Favori", Msg::Favorite),
    ]).gap(12.),
]).padding(20.).gap(16.);
```

Les nombres passés aux constructeurs sont des identifiants. Ils doivent être
uniques dans la vue et rester stables pour un même élément logique. Ils servent
à conserver le focus et les proportions des panneaux pendant la reconstruction
d’une vue. Un doublon est rejeté avant la mise en page.

Les dimensions sont exprimées en unités logiques :

- `Length::Px(240.)` : taille fixe de 240 unités ;
- `Length::Percent(25.)` : 25 % de la taille disponible du parent ;
- `Length::Fill(1.)` : une part de l’espace restant ; `Fill(2.)` reçoit deux parts ;
- `Length::Auto` : taille selon le contenu mesuré.

`min_width`, `max_width`, `min_height` et `max_height` limitent ces dimensions.
`padding` règle l’espace intérieur ; `gap` règle l’espace entre enfants.
`align` agit sur l’axe transversal et `justify` sur l’axe principal de la ligne
ou colonne. Les tailles explicites conservent leur valeur avec Stretch.

Les fractions se partagent l’espace après les tailles fixes, pourcentages,
minima et espacements. Un maximum atteint redistribue le surplus aux autres
fractions. Les contraintes impossibles entraînent un débordement découpé,
sans création automatique d’une zone de défilement.

## Grille en nombre de colonnes

```rust
let grid = Element::grid(100, 12, vec![
    Element::column(101, vec![]).span(4),
    Element::column(102, vec![]).span(4),
    Element::column(103, vec![]).span(4),
    Element::column(104, vec![]).span(6).at(2, 1),
]).gap(12.);
```

La grille possède ici douze colonnes de même largeur. Un span de quatre occupe
un tiers d’une rangée, un span de six la moitié. `at(2, 1)` désigne la deuxième
ligne et la première colonne, avec des positions comptées à partir de 1.
`column_start` fixe uniquement la colonne de départ. Les autres éléments sont
placés automatiquement dans les premières cellules disponibles.

Les positions explicites réservent leurs cellules avant l’auto-placement.
Les conflits, spans invalides et colonnes inexistantes produisent une erreur.
Chaque rangée prend la hauteur nécessaire à son enfant le plus haut ; les
cartes de hauteur automatique s’étirent dans leur rangée.

Pour un changement de disposition selon la largeur, l’application choisit un
span dans `view()` à partir de `context.width`.
Les pistes de largeurs hétérogènes, les spans verticaux, les zones nommées,
les marges négatives et le modèle complet de Flexbox/CSS Grid restent à ajouter
sur des cas concrets. Les concepts de référence sont décrits dans la
[spécification CSS Grid](https://www.w3.org/TR/css-grid-1/).

## Panneaux redimensionnables

```rust
use karbon_ui::ui::element::Split;

let panels = Element::split(
    200,
    Split::horizontal(0.3)
        .min_sizes(200., 320.)
        .resizable(true),
    sidebar,
    content,
).height(Length::Fill(1.));
```

Le premier panneau reçoit initialement 30 % de l’espace hors séparateur.
Les minima bornent le déplacement. Si la fenêtre ne peut pas satisfaire les
deux minima, l’espace disponible est partagé proportionnellement entre eux.
`Split::vertical` partage la hauteur. Le redimensionnement est désactivé par
défaut, et peut être activé par l’application avec `resizable(true)`.

Les proportions modifiées sont conservées en mémoire par identifiant pendant
la vie de la vue. Les entrées de panneaux supprimés sont retirées. La sauvegarde
sur disque des dispositions relève encore de l’application.

## État et performances

L’application conserve ses données métier. `update(message)` les modifie et
renvoie true pour reconstruire sa vue. Le runtime conserve les états de survol,
appui, focus et capture, ainsi que les proportions de panneaux.

La reconstruction de l’arbre a lieu lors d’une modification applicative ou du
contexte : dimensions, thème, backend ou transparence. Un glissement de
séparateur recalcule la disposition sans reconstruire la vue. Un survol ou un
changement de focus ne demande qu’une peinture. Une préparation sans changement
réutilise aussi la scène existante ; il n’y a pas de boucle de redessin continue.

Le tableau des commandes conserve sa capacité entre les peintures. Les textes
des widgets sont partagés via `Arc<str>`. Les mesures DirectWrite utilisent un
cache limité à 256 entrées, les formats natifs inutilisés par la scène sont
libérés, et les bitmaps restent limités aux images de la scène courante.
La présentation conserve encore la copie CPU introduite en J1.

Les dimensions des textes proviennent de DirectWrite, avec retour à la ligne
pour les labels ordinaires. Les boutons utilisent une seule ligne avec
clipping lorsqu’ils sont trop étroits. La troncature avec points de suspension
est une extension à prévoir. La mesure repose sur
[CreateTextLayout](https://learn.microsoft.com/en-us/windows/win32/api/dwrite/nf-dwrite-idwritefactory-createtextlayout)
et [GetMetrics](https://learn.microsoft.com/en-us/windows/win32/api/dwrite/nf-dwrite-idwritetextlayout-getmetrics).

## Extension et limites

Un widget personnalisé implémente `Widget::measure`, `Widget::paint` et peut
exposer une action et des informations sémantiques. Les conteneurs permettent
aussi de composer des widgets existants. Le dessin spécifique s’implémente avec le contrat `Widget`.

Les informations de rôle, nom, état, focus et rectangle existent dans le cœur
commun ; un [premier fournisseur Windows UI Automation](accessibility.md) expose
désormais cet arbre, les valeurs de champs et les actions de boutons. Le champ
monoligne expose aussi ses plages et sa sélection via TextPattern. TextPattern2,
les patterns avancés et la recette avec lecteur d’écran restent à compléter.
Les événements personnalisés complexes, la propagation entre parents, les
événements de saisie avancés restent à développer.
Le [défilement](scroll.md) est maintenant disponible : viewports imbriqués,
barres, molette, clavier et révélation du focus.
Une [liste virtualisée](virtual-list.md) verticale à hauteur fixe ajoute la
construction à la demande des lignes et une navigation clavier logique.
Le premier [TextInput](text-input.md) monoligne ajoute édition, sélection,
presse-papiers et un raccordement IMM ; la recette IME réelle et l’accessibilité
native restent à compléter.

Les tests vérifient notamment les contraintes, les positions et conflits de
grille, le clavier, la capture et son annulation, les contrôles désactivés et
la conservation du focus. Un test de 100 cycles de survol vérifie que le nombre
de reconstructions, de layouts, de mesures et la capacité de la scène restent
inchangés. La navigation, le resize, son verrouillage, l’inversion des panneaux,
le passage de trois à deux cartes et le mode logiciel ont aussi été vérifiés
à l’écran. Le test natif contrôle le retour à la ligne et la borne du cache de
mesures de texte.
