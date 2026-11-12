# Se repérer dans le moteur UI

```text
ui/
├── mod.rs          Modules publics et compatibilité des anciens imports
├── prelude.rs      Imports usuels pour écrire une application
├── animation/      Effets, préréglages, courbes et suivi temporel
├── core/           Éléments, contrat Widget, géométrie, unités et styles
├── components/     Composants et dossiers par contrôle complexe
├── overlays/       Modales, popovers, menus et tooltips
├── layout/         Mesure et stratégies de placement
├── rendering/      Scène, commandes, images, polices, vecteurs et ombre
├── runtime/        État de l’interface, événements, focus et coordination
├── view/           Syntaxe de composition et ViewStyle
├── template/       Contexte, props, slots et constructeurs typés des templates
├── styling/        Apparence commune, héritage, états et variables typées
└── tests/          Scénarios par fonctionnalité et fixtures communes
```

## Composants

Les petits widgets sont dans `components/text.rs` et `button.rs`. Les dossiers
`choices/`, `text_input/`, `select/` et `slider/` séparent les responsabilités.

| Je veux modifier… | Ouvrir… |
| --- | --- |
| Checkbox, Switch, RadioButton | `components/choices/checkbox.rs`, `switch.rs`, `radio.rs` |
| Leur dessin partagé | `components/choices/drawing.rs` |
| La configuration du champ et son fond | `components/text_input/widget.rs` |
| Le texte, la sélection et le caret | `components/text_input/drawing.rs` |
| Les frappes, le presse-papiers et la composition | `components/text_input/events.rs` |
| L’édition et l’historique | `components/text_input/editor.rs` |
| Les clusters et positions du texte | `components/text_input/geometry.rs` |
| Le popup du Select | `components/select/popup.rs` et `drawing.rs` |
| Sa navigation clavier | `components/select/navigation.rs` |
| Le tracé du Slider | `components/slider/drawing.rs` |

Dans un dossier de composant, `mod.rs` rassemble les types partagés et les
exports, `widget.rs` porte l’API publique, `style.rs` les paramètres visuels,
`drawing.rs` la peinture, `state.rs` l’état temporaire et `events.rs` les
interactions. Seuls les fichiers utiles au composant sont présents.

## Moteur

- `template/` : contrat Component et Slots dans `slots.rs`, contexte et
  conversions dans `context.rs`, construction typée dans `node/` (modèle dans
  `mod.rs`, puis une famille de propriétés par fichier : `cascade.rs`,
  `sizing.rs`, `container.rs`, `surface.rs`, `motion.rs`, `text.rs`).
  Le compilateur est séparé dans `crates/ui-macros/` (sa carte :
  [compiler.md](../../docs/compiler.md) ; arguments et fichiers dans `input/`,
  templates dans `template/`, CSS dans `css/`). Voir
  [templates](../../docs/templates.md) et [couverture CSS](../../docs/css.md).
- `animation/` : `effect.rs` définit les effets, `spec.rs` les préréglages et
  durées, `easing.rs` les courbes, `timeline/` le suivi et l’horloge (pistes
  d’effets dans `track.rs`, transitions de valeurs dans `value.rs`, fantômes
  de sortie dans `ghost.rs`). Les matrices sont dans `rendering/transform.rs`.
- `styling/` : propriétés et états dans `properties/` (typographie, fonds,
  apparence, règles, sélecteurs, valeurs calculées), héritage et résolution
  dans `resolve/` (environnement, correspondance des sélecteurs, cascade,
  application par nœud), déclarations CSS dynamiques dans `tokens/` (jetons,
  substitution des variables, lecteur de valeurs, couleurs, ombres, transitions,
  et `apply/` : une famille de propriétés par fichier), couleurs dans `color/`,
  métriques dans `text.rs`, variables dans `variables.rs`.
- `core/element/` : clés dans `id.rs`, dimensions et layout dans `types.rs`,
  conversions dans `units.rs`, contrat dans `widget.rs`, construction dans
  `builders.rs` et arbre dans `mod.rs`.
- `core/style/` : recettes séparées dans `color.rs`, `surface.rs`, `text.rs`
  et `button.rs`.
- `layout/` : entrées dans `validation.rs`, `measure.rs` et `arrange.rs`.
  Stratégies dans `linear.rs`, `grid.rs`, `split.rs`, `layers.rs`,
  `scrolling.rs` ; métriques dans `scroll.rs` ; lignes utiles dans `virtual_list.rs`.
  `flex/` (lignes et longueurs flexibles dans `lines.rs`, placement dans
  `mod.rs`), `tracks.rs`, `position.rs` et `expression.rs` portent les extensions CSS.
- `rendering/scene/` : vocabulaire dans `commands.rs`, images dans `image.rs`,
  buffer de scène dans `mod.rs`, couverture (test de contenu des fenêtres
  transparentes) dans `coverage.rs`. Ombres de composants dans
  `rendering/component_shadow/` (cache, `raster.rs`, `vector.rs`).
- `runtime/` : état découpé entre `interaction.rs` (pointeur, focus, tabulation),
  `components.rs` (états transitoires des composants et crochets `ComponentState`)
  et `invalidation.rs` (passes à refaire, caches, compteurs) ;
  préparation dans `prepare.rs`, routage/souris/clavier/molette
  dans `events/`, focus dans `focus.rs`, navigation dans `scrolling.rs` et
  `virtualization.rs`, contrat/commandes/snapshot dans `accessibility/`,
  glisser-déposer dans `drag_drop/`, peinture dans `painting/` (parcours de
  l’arbre, fonds, bordures, barres de défilement, chrome de fenêtre).
- `overlays/` : modales, placement des couches, items dans `popover/menu.rs`,
  configuration/délai/dessin dans `tooltip/`.
- `tests/` : tests de contrat du moteur, par thème ; voir
  [l’organisation des tests](../../docs/tests.md).

## Imports

Pour composer une page : `use karbon_ui::ui::prelude::*;`.

Les modules sont accessibles par responsabilité, par exemple
`ui::components::text_input`, `ui::core::element` et `ui::rendering::scene`.
Les anciens chemins (`ui::text_input`, `ui::widgets`, `ui::scene`, etc.) sont
réexportés dans `ui/mod.rs` et pointent vers les mêmes implémentations.

Le backend Windows reste dans `src/platform/windows/`. Pour écrire une page,
lire le [guide de syntaxe](../../docs/syntax.md).
