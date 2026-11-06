# Styles réutilisables : panneaux, boutons et textes

Une recette décrit l’apparence d’un composant. Plusieurs éléments peuvent
partager la même recette avec `Arc`, puis préciser certains détails localement.

## Un style partagé pour plusieurs panneaux

```rust
let card = Arc::new(SurfaceStyle {
    padding: Insets::all(20.),
    gap: 12.,
    radius: 14.,
    background: Some(ThemeColor::Surface.into()),
    border: Some(ThemeColor::Border.into()),
    border_width: 1.,
});

let first = Element::column(10, first_children).styled(Arc::clone(&card));
let second = Element::column(20, second_children)
    .styled(Arc::clone(&card))
    .padding(12.);
```

Importer les types depuis `ui::style`, et `Arc` depuis `std::sync`. `Insets`
vient de `ui::element`. Construire les recettes une fois dans l’application,
comme `DemoStyles`, puis cloner leurs références dans `Application::view`.

`SurfaceStyle` est une recette complète : elle applique padding, gap, rayon,
fond et contour. Elle conserve les dimensions, contraintes, alignements et
positions de grille déjà réglés sur l’élément.
Les appels suivants précisent la recette : `.padding`, `.gap`, `.radius` ou
`.surface` peuvent la surcharger. Un nouvel appel à `.styled` remplace la
recette précédente et réapplique ses trois paramètres de layout.

`background: None` ne peint pas de fond, y compris sur une couche stylisée.
`border: None` ou une largeur de contour nulle ne dessinent pas de contour.
Le trait est placé à l’intérieur du rectangle et ne réserve pas d’espace
supplémentaire dans le layout ; prévoir son padding selon le résultat désiré.

## Des couleurs liées au thème

`StyleColor` peut contenir une couleur fixe ou une référence à la palette :

```rust
let themed: StyleColor = ThemeColor::Accent.into();
let fixed: StyleColor = Color::rgb(80, 120, 240).into();
let translucent: StyleColor = ThemeColor::Surface.with_alpha(180);
```

Les références au thème sont résolues pendant la peinture. La même recette
peut ainsi suivre les thèmes clair et sombre. La couleur fixe garde ses valeurs
RGB/alpha. `ThemeColor` propose Background, Surface, TitleBar, Text, Muted,
Accent, Border, Hover, Pressed, Danger et OnAccent.

`OnAccent` utilise `Theme::on_accent`, la couleur du texte sur un fond accentué.
Les palettes existantes utilisent du blanc. Une palette personnalisée peut
modifier cette couleur indépendamment de l’accent :

```rust
let theme = Theme {
    accent: Color::rgb(240, 200, 40),
    on_accent: Color::rgb(20, 20, 20),
    ..Theme::DARK
};
```

La démo conserve deux recettes de panneau : opaque et avec alpha 238. Elles
partagent le rôle Surface, et suivent toutes deux le changement de palette.

## Boutons et états

```rust
let action = Arc::new(ButtonStyle {
    radius: 18.,
    padding_x: 14.,
    padding_y: 8.,
    min_height: 36.,
    font_size: 13.,
    normal: Some(ButtonColors::new(ThemeColor::Accent, ThemeColor::OnAccent)),
    hovered: Some(ButtonColors::new(ThemeColor::Hover, ThemeColor::Text)),
    pressed: Some(ButtonColors::new(ThemeColor::Pressed, ThemeColor::Text)),
    ..ButtonStyle::default()
});

let play = Element::custom(30,
    Button::new("Lire", PLAY).style(Arc::clone(&action)),
).width(Length::Auto);
```

`Button`, `ButtonStyle` et `ButtonColors` sont aussi exportés par `ui::widgets`.
Un état personnalisé contient son fond et son texte. Les états non renseignés
utilisent les couleurs du bouton et de son thème, indépendamment des autres
états personnalisés. La priorité est désactivé, pressé, survolé, puis normal.
L’application ne calcule pas ces états : le runtime les fournit à la peinture.

| Propriété | Rôle |
| --- | --- |
| `normal`, `hovered`, `pressed`, `disabled` | Fond et texte par état. |
| `focus` | Couleur de l’anneau de focus clavier. |
| `border`, `border_width` | Contour facultatif du bouton. |
| `radius` | Rayon des coins. |
| `padding_x`, `padding_y`, `min_height` | Taille naturelle et espace autour du texte. |
| `font_size`, `bold` | Mesure et dessin du libellé avec les mêmes paramètres. |

La mesure native du texte sert aussi au centrage vertical : augmenter la hauteur
minimale ne place pas le libellé au sommet. Le focus et la capture restent ceux
du bouton existant, avec le même identifiant et la même action.

Les variantes disponibles sont Primary, Secondary, Danger, Ghost et Outline.
Ghost possède un fond transparent au repos ; Outline ajoute un contour.
Une recette qui ne change que les dimensions conserve les couleurs de la
variante. Ses couleurs d’état explicites prennent priorité sur la variante,
quel que soit l’ordre de `.variant(...)` et `.style(...)`.

## Titres et textes

```rust
let heading = Arc::new(TextStyle {
    font_size: 24.,
    bold: true,
    color: Some(ThemeColor::Text.into()),
});
let title = Text::new("Bibliothèque").style(Arc::clone(&heading));
let small_title = Text::new("Albums").style(Arc::clone(&heading)).size(19.);
```

`.style(...)` applique taille, graisse et couleur. `.size`, `.bold` et `.color`
appelés ensuite restent des surcharges locales. Un nouvel appel à `.style`
réapplique sa recette. Les styles des autres textes ne sont pas modifiés.
Cette étape conserve la famille de police actuelle du renderer ; le choix de
familles, l’italique et l’héritage typographique restent à étendre.

## Fonctionnement et limites

Les recettes sont des structures Rust, partagées et immuables. Les méthodes
acceptent une valeur directement, ou un `Arc` déjà construit. Passer une valeur
crée une nouvelle référence possédée ; réutiliser un `Arc` évite de créer une
recette distincte pour chaque contrôle.

Les dimensions sont copiées à la construction de l’élément, et les couleurs de
thème sont résolues au dessin. Pour changer une recette à l’exécution, fournir
une nouvelle recette dans la prochaine vue et demander sa reconstruction.
Le survol et la pression demandent seulement une peinture : ils ne reconstruisent
pas les recettes et ne remesurent pas le layout.

Les recettes s’appliquent explicitement à chaque élément. Elles ne définissent
pas de cascade, de sélecteurs, d’héritage automatique ni d’animations. Les champs,
les choix, les sliders, les Select et les Tooltip conservent leurs API de style
actuelles ; la généralisation des références de thème à ces composants reste ouverte.

`Widget::validate` est un nouveau point de validation facultatif pour les widgets
personnalisés. Le layout l’appelle avant la mesure. Les dimensions de style sont
finies et positives ou nulles ; les tailles de police sont finies et strictement
positives. Une recette invalide est refusée avant de parvenir au renderer.

## Retrouver le code et vérifier

Les recettes sont dans `src/ui/core/style/`, leur application aux contrôles
dans `src/ui/components/` et la peinture des surfaces dans
`src/ui/runtime/painting/`. L’exemple `examples/simple_ui.rs` définit ses
recettes de mise en page avec `ViewStyle`.

```sh
cargo test --offline --lib style_tests
```
