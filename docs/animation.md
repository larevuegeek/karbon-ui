# Animations et transitions

```rust
use karbon_ui::ui::prelude::*;

column("card", [text("card.title", "Bonjour")])
    .padding(24)
    .animate(Animation::slide_up(24.).duration(350))
    .hover(Effect::new().translate(0., -4.).scale(1.02))
    .pressed(Effect::new().scale(0.98))
    .transition(Transition::new().duration(180).easing(Easing::EaseOut))
```

L’API anime l’opacité et les transformations d’un élément et de ses enfants.
Les distances sont en unités logiques, les angles en degrés, les durées et
les délais en millisecondes. L’horloge est monotone.

## Préréglages

| Animation | Effet |
| --- | --- |
| `fade_in()` | Opacité 0 → 1 |
| `fade_out()` | Opacité 1 → 0 |
| `slide_up(24.)` | Arrivée depuis le bas avec fondu |
| `slide_down(24.)` | Arrivée depuis le haut avec fondu |
| `slide_left(24.)` | Arrivée depuis la droite avec fondu |
| `slide_right(24.)` | Arrivée depuis la gauche avec fondu |
| `rotate(360.)` | Rotation de 0 à 360 degrés |
| `scale_in()` | Échelle 0,9 → 1 avec fondu |

Chaque préréglage appartient à `Animation` et accepte `.duration(ms)`,
`.delay(ms)`, `.advance(ms)`, `.easing(...)`, `.repeat(nombre)`, `.forever()`,
`.alternate()`, `.reverse()` et `.paused(bool)`.
`repeat(3)` signifie trois itérations au total ; `alternate()` inverse le
sens à chaque itération. Une durée nulle applique immédiatement l’état final.
Le délai précède la première itération. L’état initial est présenté pendant
le délai, et l’état final est conservé après la dernière itération.

```rust
Animation::rotate(360.).duration(2000).easing(Easing::Linear).forever()

Animation::from_to(
    Effect::new().opacity(0.).translate(0., 18.).rotate(-8.).scale(0.95),
    Effect::NONE,
).duration(500).easing(Easing::CubicBezier(0.16, 1., 0.3, 1.))
```

Les courbes sont `Linear`, `Ease`, `EaseIn`, `EaseOut`, `EaseInOut` et
`CubicBezier(x1, y1, x2, y2)`. Les X doivent être dans [0, 1] ; les Y peuvent
produire un dépassement. L’opacité interpolée reste entre 0 et 1.

## Identité, états et transitions

Une reconstruction de vue conserve l’animation d’un élément de même identité
et de même configuration. Pour la rejouer, modifier une clé :

```rust
card.animate(Animation::fade_in().key(self.replay_count))
```

Modifier la configuration la redémarre également. Retirer l’élément annule son
suivi ; le réinsérer crée une nouvelle entrée.

## Sortie, décalage et rejeu dans les templates

Dans un template HTML, l’entrée, les états et les transitions s’écrivent en CSS
(`animation`, `@keyframes`, `transition`, `:hover`…). Trois attributs `data-*`
couvrent ce que le CSS ne sait pas dire :

```html
<ul data-stagger="40ms">
  {#each self.tracks as track (track.id)}
    <li class="track" data-exit="slide-out-left 200ms ease-in"
        data-replay={self.replay}>{track.title}</li>
  {/each}
</ul>
```

- `data-exit="nom [durée] [easing] [délai]"` : quand l’élément disparaît entre
  deux reconstructions (bloc `{#if}` faux, ligne retirée d’un `{#each}`), une
  copie non interactive continue d’être peinte à sa dernière position, joue
  l’animation, puis est supprimée. Préréglages : `fade-out`, `slide-out-up`,
  `slide-out-down`, `slide-out-left`, `slide-out-right`, `scale-out` ; tout autre
  nom désigne un `@keyframes` de la feuille passée au template (`styles = css`).
  Durée par défaut 220 ms, easing `ease-in`. En Rust :
  `Element::exit_animation(Animation::fade_out().duration(200))`.
- `data-stagger="40ms"` : l’animation d’entrée de chaque enfant direct est
  retardée de son index × 40 ms.
- `data-replay={valeur}` : l’animation d’entrée redémarre quand la valeur change
  (elle est hachée en clé, comme `Animation::key`).

Les noms, durées et easings sont vérifiés à la compilation, avec une suggestion
en cas de faute de frappe. Le fantôme ne reçoit ni pointeur, ni focus, ni
accessibilité ; ses voisins reprennent immédiatement leur place (pas de
réservation d’espace pendant la sortie). Avec la réduction des animations, il
n’est pas créé.

`.opacity()`, `.translate()`, `.rotate()`, `.scale()` et `.effect()` définissent
l’état de base. Avec `.transition(...)`, un nouvel état est interpolé depuis
la valeur actuellement affichée. Une transition interrompue repart de sa
valeur intermédiaire.

`.hover(Effect)` couvre toute la surface de l’élément, son padding et ses enfants,
même sans bouton ni action. Ce survol ne rend pas une carte cliquable.
`.pressed(Effect)` prend la
priorité pendant l’appui, y compris au clavier. Ces effets sont relatifs à
la base : translations et angles s’ajoutent, opacités et échelles se
multiplient. L’animation d’entrée se combine de la même façon.

L’ordre est échelle, rotation, translation. Le pivot est le centre par défaut ;
`.transform_origin(x, y)` le règle avec deux proportions comprises entre 0 et 1.

## Périmètre de la première version

Les translations, rotations et échelles concernent textes, boutons, widgets
visuels personnalisés et conteneurs de ces éléments. Les clics suivent leur
position présentée et les rectangles accessibles sont actualisés après
préparation de la nouvelle pose.

Champs de saisie, sliders, selects, couches, sources/cibles de drag/drop,
ancres de tooltip et zones de scroll acceptent les **fondus**. Une transformation
géométrique sur ces contrôles, ou sur un conteneur qui les englobe, est refusée
à la validation : leurs coordonnées d’édition, popup, capture et composition
native nécessitent une intégration supplémentaire. Un texte ou un bouton
placé dans un scroll peut recevoir une transformation.

## Révélation de hauteur

```rust
column("details", [text("details.body", "Plus de détails")])
    .reveal(Transition::new().duration(350).easing(Easing::Ease))
```

`Element::reveal` (attribut de template `data-reveal="350ms ease"`, voir
[CSS](css.md#révélation-de-hauteur-auto--0)) anime la hauteur entre 0 et la
hauteur naturelle (`auto`) quand l’élément passe de `display: none` (règle de
style, `hidden`) à affiché, et retour. Contrairement aux effets, c’est une
animation de layout : la fraction de hauteur est échantillonnée pendant la
résolution des styles (piste de longueur, comme `transition: width`) et chaque
image relance un seul layout ; le contenu garde sa disposition naturelle et
est rogné à la boîte, les voisins suivent. À la fermeture l’élément reste
affiché jusqu’à la fin. Interrompue, la révélation repart de la hauteur
courante. Première apparition non animée ; mouvements réduits : état final
immédiat. Les volets `collapse` de Bootstrap l’utilisent (voir
[css-frameworks.md](css-frameworks.md)). Mesure (release, page `bootstrap_demo`
en 1280 × 900, un volet qui s’ouvre et un qui se ferme) : 21 layouts pour
21 images, ≈ 2,9 ms par image (styles, layout et scène) contre ≈ 2,0 ms sans
animation de hauteur. Ce relevé venait d’un test de mesure ignoré
(`frame_cost_of_an_accordion_animation`), retiré de la suite avec les autres mesures de
temps ; il reste dans l’historique git (commit `a37ea7e`).

L’opacité est visuelle : elle ne change ni l’espace occupé ni le focus.
`fade_out` n’efface pas l’élément. Les transitions CSS de couleur et de
dimension (`width`, `height` en pixels) passent par `Transition::properties` et
`Transition::sizes` : elles sont échantillonnées pendant la résolution des styles
et, pour les dimensions, relancent le layout à chaque image (voir
[styles CSS compilés](css.md)). Les couleurs s'interpolent en alpha prémultiplié,
comme dans les navigateurs : de `transparent` vers le blanc, seule l'opacité change
(pas de gris intermédiaire). Les animations `@keyframes` de taille et les callbacks
de fin restent à fournir.

Les étapes multiples sont disponibles avec `Animation::keyframes` et avec
`@keyframes` dans les [styles CSS compilés](css.md). Une étape déclare tout ou
partie des propriétés : `Keyframe::new(offset, effect)` déclare l'opacité et la
transformation, `Keyframe::at(offset)` aucune, complétée par `.opacity()`,
`.transform()`, `.color()`, `.background_color()`, `.border_color()` et
`.easing()`. Chaque propriété s'interpole entre ses étapes voisines qui la
déclarent, avec la courbe de la première ; absente de 0 ou de 1, elle y prend la
valeur de l'élément (l'effet neutre, ou la couleur calculée). Deux étapes peuvent
partager un offset si elles déclarent des propriétés différentes.

```rust
const POP: &[Keyframe] = &[
    Keyframe::at(0.).opacity(0.),
    Keyframe::at(0.4).transform(Effect::new().scale(1.1)).easing(Easing::EaseOut),
    Keyframe::at(1.).opacity(1.),
];
const BOTH: &[Animation] = &[
    Animation::keyframes(POP).duration(400),
    Animation::rotate(8.).duration(600).repeat(2).alternate().reverse(),
];
card.animate(Animation::group(BOTH))
```

`Animation::group` joue plusieurs animations ensemble (liste CSS `animation: a, b`) :
chacune garde son minutage, la dernière gagne sur une propriété commune, le délai du
groupe (et `data-stagger`) les décale toutes. `.reverse()` joue les itérations à
rebours (`alternate-reverse` avec `.alternate()`), `.advance(ms)` démarre déjà
avancé (délai CSS négatif), `.paused(true)` fige l'horloge : sur un élément d'identité
stable, ne changer que ce drapeau met en pause ou reprend sans redémarrer.
`FillMode` contrôle les poses avant/après l’effet. Les couleurs animées sont
échantillonnées pendant la résolution des styles (qui suit alors chaque image) ;
opacité et transformation restent appliquées à la peinture. Le CSS est traduit à la
compilation ; la timeline ne parse pas de CSS.

## Horloge et coût

Le moteur conserve arbre et géométrie et repeint pendant l’effet, sans
reconstruire la vue ni refaire le layout à chaque image. Il demande des réveils
ponctuels à environ 60 Hz pendant les animations visibles, attend un délai sans
images intermédiaires, puis cesse les réveils. Les éléments retirés sont
libérés ; ceux hors du viewport de layout ne demandent pas d’images.
Le timer de fenêtre s’arrête si elle est masquée ou minimisée ; la reprise
échantillonne le temps écoulé.

Direct2D applique matrices et calques en GPU ou en WARP. Le fondu de groupe
compose correctement les enfants qui se recouvrent. Un calque n’est utilisé
que pour une opacité inférieure à 1. La présentation Windows conserve sa
relecture CPU et son image de fenêtre complète : aucune conclusion de fluidité
ou de gain face à une UI web n’est tirée de cette étape.

`WindowOptions::reduced_motion`, `--reduced-motion` et
`UiRuntime::set_reduced_motion(true)` appliquent les états finaux sans durée ni
délai. La préférence système n’est pas encore interrogée automatiquement.
`UiRuntime::prepare_at(..., instant)` et `UiEvent::Timer(instant)` permettent
des tests déterministes sans attendre réellement les durées.

## Exemples et tests

L’ancien atelier `animation_playground` a été retiré. Les animations se voient dans
`rustmusic_html` (largeur de la barre latérale, menus avec `data-exit`, survols) et
`bootstrap_demo` (accordéon, onglets, spinners, offcanvas). Les comportements sont
fixés par les tests : progression, répétition, interruptions, annulation, réveils,
clics, géométrie accessible et mouvements réduits dans `src/ui/tests/animation/` (pistes
par propriété, directions, délai négatif, pause et listes dans `keyframes.rs`) ;
keyframes et transitions CSS dans `tests/css/motion.rs` ; animations `data-*` des
templates dans `tests/templates/html.rs` ; transitions Bootstrap dans
`tests/frameworks/bootstrap/demo/transitions.rs`. Les pixels (opacité imbriquée,
clipping, DPI) sont vérifiés sur WARP et GPU par `src/platform/windows/graphics/tests/`.
